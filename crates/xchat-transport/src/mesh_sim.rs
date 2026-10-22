use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use xchat_core::envelope::EncryptedEnvelope;

use crate::{
    errors::TransportError,
    traits::{Transport, TransportType},
};

/// Shared radio frequency / Bluetooth Low Energy virtual medium.
pub struct SharedRadioMedium {
    channels: Arc<Mutex<HashMap<String, mpsc::Sender<EncryptedEnvelope>>>>,
}

impl SharedRadioMedium {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_node(&self, node_id: String) -> mpsc::Receiver<EncryptedEnvelope> {
        let (tx, rx) = mpsc::channel(256);
        let mut map = self.channels.lock().await;
        map.insert(node_id, tx);
        rx
    }

    pub async fn unregister_node(&self, node_id: &str) {
        let mut map = self.channels.lock().await;
        map.remove(node_id);
    }
}

impl Default for SharedRadioMedium {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulated Bluetooth / BLE wireless mesh transport.
pub struct BleMeshSimulationTransport {
    node_id: String,
    medium: Arc<SharedRadioMedium>,
    inbound_rx: Mutex<mpsc::Receiver<EncryptedEnvelope>>,
    connected_neighbors: Arc<Mutex<Vec<String>>>,
    is_radio_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl BleMeshSimulationTransport {
    pub async fn new(node_id: String, medium: Arc<SharedRadioMedium>) -> Self {
        let rx = medium.register_node(node_id.clone()).await;
        Self {
            node_id,
            medium,
            inbound_rx: Mutex::new(rx),
            connected_neighbors: Arc::new(Mutex::new(Vec::new())),
            is_radio_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    pub async fn pair_neighbor(&self, neighbor_id: String) {
        let mut n = self.connected_neighbors.lock().await;
        if !n.contains(&neighbor_id) {
            n.push(neighbor_id);
        }
    }

    pub fn set_radio_power(&self, enabled: bool) {
        self.is_radio_enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[async_trait]
impl Transport for BleMeshSimulationTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::BluetoothMesh
    }

    async fn send_envelope(
        &self,
        target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        if !self.is_available() {
            return Err(TransportError::TransportUnavailable(self.transport_type()));
        }

        let map = self.medium.channels.lock().await;
        if let Some(tx) = map.get(target_address) {
            tx.send(envelope.clone())
                .await
                .map_err(|e| TransportError::ChannelError(e.to_string()))?;
            Ok(())
        } else {
            Err(TransportError::NoRouteToHost(target_address.to_string()))
        }
    }

    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError> {
        if !self.is_available() {
            return Err(TransportError::TransportUnavailable(self.transport_type()));
        }

        let neighbors = self.connected_neighbors.lock().await.clone();
        let map = self.medium.channels.lock().await;
        let mut count = 0;

        for neighbor in neighbors {
            if let Some(tx) = map.get(&neighbor) {
                if tx.send(envelope.clone()).await.is_ok() {
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    async fn receive_envelope(&self) -> Result<EncryptedEnvelope, TransportError> {
        let mut rx = self.inbound_rx.lock().await;
        rx.recv().await.ok_or(TransportError::ConnectionClosed)
    }

    fn is_available(&self) -> bool {
        self.is_radio_enabled
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xchat_core::envelope::RoutingHeader;

    #[tokio::test]
    async fn test_ble_mesh_broadcast() {
        let medium = Arc::new(SharedRadioMedium::new());
        let node_alice = BleMeshSimulationTransport::new("alice_ble".into(), medium.clone()).await;
        let node_bob = BleMeshSimulationTransport::new("bob_ble".into(), medium.clone()).await;

        node_alice.pair_neighbor("bob_ble".into()).await;

        let header = RoutingHeader::new([0x99; 16], 1000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: b"Bluetooth mesh advertisement packet".to_vec(),
        };

        let delivered = node_alice.broadcast_envelope(&env).await.unwrap();
        assert_eq!(delivered, 1);

        let received = node_bob.receive_envelope().await.unwrap();
        assert_eq!(received.ciphertext, b"Bluetooth mesh advertisement packet");
    }
}
