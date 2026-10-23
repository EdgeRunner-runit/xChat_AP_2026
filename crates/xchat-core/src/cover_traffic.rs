use rand::{thread_rng, Rng};
use xchat_crypto::padding::pad_message;

use crate::envelope::{EncryptedEnvelope, RoutingHeader};

/// Generates dummy padded envelopes for cover traffic to disguise user presence and timing patterns.
pub fn generate_cover_envelope(now: u64, ttl_seconds: u64) -> EncryptedEnvelope {
    let mut rng = thread_rng();
    let mut dummy_token = [0u8; 16];
    rng.fill(&mut dummy_token);

    let header = RoutingHeader::new(dummy_token, now, ttl_seconds);

    // Random noise payload padded to standardized 256 bytes
    let mut noise = vec![0u8; 128];
    rng.fill(&mut noise[..]);
    let padded = pad_message(&noise);

    EncryptedEnvelope {
        header,
        ratchet_header: None,
        x3dh_init: None,
        ciphertext: padded,
    }
}

/// Calculate randomized jitter delay (in milliseconds) to prevent periodic burst correlation.
pub fn calculate_jitter_delay_ms(min_ms: u64, max_ms: u64) -> u64 {
    let mut rng = thread_rng();
    rng.gen_range(min_ms..=max_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_traffic_envelope() {
        let env = generate_cover_envelope(1000, 3600);
        assert_eq!(env.header.created_at, 1000);
        assert!(!env.ciphertext.is_empty());

        let jitter = calculate_jitter_delay_ms(50, 200);
        assert!((50..=200).contains(&jitter));
    }
}
