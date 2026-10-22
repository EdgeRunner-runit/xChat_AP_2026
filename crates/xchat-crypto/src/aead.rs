use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};

use crate::errors::CryptoError;

pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;
pub const TAG_LEN: usize = 16;

/// Encrypt plaintext with ChaCha20-Poly1305 and bind associated data.
pub fn encrypt_aead(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let n = Nonce::from_slice(nonce);

    let payload = Payload {
        msg: plaintext,
        aad: associated_data,
    };

    cipher
        .encrypt(n, payload)
        .map_err(|e| CryptoError::AeadError(e.to_string()))
}

/// Decrypt ciphertext with ChaCha20-Poly1305 and verify associated data.
pub fn decrypt_aead(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let n = Nonce::from_slice(nonce);

    let payload = Payload {
        msg: ciphertext,
        aad: associated_data,
    };

    cipher
        .decrypt(n, payload)
        .map_err(|_| CryptoError::DecryptionFailed)
}

/// Deterministically construct a 12-byte nonce from a message counter and direction flag.
/// Format: [0..4]: direction + padding, [4..12]: 64-bit big-endian counter
pub fn derive_nonce(counter: u64, is_sender: bool) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[0] = if is_sender { 0x01 } else { 0x02 };
    nonce[4..12].copy_from_slice(&counter.to_be_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aead_roundtrip() {
        let key = [42u8; 32];
        let nonce = derive_nonce(7, true);
        let msg = b"xChat end-to-end secret payload";
        let aad = b"authenticated_route_metadata";

        let ct = encrypt_aead(&key, &nonce, msg, aad).expect("encryption succeeds");
        assert_ne!(ct, msg);

        let pt = decrypt_aead(&key, &nonce, &ct, aad).expect("decryption succeeds");
        assert_eq!(pt, msg);

        // AAD tampering must fail
        let tampered_aad = b"malicious_route_metadata";
        let err = decrypt_aead(&key, &nonce, &ct, tampered_aad);
        assert_eq!(err, Err(CryptoError::DecryptionFailed));

        // Ciphertext tampering must fail
        let mut tampered_ct = ct.clone();
        tampered_ct[0] ^= 0xff;
        let err2 = decrypt_aead(&key, &nonce, &tampered_ct, aad);
        assert_eq!(err2, Err(CryptoError::DecryptionFailed));
    }
}
