use std::collections::HashMap;

use crate::{envelope::MessageId, errors::CoreError};

pub const DEFAULT_REPLAY_WINDOW_SECS: u64 = 86400 * 2; // 48 hours

/// Anti-replay detection filter tracking observed message identifiers.
#[derive(Debug, Default)]
pub struct ReplayFilter {
    seen_messages: HashMap<MessageId, u64>,
}

impl ReplayFilter {
    pub fn new() -> Self {
        Self {
            seen_messages: HashMap::new(),
        }
    }

    /// Check if a message ID has already been processed. If not seen, records it.
    pub fn check_and_record(
        &mut self,
        message_id: &MessageId,
        current_time: u64,
    ) -> Result<(), CoreError> {
        if self.seen_messages.contains_key(message_id) {
            return Err(CoreError::ReplayDetected(hex::encode(message_id)));
        }

        self.seen_messages.insert(*message_id, current_time);
        Ok(())
    }

    /// Prune old message records older than the replay window.
    pub fn prune(&mut self, current_time: u64, max_age_seconds: u64) {
        let cutoff = current_time.saturating_sub(max_age_seconds);
        self.seen_messages
            .retain(|_, &mut seen_at| seen_at >= cutoff);
    }

    pub fn size(&self) -> usize {
        self.seen_messages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_protection() {
        let mut filter = ReplayFilter::new();
        let msg_id = [0x42; 16];

        // First arrival succeeds
        assert!(filter.check_and_record(&msg_id, 1000).is_ok());

        // Replay of same message ID must fail
        assert!(filter.check_and_record(&msg_id, 1005).is_err());

        // Different message ID succeeds
        let msg_id_2 = [0x43; 16];
        assert!(filter.check_and_record(&msg_id_2, 1010).is_ok());
    }

    #[test]
    fn test_replay_pruning() {
        let mut filter = ReplayFilter::new();
        let old_msg = [0x01; 16];
        let new_msg = [0x02; 16];

        filter.check_and_record(&old_msg, 100).unwrap();
        filter.check_and_record(&new_msg, 1000).unwrap();

        // Prune entries older than 500 seconds from t=1000 (cutoff = 500)
        filter.prune(1000, 500);

        assert_eq!(filter.size(), 1);
        // Old message was pruned, new message remains
        assert!(filter.check_and_record(&new_msg, 1010).is_err());
    }
}
