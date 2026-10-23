//! Telephony SMS Transport Carrier
//!
//! Enables transmission of opaque end-to-end encrypted xChat envelopes over
//! standard cellular SMS messages when mobile data, Wi-Fi, and Bluetooth are unavailable.
//!
//! Envelopes are serialized, padded, and segmented into 140-character GSM-safe
//! hex chunks: `XCHAT:v1:<msg_id>:<seq>/<total>:<hex_payload>`.
//! Receiving nodes buffer and reassemble incoming SMS parts out-of-order and reconstruct
//! the authenticated `EncryptedEnvelope` with zero exposure of plaintext or identities to telcos.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use xchat_core::envelope::EncryptedEnvelope;

use crate::errors::TransportError;
use crate::traits::{Transport, TransportType};

/// Maximum hex characters in a single SMS carrier frame payload (keeps total SMS <= 160 chars).
pub const DEFAULT_SMS_CHUNK_SIZE: usize = 100;

/// Pending multipart SMS reassembly session.
struct PendingSmsReassembly {
    total: usize,
    chunks: HashMap<usize, Vec<u8>>,
    created_at: Instant,
}

/// Standalone SMS Carrier engine for chunking and reconstructing envelopes.
#[derive(Clone)]
pub struct SmsCarrier {
    pending: Arc<Mutex<HashMap<String, PendingSmsReassembly>>>,
    inbound_queue: Arc<Mutex<Vec<EncryptedEnvelope>>>,
    outbound_sms_log: Arc<Mutex<Vec<(String, String)>>>, // (target_phone, sms_text)
    max_chunk_size: usize,
    timeout: Duration,
}

impl SmsCarrier {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            inbound_queue: Arc::new(Mutex::new(Vec::new())),
            outbound_sms_log: Arc::new(Mutex::new(Vec::new())),
            max_chunk_size: DEFAULT_SMS_CHUNK_SIZE,
            timeout: Duration::from_secs(300), // 5 minute reassembly timeout
        }
    }

    /// Set chunk size for SMS fragmentation.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.max_chunk_size = size;
        self
    }

    /// Encode an encrypted envelope into one or more SMS carrier text strings.
    pub fn encode_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<Vec<String>, TransportError> {
        let binary_bytes = bincode::serialize(envelope)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        let hex_payload = hex::encode(&binary_bytes);

        // Derive short 8-char transaction id from message_id for compact header
        let msg_id_short = hex::encode(&envelope.header.message_id[0..4]);

        let total_chunks = hex_payload.len().div_ceil(self.max_chunk_size);
        let mut frames = Vec::with_capacity(total_chunks);

        for (idx, chunk) in hex_payload
            .as_bytes()
            .chunks(self.max_chunk_size)
            .enumerate()
        {
            let chunk_str = std::str::from_utf8(chunk)
                .map_err(|e| TransportError::Serialization(e.to_string()))?;
            // Format: XCHAT:v1:<msg_id_short>:<seq>/<total>:<hex_data>
            let frame = format!(
                "XCHAT:v1:{}:{}/{}:{}",
                msg_id_short, idx, total_chunks, chunk_str
            );
            frames.push(frame);
        }

        Ok(frames)
    }

    /// Ingest a raw incoming SMS text message.
    /// Returns `Ok(Some(envelope))` if this SMS completes an envelope, or `Ok(None)` if still waiting for parts.
    pub async fn ingest_sms_text(
        &self,
        sms_text: &str,
    ) -> Result<Option<EncryptedEnvelope>, TransportError> {
        let trimmed = sms_text.trim();
        if !trimmed.starts_with("XCHAT:v1:") {
            // Non-xChat SMS ignored
            return Ok(None);
        }

        let parts: Vec<&str> = trimmed.splitn(5, ':').collect();
        if parts.len() < 5 {
            return Err(TransportError::FrameTooShort);
        }

        let msg_id_short = parts[2].to_string();
        let seq_info = parts[3];
        let hex_data = parts[4];

        let seq_parts: Vec<&str> = seq_info.split('/').collect();
        if seq_parts.len() != 2 {
            return Err(TransportError::InvalidFrameHeader);
        }

        let seq: usize = seq_parts[0]
            .parse()
            .map_err(|_| TransportError::InvalidFrameHeader)?;
        let total: usize = seq_parts[1]
            .parse()
            .map_err(|_| TransportError::InvalidFrameHeader)?;

        let raw_chunk =
            hex::decode(hex_data).map_err(|e| TransportError::Deserialization(e.to_string()))?;

        let mut pending = self.pending.lock().await;

        // Prune expired entries
        let now = Instant::now();
        let timeout = self.timeout;
        pending.retain(|_, v| now.duration_since(v.created_at) < timeout);

        let entry = pending
            .entry(msg_id_short.clone())
            .or_insert_with(|| PendingSmsReassembly {
                total,
                chunks: HashMap::new(),
                created_at: Instant::now(),
            });

        entry.chunks.insert(seq, raw_chunk);

        // Check if all chunks received
        if entry.chunks.len() == entry.total {
            let mut full_bytes = Vec::new();
            for i in 0..entry.total {
                if let Some(c) = entry.chunks.get(&i) {
                    full_bytes.extend_from_slice(c);
                } else {
                    return Err(TransportError::CorruptPayload);
                }
            }
            pending.remove(&msg_id_short);

            let envelope: EncryptedEnvelope = bincode::deserialize(&full_bytes)
                .map_err(|e| TransportError::Deserialization(e.to_string()))?;

            // Push to internal inbound queue
            let mut q = self.inbound_queue.lock().await;
            q.push(envelope.clone());

            return Ok(Some(envelope));
        }

        Ok(None)
    }

    /// Read all logged outbound SMS strings (for testing and integration).
    pub async fn get_sent_sms_log(&self) -> Vec<(String, String)> {
        self.outbound_sms_log.lock().await.clone()
    }
}

