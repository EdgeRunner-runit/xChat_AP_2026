//! xChat Core Protocol Engine
//! Envelopes, store-and-forward outbox, anti-replay state, relay routing,
//! keystore, contact book, emergency SOS, burner rooms, duress vaults, and chunking.

pub mod burner;
pub mod chunking;
pub mod contacts;
pub mod cover_traffic;
pub mod duress;
pub mod envelope;
pub mod errors;
pub mod keystore;
pub mod mixnet;
pub mod outbox;
pub mod relay;
pub mod replay;
pub mod session;
pub mod sos;

pub use burner::BurnerRoom;
pub use chunking::{chunk_payload, ChunkReassembler, FileChunk};
pub use contacts::{Contact, ContactBook};
pub use duress::{DuressVaultManager, VaultMode};
pub use errors::CoreError;
pub use mixnet::{MixPool, MixPoolConfig, MixedBatch};
pub use sos::{create_emergency_beacon, decode_emergency_beacon, SosBeacon};
