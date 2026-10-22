use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};
use xchat_core::envelope::EncryptedEnvelope;

use crate::{
    errors::TransportError,
    traits::{Transport, TransportType},
};

/// Orchestrates multiple heterogeneous network transports.
/// Multiplexes inbound envelopes and handles transport failover without resetting crypto sessions.
pub struct TransportManager {
    transports: Mutex<Vec<Arc<dyn Transport>>>,
    merged_inbound_rx: Mutex<mpsc::Receiver<EncryptedEnvelope>>,
    merged_inbound_tx: mpsc::Sender<EncryptedEnvelope>,
}

impl TransportManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(512);
        Self {
            transports: Mutex::new(Vec::new()),
            merged_inbound_rx: Mutex::new(rx),
            merged_inbound_tx: tx,
        }
    }

    /// Register a new physical or logical transport (e.g. TCP LAN, Bluetooth LE).
    pub async fn register_transport(&self, transport: Arc<dyn Transport>) {
        let mut list = self.transports.lock().await;
        list.push(transport.clone());

        // Spawn background listener forwarding from this transport into merged receiver
        let tx = self.merged_inbound_tx.clone();
        tokio::spawn(async move {
            while transport.is_available() {
                match transport.receive_envelope().await {
                    Ok(envelope) => {
                        if tx.send(envelope).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    /// Dispatch an envelope across available transports with automatic failover.
    pub async fn dispatch(
        &self,
        envelope: &EncryptedEnvelope,
        preferred_target: Option<(&str, TransportType)>,
    ) -> Result<(), TransportError> {
        let transports = self.transports.lock().await;

        // 1. Try preferred transport if specified
        if let Some((target_addr, target_type)) = preferred_target {
            for t in transports.iter() {
                if t.transport_type() == target_type
                    && t.is_available()
                    && t.send_envelope(target_addr, envelope).await.is_ok()
                {
                    return Ok(());
                }
            }
        }

        // 2. Fall back to broadcasting / sending on any available transport
        let mut any_delivered = false;
        for t in transports.iter() {
            if t.is_available() {
                if let Ok(count) = t.broadcast_envelope(envelope).await {
                    if count > 0 {
                        any_delivered = true;
                    }
                }
            }
        }

        if any_delivered {
            Ok(())
        } else {
            Err(TransportError::NoRouteToHost(
                "No reachable active transport delivered envelope".into(),
            ))
        }
    }

    /// Await the next inbound envelope arriving from any active transport.
    pub async fn receive_next(&self) -> Result<EncryptedEnvelope, TransportError> {
        let mut rx = self.merged_inbound_rx.lock().await;
        rx.recv().await.ok_or(TransportError::ConnectionClosed)
    }

    /// Count how many registered transports are currently active.
    pub async fn active_transport_count(&self) -> usize {
        let list = self.transports.lock().await;
        list.iter().filter(|t| t.is_available()).count()
    }
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_sim::{BleMeshSimulationTransport, SharedRadioMedium};
    use xchat_core::envelope::RoutingHeader;

    #[tokio::test]
    async fn test_transport_manager_multiplexing() {
        let medium = Arc::new(SharedRadioMedium::new());
        let ble_t =
            Arc::new(BleMeshSimulationTransport::new("node_1".into(), medium.clone()).await);

        let manager = TransportManager::new();
        manager.register_transport(ble_t.clone()).await;

        let peer_ble = BleMeshSimulationTransport::new("node_2".into(), medium.clone()).await;

        let header = RoutingHeader::new([0x77; 16], 1000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: b"Multiplexed payload".to_vec(),
        };

        // Peer sends to node_1 over BLE
        peer_ble.send_envelope("node_1", &env).await.unwrap();

        // Manager receives on its merged stream
        let received = manager.receive_next().await.unwrap();
        assert_eq!(received.ciphertext, b"Multiplexed payload");
    }
}
