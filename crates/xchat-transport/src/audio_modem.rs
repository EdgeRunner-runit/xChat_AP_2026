//! Acoustic & Voice Call Audio Modem Transport
//!
//! Converts opaque encrypted envelopes into audio frequency tones (Audio Frequency Shift Keying - AFSK)
//! suitable for transmission over cellular voice calls, landlines, Walkie-Talkies,
//! or speaker-to-microphone physical airgaps.
//!
//! Standard voice band frequencies:
//! - Mark tone (Binary 1): 1200 Hz
//! - Space tone (Binary 0): 2200 Hz
//! - Sync Preamble tone: 1700 Hz
//! - Standard Telephone Sample Rate: 8000 Hz

use async_trait::async_trait;
use std::f32::consts::PI;
use std::sync::Arc;
use tokio::sync::Mutex;
use xchat_core::envelope::EncryptedEnvelope;

use crate::errors::TransportError;
use crate::traits::{Transport, TransportType};

pub const DEFAULT_SAMPLE_RATE: u32 = 8000;
pub const MARK_FREQ: f32 = 1200.0;
pub const SPACE_FREQ: f32 = 2200.0;
pub const SYNC_FREQ: f32 = 1700.0;
pub const SAMPLES_PER_BIT: usize = 16; // 8000 Hz / 16 = 500 baud

