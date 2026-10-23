//! xChat Core Protocol Engine
//! Envelopes, store-and-forward outbox, anti-replay state, relay routing, and keystore.

pub mod contacts;
pub mod envelope;
pub mod errors;
pub mod keystore;
pub mod outbox;
pub mod relay;
pub mod replay;
pub mod session;

pub use contacts::{Contact, ContactBook};
pub use errors::CoreError;
