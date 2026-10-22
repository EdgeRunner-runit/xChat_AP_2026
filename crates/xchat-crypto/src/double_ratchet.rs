use std::collections::HashMap;

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::{
    aead::{decrypt_aead, derive_nonce, encrypt_aead},
    errors::CryptoError,
    kdf::{kdf_ck, kdf_rk},
    padding::{pad_message, unpad_message},
};

pub const MAX_SKIPPED_KEYS: usize = 1000;

/// Header included with every encrypted message to steer the receiving ratchet.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RatchetHeader {
    pub dh_pubkey: [u8; 32],
    pub pn: u32,
    pub n: u32,
}

impl RatchetHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("RatchetHeader serialization infallible")
    }
}

/// The core Double Ratchet state machine.
pub struct DoubleRatchet {
    dhs: StaticSecret,
    dhr: Option<[u8; 32]>,
    rk: [u8; 32],
    cks: Option<[u8; 32]>,
    ckr: Option<[u8; 32]>,
    ns: u32,
    nr: u32,
    pn: u32,
    skipped_message_keys: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl DoubleRatchet {
    /// Initialize Alice (the party who sends the first message after X3DH).
    pub fn init_alice(shared_secret: &[u8; 32], bob_dh_pubkey: &[u8; 32]) -> Self {
        let rng = OsRng;
        let dhs = StaticSecret::random_from_rng(rng);
        let dh_pub = X25519PublicKey::from(*bob_dh_pubkey);
        let dh_out = dhs.diffie_hellman(&dh_pub).to_bytes();

        let (rk, cks) = kdf_rk(shared_secret, &dh_out);

        Self {
            dhs,
            dhr: Some(*bob_dh_pubkey),
            rk,
            cks: Some(cks),
            ckr: None,
            ns: 0,
            nr: 0,
            pn: 0,
            skipped_message_keys: HashMap::new(),
        }
    }

    /// Initialize Bob (the party who receives the first message after X3DH).
    pub fn init_bob(shared_secret: &[u8; 32], bob_dh_keypair: StaticSecret) -> Self {
        Self {
            dhs: bob_dh_keypair,
            dhr: None,
            rk: *shared_secret,
            cks: None,
            ckr: None,
            ns: 0,
            nr: 0,
            pn: 0,
            skipped_message_keys: HashMap::new(),
        }
    }

    /// Encrypt a message under the current sending chain.
    pub fn ratchet_encrypt(
        &mut self,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<(RatchetHeader, Vec<u8>), CryptoError> {
        let cks = self
            .cks
            .ok_or_else(|| CryptoError::RatchetError("Sending chain key not established".into()))?;

        // 1. Advance symmetric sending chain
        let (next_cks, message_key) = kdf_ck(&cks);
        self.cks = Some(next_cks);

        // 2. Form header
        let my_dh_pubkey = X25519PublicKey::from(&self.dhs).to_bytes();
        let header = RatchetHeader {
            dh_pubkey: my_dh_pubkey,
            pn: self.pn,
            n: self.ns,
        };

        // 3. Derive nonce from message sequence
        let nonce = derive_nonce(self.ns as u64, true);
        self.ns += 1;

        // 4. Pad payload and build combined AAD (associated_data + serialized header)
        let padded = pad_message(plaintext);
        let mut full_aad = Vec::with_capacity(associated_data.len() + 40);
        full_aad.extend_from_slice(associated_data);
        full_aad.extend_from_slice(&header.to_bytes());

        // 5. Encrypt with AEAD ChaCha20-Poly1305
        let ciphertext = encrypt_aead(&message_key, &nonce, &padded, &full_aad)?;

        Ok((header, ciphertext))
    }

    /// Decrypt a message received from peer, advancing ratchet or retrieving skipped keys.
    pub fn ratchet_decrypt(
        &mut self,
        header: &RatchetHeader,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut full_aad = Vec::with_capacity(associated_data.len() + 40);
        full_aad.extend_from_slice(associated_data);
        full_aad.extend_from_slice(&header.to_bytes());

        // Check if message key was previously skipped (e.g. arrived out-of-order)
        let key_lookup = (header.dh_pubkey, header.n);
        if let Some(mut message_key) = self.skipped_message_keys.remove(&key_lookup) {
            let nonce = derive_nonce(header.n as u64, true);
            let decrypted = decrypt_aead(&message_key, &nonce, ciphertext, &full_aad)?;
            message_key.zeroize();
            return unpad_message(&decrypted);
        }

        // If peer's DH ratchet key changed, perform DH ratchet step
        if self.dhr != Some(header.dh_pubkey) {
            self.skip_message_keys(header.pn)?;
            self.dh_ratchet(header.dh_pubkey)?;
        }

        // Skip any message keys in current chain up to header.n
        self.skip_message_keys(header.n)?;

        // Advance receiving chain
        let ckr = self.ckr.ok_or_else(|| {
            CryptoError::RatchetError("Receiving chain key not established".into())
        })?;
        let (next_ckr, mut message_key) = kdf_ck(&ckr);
        self.ckr = Some(next_ckr);
        self.nr += 1;

        let nonce = derive_nonce(header.n as u64, true);
        let decrypted = decrypt_aead(&message_key, &nonce, ciphertext, &full_aad)?;
        message_key.zeroize();

        unpad_message(&decrypted)
    }

    fn dh_ratchet(&mut self, remote_dh_pubkey: [u8; 32]) -> Result<(), CryptoError> {
        self.pn = self.ns;
        self.ns = 0;
        self.nr = 0;
        self.dhr = Some(remote_dh_pubkey);

        let remote_pub = X25519PublicKey::from(remote_dh_pubkey);

        // DH ratchet step 1: derive new receiving chain with existing local DH and new remote DH
        let dh_recv = self.dhs.diffie_hellman(&remote_pub).to_bytes();
        let (rk1, ckr) = kdf_rk(&self.rk, &dh_recv);
        self.rk = rk1;
        self.ckr = Some(ckr);

        // DH ratchet step 2: generate new local DH keypair and derive new sending chain
        let rng = OsRng;
        self.dhs = StaticSecret::random_from_rng(rng);
        let dh_send = self.dhs.diffie_hellman(&remote_pub).to_bytes();
        let (rk2, cks) = kdf_rk(&self.rk, &dh_send);
        self.rk = rk2;
        self.cks = Some(cks);

        Ok(())
    }

    fn skip_message_keys(&mut self, until: u32) -> Result<(), CryptoError> {
        if let (Some(mut ckr), Some(dhr)) = (self.ckr, self.dhr) {
            if self.nr.saturating_add(MAX_SKIPPED_KEYS as u32) < until {
                return Err(CryptoError::SkippedKeyLimitExceeded(MAX_SKIPPED_KEYS));
            }

            while self.nr < until {
                let (next_ckr, message_key) = kdf_ck(&ckr);
                ckr = next_ckr;

                if self.skipped_message_keys.len() >= MAX_SKIPPED_KEYS {
                    return Err(CryptoError::SkippedKeyLimitExceeded(MAX_SKIPPED_KEYS));
                }

                self.skipped_message_keys
                    .insert((dhr, self.nr), message_key);
                self.nr += 1;
            }
            self.ckr = Some(ckr);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_ratchet_conversation() {
        let shared_secret = [0x5au8; 32];

        // Bob generates his prekey DH
        let mut rng = OsRng;
        let bob_dh = StaticSecret::random_from_rng(&mut rng);
        let bob_pub = X25519PublicKey::from(&bob_dh).to_bytes();

        let mut alice = DoubleRatchet::init_alice(&shared_secret, &bob_pub);
        let mut bob = DoubleRatchet::init_bob(&shared_secret, bob_dh);

        let aad = b"test_metadata";

        // 1. Alice -> Bob message 1
        let (h1, ct1) = alice.ratchet_encrypt(b"Hello Bob!", aad).unwrap();
        let pt1 = bob.ratchet_decrypt(&h1, &ct1, aad).unwrap();
        assert_eq!(pt1, b"Hello Bob!");

        // 2. Alice -> Bob message 2
        let (h2, ct2) = alice.ratchet_encrypt(b"Are you there?", aad).unwrap();
        let pt2 = bob.ratchet_decrypt(&h2, &ct2, aad).unwrap();
        assert_eq!(pt2, b"Are you there?");

        // 3. Bob -> Alice reply 1 (triggers DH ratchet!)
        let (h3, ct3) = bob.ratchet_encrypt(b"Hi Alice, I am here!", aad).unwrap();
        let pt3 = alice.ratchet_decrypt(&h3, &ct3, aad).unwrap();
        assert_eq!(pt3, b"Hi Alice, I am here!");

        // 4. Bob -> Alice reply 2
        let (h4, ct4) = bob.ratchet_encrypt(b"Everything is secure.", aad).unwrap();
        let pt4 = alice.ratchet_decrypt(&h4, &ct4, aad).unwrap();
        assert_eq!(pt4, b"Everything is secure.");

        // 5. Alice -> Bob reply 3
        let (h5, ct5) = alice
            .ratchet_encrypt(b"Great, ratcheting works!", aad)
            .unwrap();
        let pt5 = bob.ratchet_decrypt(&h5, &ct5, aad).unwrap();
        assert_eq!(pt5, b"Great, ratcheting works!");
    }

    #[test]
    fn test_out_of_order_message_handling() {
        let shared_secret = [0x7cu8; 32];
        let mut rng = OsRng;
        let bob_dh = StaticSecret::random_from_rng(&mut rng);
        let bob_pub = X25519PublicKey::from(&bob_dh).to_bytes();

        let mut alice = DoubleRatchet::init_alice(&shared_secret, &bob_pub);
        let mut bob = DoubleRatchet::init_bob(&shared_secret, bob_dh);
        let aad = b"out_of_order";

        // Alice sends 3 messages
        let (h0, ct0) = alice.ratchet_encrypt(b"Message 0", aad).unwrap();
        let (h1, ct1) = alice.ratchet_encrypt(b"Message 1", aad).unwrap();
        let (h2, ct2) = alice.ratchet_encrypt(b"Message 2", aad).unwrap();

        // Deliver Message 2 first!
        let pt2 = bob.ratchet_decrypt(&h2, &ct2, aad).unwrap();
        assert_eq!(pt2, b"Message 2");

        // Then deliver Message 0
        let pt0 = bob.ratchet_decrypt(&h0, &ct0, aad).unwrap();
        assert_eq!(pt0, b"Message 0");

        // Then deliver Message 1
        let pt1 = bob.ratchet_decrypt(&h1, &ct1, aad).unwrap();
        assert_eq!(pt1, b"Message 1");
    }
}
