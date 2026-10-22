use thiserror::Error;
use xchat_crypto::CryptoError;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Cryptographic operation failed: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Envelope error: {0}")]
    EnvelopeError(String),

    #[error("Replay attack detected for message: {0}")]
    ReplayDetected(String),

    #[error("Envelope expired (expires_at: {expires_at}, now: {now})")]
    ExpiredEnvelope { expires_at: u64, now: u64 },

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Storage/Keystore error: {0}")]
    StorageError(String),

    #[error("Routing error: {0}")]
    RoutingError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
