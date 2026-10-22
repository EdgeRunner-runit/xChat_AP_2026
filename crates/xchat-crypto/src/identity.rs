use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{errors::CryptoError, kdf::derive_discovery_token};

/// Long-term cryptographic identity keypair for an xChat device.
/// Combines an Ed25519 identity key for digital signatures with an X25519 key for Diffie-Hellman.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct IdentityKeyPair {
    #[zeroize(skip)]
    pub signing_key: SigningKey,
    pub dh_secret: StaticSecret,
}

/// Public component of a device's cryptographic identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct IdentityPublicKey {
    pub sign_pubkey: [u8; 32],
    pub dh_pubkey: [u8; 32],
}

impl IdentityKeyPair {
    /// Generate a brand new cryptographic identity using OS entropy.
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let dh_secret = StaticSecret::random_from_rng(rng);
        Self {
            signing_key,
            dh_secret,
        }
    }

    /// Restore an identity keypair from raw 32-byte seed / secret bytes.
    pub fn from_bytes(sign_seed: [u8; 32], dh_secret_bytes: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&sign_seed);
        let dh_secret = StaticSecret::from(dh_secret_bytes);
        Self {
            signing_key,
            dh_secret,
        }
    }

    /// Export private keys as raw bytes (for local encrypted keystore storage).
    pub fn to_bytes(&self) -> ([u8; 32], [u8; 32]) {
        (self.signing_key.to_bytes(), self.dh_secret.to_bytes())
    }

    /// Get the associated public identity.
    pub fn public_key(&self) -> IdentityPublicKey {
        let sign_pubkey = self.signing_key.verifying_key().to_bytes();
        let dh_pubkey = X25519PublicKey::from(&self.dh_secret).to_bytes();
        IdentityPublicKey {
            sign_pubkey,
            dh_pubkey,
        }
    }

    /// Sign a message using the Ed25519 identity key.
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        self.signing_key.sign(data).to_bytes()
    }

    /// Perform classical X25519 Diffie-Hellman with a remote public key.
    pub fn diffie_hellman(&self, peer_dh_pubkey: &[u8; 32]) -> [u8; 32] {
        let remote_public = X25519PublicKey::from(*peer_dh_pubkey);
        self.dh_secret.diffie_hellman(&remote_public).to_bytes()
    }
}

impl IdentityPublicKey {
    /// Verify an Ed25519 signature.
    pub fn verify(&self, data: &[u8], signature_bytes: &[u8; 64]) -> Result<(), CryptoError> {
        let vk = VerifyingKey::from_bytes(&self.sign_pubkey)
            .map_err(|_| CryptoError::InvalidSignature)?;
        let sig = Signature::from_bytes(signature_bytes);
        vk.verify(data, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Human-readable address representation (hex formatted).
    pub fn address(&self) -> String {
        format!("xchat_{}", hex::encode(&self.sign_pubkey[..8]))
    }

    /// Full fingerprint for verification (safety numbers / QR codes).
    pub fn fingerprint(&self) -> String {
        hex::encode(self.sign_pubkey)
    }

    /// Compute rotating ephemeral discovery token for metadata-safe nearby advertising.
    pub fn ephemeral_discovery_token(&self, epoch: u64) -> [u8; 16] {
        derive_discovery_token(&self.sign_pubkey, epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_signature_and_verification() {
        let alice = IdentityKeyPair::generate();
        let alice_pub = alice.public_key();

        let msg = b"xChat protocol signed test payload";
        let sig = alice.sign(msg);
        assert!(alice_pub.verify(msg, &sig).is_ok());

        let mut tampered = sig;
        tampered[0] ^= 0x55;
        assert!(alice_pub.verify(msg, &tampered).is_err());
    }

    #[test]
    fn test_identity_dh_exchange() {
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();

        let alice_pub = alice.public_key();
        let bob_pub = bob.public_key();

        let secret_ab = alice.diffie_hellman(&bob_pub.dh_pubkey);
        let secret_ba = bob.diffie_hellman(&alice_pub.dh_pubkey);

        assert_eq!(secret_ab, secret_ba);
    }
}
