use std::collections::HashMap;

use crate::{
    envelope::{EncryptedEnvelope, RouteToken},
    errors::CoreError,
};

pub const MAX_RELAY_STORE_SIZE: usize = 5000;

/// Relay node facilitating store-and-forward routing between asynchronous/offline peers.
/// Completely BLIND to message plaintext, sender identity, and conversation graph.
#[derive(Debug, Default)]
pub struct RelayNode {
    storage: HashMap<RouteToken, Vec<EncryptedEnvelope>>,
    total_stored: usize,
}

impl RelayNode {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            total_stored: 0,
        }
    }

    /// Accept and buffer an opaque encrypted envelope for a destination routing token.
    pub fn forward_envelope(
        &mut self,
        mut envelope: EncryptedEnvelope,
        current_time: u64,
    ) -> Result<(), CoreError> {
        if envelope.is_expired(current_time) {
            return Err(CoreError::ExpiredEnvelope {
                expires_at: envelope.header.expires_at,
                now: current_time,
            });
        }

        if self.total_stored >= MAX_RELAY_STORE_SIZE {
            self.prune_expired(current_time);
            if self.total_stored >= MAX_RELAY_STORE_SIZE {
                return Err(CoreError::RoutingError("Relay capacity exceeded".into()));
            }
        }

        envelope.increment_hop();
        let token = envelope.header.destination_token;
        self.storage.entry(token).or_default().push(envelope);
        self.total_stored += 1;
        Ok(())
    }

    /// Retrieve and drain all pending envelopes addressed to a specific ephemeral routing token.
    pub fn fetch_for_token(&mut self, token: &RouteToken) -> Vec<EncryptedEnvelope> {
        if let Some(envelopes) = self.storage.remove(token) {
            self.total_stored = self.total_stored.saturating_sub(envelopes.len());
            envelopes
        } else {
            Vec::new()
        }
    }

    /// Prune expired packets according to packet headers.
    pub fn prune_expired(&mut self, current_time: u64) {
        let mut count = 0;
        for list in self.storage.values_mut() {
            list.retain(|env| !env.is_expired(current_time));
            count += list.len();
        }
        self.storage.retain(|_, list| !list.is_empty());
        self.total_stored = count;
    }

    pub fn total_stored(&self) -> usize {
        self.total_stored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::RoutingHeader;

    #[test]
    fn test_relay_blind_forwarding() {
        let mut relay = RelayNode::new();
        let token_bob = [0x55; 16];

        let header = RoutingHeader::new(token_bob, 1000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![0xde, 0xad, 0xbe, 0xef],
        };

        // Forward to relay
        relay
            .forward_envelope(env.clone(), 1005)
            .expect("forward succeeds");
        assert_eq!(relay.total_stored(), 1);

        // Fetch using token
        let fetched = relay.fetch_for_token(&token_bob);
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].header.hop_count, 1);
        assert_eq!(relay.total_stored(), 0);
    }
}
