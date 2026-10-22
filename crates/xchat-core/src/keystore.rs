use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use xchat_crypto::{
    aead::{decrypt_aead, derive_nonce, encrypt_aead},
    identity::IdentityKeyPair,
    x3dh::{OneTimePrekeyPair, PrekeyBundle, SignedPrekeyPair},
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::errors::CoreError;

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct StoredKeyData {
    sign_seed: [u8; 32],
    dh_secret: [u8; 32],
    spk_secret: [u8; 32],
    spk_pubkey: [u8; 32],
    #[zeroize(skip)]
    spk_signature: Vec<u8>,
    otps: Vec<(u32, [u8; 32], [u8; 32])>,
}

/// In-memory and persistent cryptographic keystore.
pub struct KeyStore {
    pub identity: IdentityKeyPair,
    pub signed_prekey: SignedPrekeyPair,
    pub one_time_prekeys: Vec<OneTimePrekeyPair>,
    pub peer_bundles: HashMap<[u8; 32], PrekeyBundle>,
}

impl KeyStore {
    /// Generate a fresh keystore with new identity, signed prekey, and 10 one-time prekeys.
    pub fn generate() -> Self {
        let identity = IdentityKeyPair::generate();
        let signed_prekey = SignedPrekeyPair::generate(&identity);
        let mut one_time_prekeys = Vec::with_capacity(10);
        for id in 0..10 {
            one_time_prekeys.push(OneTimePrekeyPair::generate(id));
        }

        Self {
            identity,
            signed_prekey,
            one_time_prekeys,
            peer_bundles: HashMap::new(),
        }
    }

    /// Export public prekey bundle for publication or peer discovery.
    pub fn export_bundle(&mut self) -> PrekeyBundle {
        let opk = self.one_time_prekeys.last().map(|last| last.pubkey);

        PrekeyBundle {
            identity_key: self.identity.public_key(),
            signed_prekey: self.signed_prekey.pubkey,
            signed_prekey_signature: self.signed_prekey.signature,
            one_time_prekey: opk,
        }
    }

    /// Register a peer's public bundle.
    pub fn register_peer_bundle(&mut self, bundle: PrekeyBundle) {
        self.peer_bundles
            .insert(bundle.identity_key.sign_pubkey, bundle);
    }

    /// Save keystore to disk encrypted under a passphrase-derived key.
    pub fn save_encrypted(&self, path: &Path, encryption_key: &[u8; 32]) -> Result<(), CoreError> {
        let (sign_seed, dh_secret) = self.identity.to_bytes();

        let otps = self
            .one_time_prekeys
            .iter()
            .map(|opk| (opk.id, opk.secret.to_bytes(), opk.pubkey))
            .collect();

        let stored = StoredKeyData {
            sign_seed,
            dh_secret,
            spk_secret: self.signed_prekey.secret.to_bytes(),
            spk_pubkey: self.signed_prekey.pubkey,
            spk_signature: self.signed_prekey.signature.to_vec(),
            otps,
        };

        let serialized =
            bincode::serialize(&stored).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let nonce = derive_nonce(0xcafe, true);
        let ciphertext = encrypt_aead(encryption_key, &nonce, &serialized, b"xchat-keystore-v1")
            .map_err(CoreError::Crypto)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::StorageError(e.to_string()))?;
        }

        let mut file = File::create(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        file.write_all(&ciphertext)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Load encrypted keystore from disk.
    pub fn load_encrypted(path: &Path, encryption_key: &[u8; 32]) -> Result<Self, CoreError> {
        let mut file = File::open(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;

        let nonce = derive_nonce(0xcafe, true);
        let decrypted = decrypt_aead(encryption_key, &nonce, &ciphertext, b"xchat-keystore-v1")
            .map_err(CoreError::Crypto)?;

        let mut stored: StoredKeyData =
            bincode::deserialize(&decrypted).map_err(|e| CoreError::StorageError(e.to_string()))?;

        let identity = IdentityKeyPair::from_bytes(stored.sign_seed, stored.dh_secret);

        let mut sig_arr = [0u8; 64];
        if stored.spk_signature.len() == 64 {
            sig_arr.copy_from_slice(&stored.spk_signature);
        }
        let signed_prekey = SignedPrekeyPair {
            secret: x25519_dalek::StaticSecret::from(stored.spk_secret),
            pubkey: stored.spk_pubkey,
            signature: sig_arr,
        };

        let otps = std::mem::take(&mut stored.otps);
        let one_time_prekeys = otps
            .into_iter()
            .map(|(id, sec, pubk)| OneTimePrekeyPair {
                id,
                secret: x25519_dalek::StaticSecret::from(sec),
                pubkey: pubk,
            })
            .collect();

        Ok(Self {
            identity,
            signed_prekey,
            one_time_prekeys,
            peer_bundles: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_keystore_encryption_and_restoration() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let key = [0x77u8; 32];

        let original = KeyStore::generate();
        let orig_addr = original.identity.public_key().address();

        original.save_encrypted(path, &key).expect("save succeeds");

        let loaded = KeyStore::load_encrypted(path, &key).expect("load succeeds");
        assert_eq!(loaded.identity.public_key().address(), orig_addr);
        assert_eq!(loaded.one_time_prekeys.len(), 10);
    }
}
