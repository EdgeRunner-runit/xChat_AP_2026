//! Chaumian Message Mixing Pool (Mixnet)
//!
//! Provides traffic analysis resistance, unlinkability, and timing obfuscation
//! across Internet and cellular mobile data transports.
//!
//! Mix pools buffer inbound and outbound envelopes, inject synthetic cover traffic
//! to ensure constant batch sizes, cryptographically shuffle messages, and introduce
//! randomized jitter delays to prevent adversaries from correlating packet ingress
//! and egress timings.

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::info;

use crate::cover_traffic::generate_cover_envelope;
use crate::envelope::EncryptedEnvelope;

/// Configuration options for the Chaumian Mix Pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixPoolConfig {
    /// Minimum number of envelopes in a batch before dispatch.
    pub batch_size: usize,
    /// Maximum duration an envelope will sit in the pool before being forced out.
    pub max_delay: Duration,
    /// Whether to pad undersized batches with synthetic cover traffic envelopes.
    pub inject_cover_traffic: bool,
    /// Minimum jitter delay in milliseconds for individual envelope dispatch.
    pub min_jitter_ms: u64,
    /// Maximum jitter delay in milliseconds for individual envelope dispatch.
    pub max_jitter_ms: u64,
}

impl Default for MixPoolConfig {
    fn default() -> Self {
        Self {
            batch_size: 4,
            max_delay: Duration::from_millis(500),
            inject_cover_traffic: true,
            min_jitter_ms: 10,
            max_jitter_ms: 100,
        }
    }
}

/// A queued item in the mix pool awaiting batch dispatch.
#[derive(Debug, Clone)]
struct QueuedEnvelope {
    envelope: EncryptedEnvelope,
    enqueued_at: Instant,
    target_hint: Option<String>,
}

/// Chaumian Mix Pool buffer for anonymizing traffic over Internet and cellular data.
pub struct MixPool {
    config: MixPoolConfig,
    queue: Vec<QueuedEnvelope>,
    last_flush: Instant,
}

/// A mixed, shuffled batch ready for dispatch with calculated timing delays.
#[derive(Debug, Clone)]
pub struct MixedBatch {
    /// Shuffled list of envelopes (real messages and synthetic cover envelopes).
    pub envelopes: Vec<EncryptedEnvelope>,
    /// Number of genuine user envelopes in this batch.
    pub real_count: usize,
    /// Number of synthetic cover envelopes injected into this batch.
    pub cover_count: usize,
    /// Associated target hints (if any) matched to envelopes post-shuffle.
    pub targets: Vec<Option<String>>,
}

impl MixPool {
    /// Create a new MixPool with the provided configuration.
    pub fn new(config: MixPoolConfig) -> Self {
        Self {
            config,
            queue: Vec::new(),
            last_flush: Instant::now(),
        }
    }

    /// Add an envelope to the mix pool buffer.
    pub fn submit(&mut self, envelope: EncryptedEnvelope, target_hint: Option<String>) {
        self.queue.push(QueuedEnvelope {
            envelope,
            enqueued_at: Instant::now(),
            target_hint,
        });
    }

    /// Number of pending envelopes in the pool.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns true if the pool has accumulated enough messages or exceeded max delay.
    pub fn should_flush(&self) -> bool {
        if self.queue.is_empty() {
            return false;
        }
        if self.queue.len() >= self.config.batch_size {
            return true;
        }
        if self
            .queue
            .iter()
            .any(|item| item.enqueued_at.elapsed() >= self.config.max_delay)
        {
            return true;
        }
        false
    }

    /// Flush and process a batch:
    /// 1. Inject cover traffic if needed to reach batch_size.
    /// 2. Shuffles the combined batch using secure RNG.
    /// 3. Returns the mixed batch.
    pub fn flush(&mut self) -> Option<MixedBatch> {
        if self.queue.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let mut items = std::mem::take(&mut self.queue);
        let real_count = items.len();
        let mut cover_count = 0;

        // Pad with cover traffic if enabled and below batch_size
        if self.config.inject_cover_traffic && items.len() < self.config.batch_size {
            let needed = self.config.batch_size - items.len();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            for _ in 0..needed {
                // Generate indistinguishable random cover envelope
                let cover = generate_cover_envelope(now, 3600);
                items.push(QueuedEnvelope {
                    envelope: cover,
                    enqueued_at: Instant::now(),
                    target_hint: None,
                });
                cover_count += 1;
            }
        }

        // Cryptographically shuffle the entire batch to decouple ingress and egress order
        items.shuffle(&mut rng);

        let mut envelopes = Vec::with_capacity(items.len());
        let mut targets = Vec::with_capacity(items.len());

        for item in items {
            envelopes.push(item.envelope);
            targets.push(item.target_hint);
        }

        self.last_flush = Instant::now();

        info!(
            "MixPool flushed batch: {} envelopes ({} real, {} cover injected)",
            envelopes.len(),
            real_count,
            cover_count
        );

        Some(MixedBatch {
            envelopes,
            real_count,
            cover_count,
            targets,
        })
    }

    /// Calculate a randomized jitter delay for dispatching an envelope.
    pub fn sample_jitter_delay(&self) -> Duration {
        let mut rng = rand::thread_rng();
        if self.config.max_jitter_ms <= self.config.min_jitter_ms {
            Duration::from_millis(self.config.min_jitter_ms)
        } else {
            let ms = rng.gen_range(self.config.min_jitter_ms..=self.config.max_jitter_ms);
            Duration::from_millis(ms)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::RoutingHeader;

    fn make_test_envelope(token_byte: u8) -> EncryptedEnvelope {
        let header = RoutingHeader {
            version: 1,
            message_id: [token_byte; 16],
            destination_token: [token_byte; 16],
            created_at: 1000,
            expires_at: 2000,
            hop_count: 0,
        };
        EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![token_byte; 64],
        }
    }

    #[test]
    fn test_mixpool_batch_threshold_and_cover_injection() {
        let config = MixPoolConfig {
            batch_size: 4,
            max_delay: Duration::from_secs(60),
            inject_cover_traffic: true,
            min_jitter_ms: 5,
            max_jitter_ms: 20,
        };
        let mut pool = MixPool::new(config);

        pool.submit(make_test_envelope(0x01), Some("target-1".to_string()));
        pool.submit(make_test_envelope(0x02), Some("target-2".to_string()));

        assert_eq!(pool.len(), 2);
        assert!(!pool.should_flush());

        // Forcing flush pads to batch size 4 with 2 cover envelopes
        let batch = pool.flush().expect("Batch should be produced");
        assert_eq!(batch.envelopes.len(), 4);
        assert_eq!(batch.real_count, 2);
        assert_eq!(batch.cover_count, 2);
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_mixpool_shuffling() {
        let config = MixPoolConfig {
            batch_size: 5,
            max_delay: Duration::from_secs(60),
            inject_cover_traffic: false,
            min_jitter_ms: 1,
            max_jitter_ms: 10,
        };
        let mut pool = MixPool::new(config);

        for i in 1..=5 {
            pool.submit(make_test_envelope(i), None);
        }

        assert!(pool.should_flush());
        let batch = pool.flush().expect("Batch must exist");
        assert_eq!(batch.envelopes.len(), 5);
        assert_eq!(batch.real_count, 5);
        assert_eq!(batch.cover_count, 0);

        // Verify all 5 original envelopes are present in the batch
        let mut tokens: Vec<u8> = batch
            .envelopes
            .iter()
            .map(|e| e.header.destination_token[0])
            .collect();
        tokens.sort();
        assert_eq!(tokens, vec![1, 2, 3, 4, 5]);
    }
}
