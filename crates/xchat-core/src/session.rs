use xchat_crypto::{
    double_ratchet::DoubleRatchet,
    identity::IdentityPublicKey,
    x3dh::{x3dh_initiate, x3dh_respond, PrekeyBundle},
};

use crate::{
    envelope::{EncryptedEnvelope, RouteToken, RoutingHeader},
    errors::CoreError,
    keystore::KeyStore,
    replay::ReplayFilter,
};

/// High-level 1:1 secure session between two cryptographic identities.
/// Coordinates X3DH key establishment, continuous Double Ratchet, and anti-replay protection.
pub struct Session {
    pub peer_identity: IdentityPublicKey,
    ratchet: DoubleRatchet,
    replay_filter: ReplayFilter,
}

impl Session {
    /// Alice initiates an outbound secure session with Bob using Bob's prekey bundle.
    /// Immediately creates the first Double-Ratcheted encrypted envelope containing the initial plaintext.
    pub fn initiate_outbound(
        sender_keystore: &KeyStore,
        recipient_bundle: &PrekeyBundle,
        initial_message: &[u8],
        route_token: RouteToken,
        now: u64,
        ttl_seconds: u64,
    ) -> Result<(Self, EncryptedEnvelope), CoreError> {
        // 1. Perform X3DH key agreement
        let (init_msg, shared_secret) = x3dh_initiate(&sender_keystore.identity, recipient_bundle)?;

        // 2. Initialize Double Ratchet as Alice (sender)
        let mut ratchet =
            DoubleRatchet::init_alice(&shared_secret, &recipient_bundle.signed_prekey);

        // 3. Construct routing header
        let header = RoutingHeader::new(route_token, now, ttl_seconds);
        let aad = header.to_aad();

        // 4. Encrypt initial message under first sending chain key
        let (ratchet_header, ciphertext) = ratchet.ratchet_encrypt(initial_message, &aad)?;

        let envelope = EncryptedEnvelope {
            header,
            ratchet_header: Some(ratchet_header),
            x3dh_init: Some(init_msg),
            ciphertext,
        };

        let session = Self {
            peer_identity: recipient_bundle.identity_key,
            ratchet,
            replay_filter: ReplayFilter::new(),
        };

        Ok((session, envelope))
    }

    /// Bob responds to Alice's initial inbound message envelope, establishing his session.
    pub fn respond_inbound(
        recipient_keystore: &KeyStore,
        envelope: &EncryptedEnvelope,
        now: u64,
    ) -> Result<(Self, Vec<u8>), CoreError> {
        let x3dh_init = envelope.x3dh_init.as_ref().ok_or_else(|| {
            CoreError::SessionError("Initial envelope missing X3DH header".into())
        })?;

        let ratchet_header = envelope.ratchet_header.as_ref().ok_or_else(|| {
            CoreError::SessionError("Initial envelope missing Ratchet header".into())
        })?;

        // 1. Perform X3DH response
        let shared_secret = x3dh_respond(
            &recipient_keystore.identity,
            &recipient_keystore.signed_prekey.secret,
            &recipient_keystore.one_time_prekeys,
            x3dh_init,
        )?;

        // 2. Initialize Double Ratchet as Bob (recipient)
        let mut ratchet = DoubleRatchet::init_bob(
            &shared_secret,
            x3dh_dalek_clone(&recipient_keystore.signed_prekey.secret),
        );

        // 3. Check anti-replay
        let mut replay_filter = ReplayFilter::new();
        replay_filter.check_and_record(&envelope.header.message_id, now)?;

        // 4. Decrypt initial payload
        let aad = envelope.header.to_aad();
        let plaintext = ratchet.ratchet_decrypt(ratchet_header, &envelope.ciphertext, &aad)?;

        let session = Self {
            peer_identity: x3dh_init.sender_identity,
            ratchet,
            replay_filter,
        };

        Ok((session, plaintext))
    }

    /// Encrypt an outgoing message within an active ratcheted session.
    pub fn encrypt_message(
        &mut self,
        plaintext: &[u8],
        route_token: RouteToken,
        now: u64,
        ttl_seconds: u64,
    ) -> Result<EncryptedEnvelope, CoreError> {
        let header = RoutingHeader::new(route_token, now, ttl_seconds);
        let aad = header.to_aad();

        let (ratchet_header, ciphertext) = self.ratchet.ratchet_encrypt(plaintext, &aad)?;

        Ok(EncryptedEnvelope {
            header,
            ratchet_header: Some(ratchet_header),
            x3dh_init: None,
            ciphertext,
        })
    }

    /// Decrypt an incoming message envelope within an active ratcheted session.
    pub fn decrypt_message(
        &mut self,
        envelope: &EncryptedEnvelope,
        now: u64,
    ) -> Result<Vec<u8>, CoreError> {
        // 1. Check TTL
        if envelope.is_expired(now) {
            return Err(CoreError::ExpiredEnvelope {
                expires_at: envelope.header.expires_at,
                now,
            });
        }

        // 2. Enforce replay resistance
        self.replay_filter
            .check_and_record(&envelope.header.message_id, now)?;

        // 3. Verify ratchet header exists
        let ratchet_header = envelope
            .ratchet_header
            .as_ref()
            .ok_or_else(|| CoreError::SessionError("Missing ratchet header".into()))?;

        // 4. Decrypt with bound AAD
        let aad = envelope.header.to_aad();
        let plaintext = self
            .ratchet
            .ratchet_decrypt(ratchet_header, &envelope.ciphertext, &aad)?;

        Ok(plaintext)
    }
}

fn x3dh_dalek_clone(secret: &x25519_dalek::StaticSecret) -> x25519_dalek::StaticSecret {
    x25519_dalek::StaticSecret::from(secret.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end_session_lifecycle() {
        let alice_keystore = KeyStore::generate();
        let mut bob_keystore = KeyStore::generate();

        let bob_bundle = bob_keystore.export_bundle();
        let bob_route_token = [0xbb; 16];
        let alice_route_token = [0xaa; 16];
        let now = 1000;

        // Alice initiates outbound session
        let (mut alice_session, env1) = Session::initiate_outbound(
            &alice_keystore,
            &bob_bundle,
            b"Hello Bob from Alice!",
            bob_route_token,
            now,
            3600,
        )
        .expect("Alice initiates session");

        // Bob receives and establishes session
        let (mut bob_session, pt1) = Session::respond_inbound(&bob_keystore, &env1, now + 1)
            .expect("Bob establishes session");
        assert_eq!(pt1, b"Hello Bob from Alice!");

        // Bob replies back to Alice
        let env2 = bob_session
            .encrypt_message(
                b"Hi Alice, I got your message!",
                alice_route_token,
                now + 2,
                3600,
            )
            .expect("Bob encrypts reply");

        let pt2 = alice_session
            .decrypt_message(&env2, now + 3)
            .expect("Alice decrypts reply");
        assert_eq!(pt2, b"Hi Alice, I got your message!");

        // Replay of env2 must be rejected by Alice!
        let replay_err = alice_session.decrypt_message(&env2, now + 4);
        assert!(replay_err.is_err());
    }
}
