use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use xchat_core::envelope::EncryptedEnvelope;

use crate::{
    errors::TransportError,
    traits::{Transport, TransportType},
};

pub const MAX_ENVELOPE_FRAME_SIZE: usize = 10 * 1024 * 1024; // 10 MB maximum payload

/// Production-ready TCP/IP Transport for Local LAN and Internet peer connections.
pub struct TcpTransport {
    local_addr: SocketAddr,
    inbound_rx: Mutex<mpsc::Receiver<EncryptedEnvelope>>,
    known_peers: Arc<Mutex<Vec<String>>>,
    is_active: Arc<std::sync::atomic::AtomicBool>,
}

impl TcpTransport {
    /// Bind a new TCP transport listener on the requested address (use port 0 for OS ephemeral port).
    pub async fn bind(bind_addr: &str) -> Result<Arc<Self>, TransportError> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;

        let (tx, rx) = mpsc::channel(256);
        let known_peers = Arc::new(Mutex::new(Vec::new()));
        let is_active = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let active_flag = is_active.clone();
        tokio::spawn(async move {
            while active_flag.load(std::sync::atomic::Ordering::Relaxed) {
                match listener.accept().await {
                    Ok((mut socket, _peer_addr)) => {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            if let Ok(envelope) = Self::read_framed_envelope(&mut socket).await {
                                let _ = tx.send(envelope).await;
                            }
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Arc::new(Self {
            local_addr,
            inbound_rx: Mutex::new(rx),
            known_peers,
            is_active,
        }))
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn add_peer(&self, peer_addr: String) {
        let mut peers = self.known_peers.lock().await;
        if !peers.contains(&peer_addr) {
            peers.push(peer_addr);
        }
    }

    async fn read_framed_envelope(
        stream: &mut TcpStream,
    ) -> Result<EncryptedEnvelope, TransportError> {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let frame_len = u32::from_be_bytes(len_buf) as usize;

        if frame_len > MAX_ENVELOPE_FRAME_SIZE {
            return Err(TransportError::Serialization(
                "Frame size exceeds limit".into(),
            ));
        }

        let mut body = vec![0u8; frame_len];
        stream.read_exact(&mut body).await?;

        EncryptedEnvelope::from_bytes(&body)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }

    async fn write_framed_envelope(
        stream: &mut TcpStream,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        let payload = envelope
            .to_bytes()
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        let frame_len = (payload.len() as u32).to_be_bytes();
        stream.write_all(&frame_len).await?;
        stream.write_all(&payload).await?;
        stream.flush().await?;
        Ok(())
    }
}

#[async_trait]
impl Transport for TcpTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::LocalWifi
    }

    async fn send_envelope(
        &self,
        target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        let mut stream = TcpStream::connect(target_address).await?;
        Self::write_framed_envelope(&mut stream, envelope).await
    }

    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError> {
        let peers = self.known_peers.lock().await.clone();
        let mut delivered = 0;
        for peer in peers {
            if self.send_envelope(&peer, envelope).await.is_ok() {
                delivered += 1;
            }
        }
        Ok(delivered)
    }

    async fn receive_envelope(&self) -> Result<EncryptedEnvelope, TransportError> {
        let mut rx = self.inbound_rx.lock().await;
        rx.recv().await.ok_or(TransportError::ConnectionClosed)
    }

    fn is_available(&self) -> bool {
        self.is_active.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xchat_core::envelope::RoutingHeader;

    #[tokio::test]
    async fn test_tcp_transport_transmission() {
        let node_a = TcpTransport::bind("127.0.0.1:0").await.unwrap();
        let node_b = TcpTransport::bind("127.0.0.1:0").await.unwrap();

        let node_b_addr = node_b.local_addr().to_string();

        let header = RoutingHeader::new([0x33; 16], 1000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: b"TCP network payload".to_vec(),
        };

        // Node A sends to Node B
        node_a.send_envelope(&node_b_addr, &env).await.unwrap();

        // Node B receives
        let received = node_b.receive_envelope().await.unwrap();
        assert_eq!(received.ciphertext, b"TCP network payload");
    }
}
