//! Internet & Cellular Mixnet Transport
//!
//! Wraps a physical network transport (such as TCP/IP over Internet or mobile Cellular Data)
//! with a Chaumian Mix Pool buffer to provide timing obfuscation, batch shuffling,
//! and synthetic cover traffic.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use xchat_core::envelope::EncryptedEnvelope;
use xchat_core::mixnet::{MixPool, MixPoolConfig, MixedBatch};

use crate::errors::TransportError;
use crate::traits::{Transport, TransportType};

/// A transport decorator that applies Chaumian message mixing and shuffling
/// to an underlying physical transport before transmission over Internet or Cellular.
pub struct MixnetTransport {
    inner: Arc<dyn Transport>,
    mix_pool: Arc<Mutex<MixPool>>,
    transport_type: TransportType,
}

impl MixnetTransport {
    /// Create a new MixnetTransport wrapping the specified transport.
    pub fn new(
        inner: Arc<dyn Transport>,
        config: MixPoolConfig,
        transport_type: TransportType,
    ) -> Self {
        Self {
            inner,
            mix_pool: Arc::new(Mutex::new(MixPool::new(config))),
            transport_type,
        }
    }

    /// Access the internal mix pool to flush pending batched messages.
    pub async fn flush_and_dispatch(&self) -> Result<Option<MixedBatch>, TransportError> {
        let mut pool = self.mix_pool.lock().await;
        if let Some(batch) = pool.flush() {
            // Dispatch each envelope with randomized jitter
            for (env, target) in batch.envelopes.iter().zip(batch.targets.iter()) {
                let jitter = pool.sample_jitter_delay();
                tokio::time::sleep(jitter).await;

                if let Some(t) = target {
                    let _ = self.inner.send_envelope(t, env).await;
                } else {
                    let _ = self.inner.broadcast_envelope(env).await;
                }
            }
            Ok(Some(batch))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl Transport for MixnetTransport {
    fn transport_type(&self) -> TransportType {
        self.transport_type
    }

    async fn send_envelope(
        &self,
        target_address: &str,
        envelope: &EncryptedEnvelope,
    ) -> Result<(), TransportError> {
        let mut pool = self.mix_pool.lock().await;
        pool.submit(envelope.clone(), Some(target_address.to_string()));

        if pool.should_flush() {
            drop(pool);
            let _ = self.flush_and_dispatch().await;
        }
        Ok(())
    }

    async fn broadcast_envelope(
        &self,
        envelope: &EncryptedEnvelope,
    ) -> Result<usize, TransportError> {
        let mut pool = self.mix_pool.lock().await;
        pool.submit(envelope.clone(), None);

        if pool.should_flush() {
            drop(pool);
            let _ = self.flush_and_dispatch().await;
        }
        Ok(1)
    }

    async fn receive_envelope(&self) -> Result<EncryptedEnvelope, TransportError> {
        // Inbound packets are received from the physical link
        self.inner.receive_envelope().await
    }

    fn is_available(&self) -> bool {
        self.inner.is_available()
    }
}
