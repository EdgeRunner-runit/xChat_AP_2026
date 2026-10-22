use rand::Rng;
use serde::{Deserialize, Serialize};
use xchat_crypto::{double_ratchet::RatchetHeader, x3dh::X3dhInitMessage};

use crate::errors::CoreError;

pub const CURRENT_PROTOCOL_VERSION: u8 = 1;
pub const DEFAULT_TTL_SECONDS: u64 = 86400; // 24 hours

pub type MessageId = [u8; 16];
pub type RouteToken = [u8; 16];

/// Generate a cryptographically random 128-bit Message ID.
pub fn generate_message_id() -> MessageId {
    let mut rng = rand::thread_rng();
    let mut id = [0u8; 16];
    rng.fill(&mut id);
    id
}

/// Relay-visible routing header.
/// CRITICAL: Contains zero sender/recipient usernames, phone numbers, or plaintexts.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RoutingHeader {
    pub version: u8,
    pub message_id: MessageId,
    pub destination_token: RouteToken,
    pub created_at: u64,
    pub expires_at: u64,
    pub hop_count: u8,
}

impl RoutingHeader {
    pub fn new(destination_token: RouteToken, created_at: u64, ttl_seconds: u64) -> Self {
        Self {
            version: CURRENT_PROTOCOL_VERSION,
            message_id: generate_message_id(),
            destination_token,
            created_at,
            expires_at: created_at.saturating_add(ttl_seconds),
            hop_count: 0,
        }
    }

    /// Serialize immutable routing context to bytes for use as Authenticated Associated Data (AAD).
    /// Bound to AEAD encryption so routing destination, message ID, version, and TTL cannot be tampered with,
    /// while hop_count remains a mutable network hop counter for multi-hop relays.
    pub fn to_aad(&self) -> Vec<u8> {
        let mut aad = Vec::with_capacity(49);
        aad.push(self.version);
        aad.extend_from_slice(&self.message_id);
        aad.extend_from_slice(&self.destination_token);
        aad.extend_from_slice(&self.created_at.to_be_bytes());
        aad.extend_from_slice(&self.expires_at.to_be_bytes());
        aad
    }

    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expires_at
    }
}

/// Opaque wire envelope traversing heterogeneous transports.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    pub header: RoutingHeader,
    pub ratchet_header: Option<RatchetHeader>,
    pub x3dh_init: Option<X3dhInitMessage>,
    pub ciphertext: Vec<u8>,
}

impl EncryptedEnvelope {
    pub fn to_bytes(&self) -> Result<Vec<u8>, CoreError> {
        bincode::serialize(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        bincode::deserialize(bytes).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    pub fn is_expired(&self, current_time: u64) -> bool {
        self.header.is_expired(current_time)
    }

    pub fn increment_hop(&mut self) {
        self.header.hop_count = self.header.hop_count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_serialization_roundtrip() {
        let header = RoutingHeader::new([0xab; 16], 1000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![1, 2, 3, 4, 5],
        };

        let bytes = env.to_bytes().expect("serialize succeeds");
        let restored = EncryptedEnvelope::from_bytes(&bytes).expect("deserialize succeeds");
        assert_eq!(env, restored);
    }

    #[test]
    fn test_envelope_ttl() {
        let header = RoutingHeader::new([0xab; 16], 1000, 3600);
        assert!(!header.is_expired(2000));
        assert!(header.is_expired(4600));
        assert!(header.is_expired(4601));
    }
}
