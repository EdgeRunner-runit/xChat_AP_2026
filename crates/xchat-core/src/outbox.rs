use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    envelope::{EncryptedEnvelope, MessageId},
    errors::CoreError,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct QueuedMessage {
    pub envelope: EncryptedEnvelope,
    pub attempts: u32,
    pub last_attempt_at: u64,
}

/// Store-and-forward outbox queue.
/// Messages are enqueued as OPAQUE CIPHERTEXT ONLY and persisted to disk if needed.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoreAndForwardQueue {
    queue: Vec<QueuedMessage>,
}

impl StoreAndForwardQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Enqueue an encrypted envelope for transmission across any available transport.
    pub fn enqueue(&mut self, envelope: EncryptedEnvelope) {
        // Prevent duplicate queuing of same message ID
        if !self
            .queue
            .iter()
            .any(|q| q.envelope.header.message_id == envelope.header.message_id)
        {
            self.queue.push(QueuedMessage {
                envelope,
                attempts: 0,
                last_attempt_at: 0,
            });
        }
    }

    /// Retrieve envelopes ready for transmission attempt.
    pub fn peek_due(&mut self, now: u64, retry_backoff_secs: u64) -> Vec<EncryptedEnvelope> {
        let mut ready = Vec::new();
        for item in &mut self.queue {
            if !item.envelope.is_expired(now)
                && (item.last_attempt_at == 0
                    || now.saturating_sub(item.last_attempt_at) >= retry_backoff_secs)
            {
                item.attempts = item.attempts.saturating_add(1);
                item.last_attempt_at = now;
                ready.push(item.envelope.clone());
            }
        }
        ready
    }

    /// Acknowledge delivery or receipt of a message, removing it from outbox.
    pub fn mark_delivered(&mut self, message_id: &MessageId) {
        self.queue
            .retain(|item| item.envelope.header.message_id != *message_id);
    }

    /// Remove expired envelopes according to cryptographic TTL.
    pub fn prune_expired(&mut self, now: u64) -> usize {
        let before = self.queue.len();
        self.queue.retain(|item| !item.envelope.is_expired(now));
        before - self.queue.len()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Persist the encrypted queue to a disk file.
    pub fn save_to_disk(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::StorageError(e.to_string()))?;
        }
        let data = bincode::serialize(self).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut file = File::create(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        file.write_all(&data)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Restore the encrypted queue from a disk file.
    pub fn load_from_disk(path: &Path) -> Result<Self, CoreError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let mut file = File::open(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        bincode::deserialize(&buffer).map_err(|e| CoreError::StorageError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::RoutingHeader;
    use tempfile::NamedTempFile;

    #[test]
    fn test_outbox_queue_lifecycle() {
        let mut outbox = StoreAndForwardQueue::new();
        let header = RoutingHeader::new([0x11; 16], 1000, 3600);
        let msg_id = header.message_id;

        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![9, 8, 7],
        };

        outbox.enqueue(env);
        assert_eq!(outbox.len(), 1);

        // First peek due
        let due = outbox.peek_due(1001, 10);
        assert_eq!(due.len(), 1);

        // Immediate second peek should return empty due to backoff
        let due2 = outbox.peek_due(1005, 10);
        assert_eq!(due2.len(), 0);

        // Peek after backoff succeeds
        let due3 = outbox.peek_due(1015, 10);
        assert_eq!(due3.len(), 1);

        // Mark delivered removes from queue
        outbox.mark_delivered(&msg_id);
        assert_eq!(outbox.len(), 0);
    }

    #[test]
    fn test_outbox_disk_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut outbox = StoreAndForwardQueue::new();
        let header = RoutingHeader::new([0x22; 16], 2000, 3600);
        let env = EncryptedEnvelope {
            header,
            ratchet_header: None,
            x3dh_init: None,
            ciphertext: vec![1, 2, 3],
        };

        outbox.enqueue(env);
        outbox.save_to_disk(path).unwrap();

        let loaded = StoreAndForwardQueue::load_from_disk(path).unwrap();
        assert_eq!(loaded.len(), 1);
    }
}
