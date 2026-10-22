use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use xchat_core::envelope::EncryptedEnvelope;

use crate::errors::TransportError;

/// Physical and logical network transport varieties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// Wide Area Network / Internet over TCP / TLS
    InternetTcp,
    /// Local Area Network over Wi-Fi / Multicast
    LocalWifi,
    /// Direct peer-to-peer Bluetooth Low Energy / ad-hoc radio mesh
    BluetoothMesh,
}

/// Abstract transport layer decoupled from upper cryptographic and routing layers.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Return the transport medium category.
    fn transport_type(&self) -> TransportType;

    /// Direct point-to-point dispatch to a known transport address.
    async fn send_envelope(
        &self,
        target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError>;

    /// Broadcast an opaque envelope to all directly reachable neighbors (e.g. BLE advertisement / LAN broadcast).
    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError>;

    /// Poll or await the next inbound envelope received on this transport.
    async fn receive_envelope(&self) -> Result<EncryptedEnvelope, TransportError>;

    /// Verify whether this physical link is currently active and reachable.
    fn is_available(&self) -> bool;
}