impl Default for SmsCarrier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for SmsCarrier {
    fn transport_type(&self) -> TransportType {
        TransportType::SmsCarrier
    }

    async fn send_envelope(
        &self,
        target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        let sms_list = self.encode_envelope(envelope)?;
        let mut log = self.outbound_sms_log.lock().await;
        for sms in sms_list {
            log.push((target_address.to_string(), sms));
        }
        Ok(())
    }

    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError> {
        let sms_list = self.encode_envelope(envelope)?;
        let count = sms_list.len();
        let mut log = self.outbound_sms_log.lock().await;
        for sms in sms_list {
            log.push(("BROADCAST".to_string(), sms));
        }
        Ok(count)
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

    fn make_sample_envelope(content_size: usize) -> EncryptedEnvelope {
        let header = RoutingHeader {
            version: 1,
            message_id: [0x77; 16],
            destination_token: [0x55; 16],
            created_at: 1000,
            expires_at: 2000,
            hop_count: 0,
        };
        EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![0xcc; content_size],
        }
    }

    #[tokio::test]
    async fn test_sms_carrier_fragmentation_and_reassembly() {
        let carrier_send = SmsCarrier::new().with_chunk_size(60);
        let carrier_recv = SmsCarrier::new();

        let original = make_sample_envelope(180);

        // Encode to SMS frames
        let frames = carrier_send
            .encode_envelope(&original)
            .expect("Encoding should succeed");
        assert!(frames.len() > 1, "Should split into multiple SMS frames");

        // Verify frame prefix
        for frame in &frames {
            assert!(frame.starts_with("XCHAT:v1:"));
        }

        // Deliver out of order to receiver
        let mut reconstructed = None;
        for frame in frames.iter().rev() {
            if let Some(env) = carrier_recv
                .ingest_sms_text(frame)
                .await
                .expect("Ingest ok")
            {
                reconstructed = Some(env);
            }
        }

        let result = reconstructed.expect("Should have reassembled full envelope");
        assert_eq!(result.header.message_id, original.header.message_id);
        assert_eq!(result.ciphertext, original.ciphertext);
        assert_eq!(
            result.header.destination_token,
            original.header.destination_token
        );
    }
}
