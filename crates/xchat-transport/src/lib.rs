//! xChat Transport Abstraction Layer
//! Multi-transport interfaces, TCP framing, Bluetooth/BLE wireless mesh simulation,
//! and multi-transport failover/routing manager.

pub mod errors;
pub mod manager;
pub mod mesh_sim;
pub mod tcp;
pub mod traits;

pub use errors::TransportError;
pub use traits::{Transport, TransportType};
