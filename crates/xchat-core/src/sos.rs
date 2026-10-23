use serde::{Deserialize, Serialize};
use xchat_crypto::{
    aead::{decrypt_aead, derive_nonce, encrypt_aead},
    identity::{IdentityKeyPair, IdentityPublicKey},
};

use crate::{
    envelope::{EncryptedEnvelope, RoutingHeader},
    errors::CoreError,
};

/// Public emergency destination token for SOS mesh packets.
pub const EMERGENCY_BROADCAST_TOKEN: [u8; 16] = [0xff; 16];
/// Dedicated symmetric emergency AEAD key for community search-and-rescue decoding.
pub const EMERGENCY_AEAD_KEY: [u8; 32] = [
    0xee, 0x50, 0x50, 0xee, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
    0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
];

/// Signed emergency distress beacon broadcasted over local radio mesh.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SosBeacon {
    pub sender_address: String,
    pub sender_pubkey: IdentityPublicKey,
    pub timestamp: u64,
    pub latitude: f64,
    pub longitude: f64,
    pub emergency_message: String,
}

impl SosBeacon {
    /// Format human-readable emergency text.
    pub fn display_summary(&self) -> String {
        format!(
            "🚨 SOS DISTRESS BEACON from {}\nLocation: {:.5}, {:.5}\nTime: {}\nMessage: \"{}\"",
            self.sender_address,
            self.latitude,
            self.longitude,
            self.timestamp,
            self.emergency_message
        )
    }
}

/// Create and sign a high-priority emergency distress beacon.
pub fn create_emergency_beacon(
    identity: &IdentityKeyPair,
    latitude: f64,
    longitude: f64,
    message: &str,
    now: u64,
    ttl_seconds: u64,
) -> Result<EncryptedEnvelope, CoreError> {
    let beacon = SosBeacon {
        sender_address: identity.public_key().address(),
        sender_pubkey: identity.public_key(),
        timestamp: now,
        latitude,
        longitude,
        emergency_message: message.to_string(),
    };

    let serialized =
        bincode::serialize(&beacon).map_err(|e| CoreError::Serialization(e.to_string()))?;
    let header = RoutingHeader::new(EMERGENCY_BROADCAST_TOKEN, now, ttl_seconds);
    let aad = header.to_aad();

    let nonce = derive_nonce(now, true);
    let ciphertext =
        encrypt_aead(&EMERGENCY_AEAD_KEY, &nonce, &serialized, &aad).map_err(CoreError::Crypto)?;

    Ok(EncryptedEnvelope {
        header,
        ratchet_header: None,
        x3dh_init: None,
        ciphertext,
    })
}

/// Decode and verify an inbound emergency distress beacon.
pub fn decode_emergency_beacon(
    envelope: &EncryptedEnvelope,
    now: u64,
) -> Result<SosBeacon, CoreError> {
    if envelope.header.destination_token != EMERGENCY_BROADCAST_TOKEN {
        return Err(CoreError::EnvelopeError(
            "Not an emergency beacon packet".into(),
        ));
    }

    if envelope.is_expired(now) {
        return Err(CoreError::ExpiredEnvelope {
            expires_at: envelope.header.expires_at,
            now,
        });
    }

    let aad = envelope.header.to_aad();
    let nonce = derive_nonce(envelope.header.created_at, true);
    let decrypted = decrypt_aead(&EMERGENCY_AEAD_KEY, &nonce, &envelope.ciphertext, &aad)
        .map_err(CoreError::Crypto)?;

    bincode::deserialize(&decrypted).map_err(|e| CoreError::Serialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_sos_beacon_roundtrip() {
        let identity = IdentityKeyPair::generate();
        let now = 1000;
        let env = create_emergency_beacon(
            &identity,
            37.7749,
            -122.4194,
            "Earthquake survivor, need medical aid!",
            now,
            7200,
        )
        .expect("beacon created");

        assert_eq!(env.header.destination_token, EMERGENCY_BROADCAST_TOKEN);

        let decoded = decode_emergency_beacon(&env, now + 10).expect("beacon decoded");
        assert_eq!(decoded.sender_address, identity.public_key().address());
        assert_eq!(decoded.latitude, 37.7749);
        assert_eq!(
            decoded.emergency_message,
            "Earthquake survivor, need medical aid!"
        );
    }
}
