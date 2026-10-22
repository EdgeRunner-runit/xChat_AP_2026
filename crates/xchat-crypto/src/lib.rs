//! xChat Cryptography Engine
//! Implements Ed25519 identity keys, X25519 key exchange, X3DH initial handshake,
//! Double Ratchet session ratcheting, ChaCha20-Poly1305 AEAD, HKDF key derivation,
//! bucketed padding, and ephemeral discovery tokens.

pub mod aead;
pub mod double_ratchet;
pub mod errors;
pub mod identity;
pub mod kdf;
pub mod padding;
pub mod x3dh;

pub use errors::CryptoError;
