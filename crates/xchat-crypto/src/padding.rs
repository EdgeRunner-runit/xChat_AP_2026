use crate::errors::CryptoError;

pub const MIN_BUCKET: usize = 64;
pub const BUCKET_TIERS: &[usize] = &[
    64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
];

/// Calculate the padded target size for a given input length.
pub fn target_bucket_size(raw_len: usize) -> usize {
    // 4 bytes for length prefix
    let total_len = raw_len.saturating_add(4);
    for &tier in BUCKET_TIERS {
        if total_len <= tier {
            return tier;
        }
    }
    // Beyond 64KB, round up to next 64KB block
    let block = 65536;
    total_len.div_ceil(block) * block
}

/// Apply bucketed padding to plaintext to obscure true message lengths.
pub fn pad_message(payload: &[u8]) -> Vec<u8> {
    let raw_len = payload.len();
    let target_size = target_bucket_size(raw_len);
    let mut out = Vec::with_capacity(target_size);

    // 4-byte big endian length prefix
    out.extend_from_slice(&(raw_len as u32).to_be_bytes());
    out.extend_from_slice(payload);

    // Pad with zeroes up to target size
    out.resize(target_size, 0u8);
    out
}

/// Unpad a message and extract the original payload.
pub fn unpad_message(padded: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if padded.len() < 4 {
        return Err(CryptoError::PaddingError);
    }

    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&padded[..4]);
    let raw_len = u32::from_be_bytes(len_bytes) as usize;

    if padded.len() < 4 + raw_len {
        return Err(CryptoError::PaddingError);
    }

    let payload = padded[4..4 + raw_len].to_vec();
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding_bucket_sizes() {
        assert_eq!(target_bucket_size(10), 64);
        assert_eq!(target_bucket_size(60), 64); // 60 + 4 = 64
        assert_eq!(target_bucket_size(61), 128); // 61 + 4 = 65 -> 128
        assert_eq!(target_bucket_size(250), 256);
        assert_eq!(target_bucket_size(255), 512);
    }

    #[test]
    fn test_pad_unpad_roundtrip() {
        let original = b"Hello xChat! This message is secret.";
        let padded = pad_message(original);
        assert_eq!(padded.len(), 64);

        let restored = unpad_message(&padded).expect("unpadding succeeds");
        assert_eq!(restored, original);
    }
}
