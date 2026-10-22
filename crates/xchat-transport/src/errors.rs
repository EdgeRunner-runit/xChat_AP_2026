use thiserror::Error;

use crate::traits::TransportType;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    #[error("No route to host: {0}")]
    NoRouteToHost(String),

    #[error("Transport operation timed out")]
    Timeout,

    #[error("Serialization / framing error: {0}")]
    Serialization(String),

    #[error("Transport type {0:?} is currently unavailable")]
    TransportUnavailable(TransportType),

    #[error("Internal channel error: {0}")]
    ChannelError(String),
}