/// Acoustic Call Modem for transmitting encrypted packets over telephone voice channels.
#[derive(Clone)]
pub struct AudioCallModem {
    sample_rate: u32,
    samples_per_bit: usize,
    inbound_queue: Arc<Mutex<Vec<EncryptedEnvelope>>>,
    modulated_audio_log: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl AudioCallModem {
    pub fn new() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            samples_per_bit: SAMPLES_PER_BIT,
            inbound_queue: Arc::new(Mutex::new(Vec::new())),
            modulated_audio_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Calculate standard CRC-16-CCITT checksum for data integrity verification over noisy audio lines.
    pub fn compute_crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= (byte as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    /// Modulate arbitrary binary payload into floating-point PCM audio samples (-1.0 to 1.0).
    pub fn modulate_bytes(&self, data: &[u8]) -> Vec<f32> {
        let mut samples = Vec::new();

        // 1. Preamble synchronization tone (32 bit periods of SYNC_FREQ)
        let sync_samples = self.samples_per_bit * 32;
        for i in 0..sync_samples {
            let t = i as f32 / self.sample_rate as f32;
            let sample = (2.0 * PI * SYNC_FREQ * t).sin();
            samples.push(sample * 0.8);
        }

        // 2. Frame Assembly: [Length: 2 bytes] [Data] [CRC-16: 2 bytes]
        let len = data.len() as u16;
        let crc = Self::compute_crc16(data);

        let mut frame_bytes = Vec::with_capacity(data.len() + 4);
        frame_bytes.extend_from_slice(&len.to_be_bytes());
        frame_bytes.extend_from_slice(data);
        frame_bytes.extend_from_slice(&crc.to_be_bytes());

        // 3. Modulate bits to Mark/Space tones
        let mut sample_index = sync_samples;
        for byte in frame_bytes {
            for bit_idx in (0..8).rev() {
                let bit = (byte >> bit_idx) & 1;
                let freq = if bit == 1 { MARK_FREQ } else { SPACE_FREQ };

                for _ in 0..self.samples_per_bit {
                    let t = sample_index as f32 / self.sample_rate as f32;
                    let sample = (2.0 * PI * freq * t).sin();
                    samples.push(sample * 0.8);
                    sample_index += 1;
                }
            }
        }

        samples
    }

    /// Demodulate floating-point PCM audio samples back into original binary bytes.
    pub fn demodulate_audio(&self, samples: &[f32]) -> Result<Vec<u8>, TransportError> {
        if samples.len() < self.samples_per_bit * 36 {
            return Err(TransportError::FrameTooShort);
        }

        // Detect bitstream by testing dominant energy at Mark vs Space frequency per bit window
        let mut bits = Vec::new();

        // Skip preamble
        let start_offset = self.samples_per_bit * 32;
        let mut i = start_offset;

        while i + self.samples_per_bit <= samples.len() {
            let chunk = &samples[i..i + self.samples_per_bit];

            // Compute energy correlation with Mark (1200Hz) and Space (2200Hz)
            let mut mark_energy_i = 0.0f32;
            let mut mark_energy_q = 0.0f32;
            let mut space_energy_i = 0.0f32;
            let mut space_energy_q = 0.0f32;

            for (j, &s) in chunk.iter().enumerate() {
                let t = (i + j) as f32 / self.sample_rate as f32;
                mark_energy_i += s * (2.0 * PI * MARK_FREQ * t).cos();
                mark_energy_q += s * (2.0 * PI * MARK_FREQ * t).sin();
                space_energy_i += s * (2.0 * PI * SPACE_FREQ * t).cos();
                space_energy_q += s * (2.0 * PI * SPACE_FREQ * t).sin();
            }

            let mark_pwr = mark_energy_i * mark_energy_i + mark_energy_q * mark_energy_q;
            let space_pwr = space_energy_i * space_energy_i + space_energy_q * space_energy_q;

            let bit = if mark_pwr >= space_pwr { 1u8 } else { 0u8 };
            bits.push(bit);

            i += self.samples_per_bit;
        }

        // Reconstruct bytes from bits
        let mut bytes = Vec::new();
        for chunk in bits.chunks_exact(8) {
            let mut byte = 0u8;
            for &b in chunk {
                byte = (byte << 1) | b;
            }
            bytes.push(byte);
        }

        if bytes.len() < 4 {
            return Err(TransportError::FrameTooShort);
        }

        let payload_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        if bytes.len() < 4 + payload_len {
            return Err(TransportError::CorruptPayload);
        }

        let payload = &bytes[2..2 + payload_len];
        let expected_crc = u16::from_be_bytes([bytes[2 + payload_len], bytes[3 + payload_len]]);
        let actual_crc = Self::compute_crc16(payload);

        if actual_crc != expected_crc {
            return Err(TransportError::CorruptPayload);
        }

        Ok(payload.to_vec())
    }

    /// Modulate an EncryptedEnvelope into an audio waveform.
    pub fn encode_envelope_audio(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<Vec<f32>, TransportError> {
        let bytes = bincode::serialize(envelope)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        Ok(self.modulate_bytes(&bytes))
    }

    /// Demodulate an audio waveform into an EncryptedEnvelope.
    pub fn decode_envelope_audio(
        &self,
        samples: &[f32],
    ) -> Result<EncryptedEnvelope, TransportError> {
        let bytes = self.demodulate_audio(samples)?;
        let env = bincode::deserialize(&bytes)
            .map_err(|e| TransportError::Deserialization(e.to_string()))?;
        Ok(env)
    }

    /// Inspect logged audio samples.
    pub async fn get_audio_log(&self) -> Vec<Vec<f32>> {
        self.modulated_audio_log.lock().await.clone()
    }
}

impl Default for AudioCallModem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for AudioCallModem {
    fn transport_type(&self) -> TransportType {
        TransportType::AudioAcousticModem
    }

    async fn send_envelope(
        &self,
        _target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        let audio = self.encode_envelope_audio(envelope)?;
        self.modulated_audio_log.lock().await.push(audio);
        Ok(())
    }

    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError> {
        let audio = self.encode_envelope_audio(envelope)?;
        self.modulated_audio_log.lock().await.push(audio);
        Ok(1)
    }

    async fn receive_envelope(&self) -> Result<EncryptedEnvelope, TransportError> {
        let mut q = self.inbound_queue.lock().await;
        if let Some(env) = q.pop() {
            Ok(env)
        } else {
            Err(TransportError::ConnectionClosed)
        }
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xchat_core::envelope::RoutingHeader;

    #[test]
    fn test_audio_modem_modulation_and_demodulation() {
        let modem = AudioCallModem::new();
        let test_data = b"xChat-Secure-Voice-Audio-Modem-Packet-2026";

        // Modulate bytes to audio samples
        let samples = modem.modulate_bytes(test_data);
        assert!(!samples.is_empty());

        // Demodulate audio samples back to bytes
        let decoded = modem
            .demodulate_audio(&samples)
            .expect("Demodulation should succeed");
        assert_eq!(decoded, test_data);
    }

    #[test]
    fn test_audio_modem_envelope_roundtrip() {
        let modem = AudioCallModem::new();
        let header = RoutingHeader {
            version: 1,
            message_id: [0x77; 16],
            destination_token: [0x42; 16],
            created_at: 1000,
            expires_at: 2000,
            hop_count: 0,
        };
        let original = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![0x55; 32],
        };

        let samples = modem
            .encode_envelope_audio(&original)
            .expect("Audio encode ok");
        let decoded = modem
            .decode_envelope_audio(&samples)
            .expect("Audio decode ok");

        assert_eq!(decoded.header.message_id, original.header.message_id);
        assert_eq!(
            decoded.header.destination_token,
            original.header.destination_token
        );
        assert_eq!(decoded.ciphertext, original.ciphertext);
    }
}
