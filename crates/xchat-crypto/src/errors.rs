use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    #[error("AEAD encryption error: {0}")]
    AeadError(String),

    #[error("AEAD decryption error (authentication failed)")]
    DecryptionFailed,

    #[error("Invalid cryptographic signature")]
    InvalidSignature,

    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },

    #[error("Double Ratchet error: {0}")]
    RatchetError(String),

    #[error("X3DH protocol error: {0}")]
    X3dhError(String),

    #[error("Serialization / Deserialization error: {0}")]
    SerializationError(String),

    #[error("Skipped keys limit exceeded ({0})")]
    SkippedKeyLimitExceeded(usize),

    #[error("Padding error: invalid padding structure")]
    PaddingError,

    #[error("Replay detected: message key already used or discarded")]
    ReplayDetected,
}
