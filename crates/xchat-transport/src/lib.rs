//! xChat Transport Abstraction Layer
//! Multi-transport interfaces, TCP framing, Bluetooth/BLE wireless mesh simulation,
//! and multi-transport failover/routing manager.

pub mod audio_modem;
pub mod errors;
pub mod manager;
pub mod mesh_sim;
pub mod mixnet_transport;
pub mod sms;
pub mod tcp;
pub mod traits;

pub use audio_modem::AudioCallModem;
pub use errors::TransportError;
pub use mixnet_transport::MixnetTransport;
pub use sms::SmsCarrier;
pub use traits::{Transport, TransportType};
