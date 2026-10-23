use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::CoreError;

pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunk size

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FileChunk {
    pub file_id: [u8; 16],
    pub file_name: String,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub chunk_hash: [u8; 32],
    pub data: Vec<u8>,
}

/// Chunks an arbitrary file or voice recording into 64KB blocks for delay-tolerant mesh transmission.
pub fn chunk_payload(file_id: [u8; 16], file_name: &str, payload: &[u8]) -> Vec<FileChunk> {
    let total_chunks = payload.len().div_ceil(CHUNK_SIZE).max(1) as u32;
    let mut chunks = Vec::with_capacity(total_chunks as usize);

    for (idx, slice) in payload.chunks(CHUNK_SIZE).enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(slice);
        let chunk_hash: [u8; 32] = hasher.finalize().into();

        chunks.push(FileChunk {
            file_id,
            file_name: file_name.to_string(),
            chunk_index: idx as u32,
            total_chunks,
            chunk_hash,
            data: slice.to_vec(),
        });
    }

    chunks
}

/// Collects and reassembles chunks received out of order or across multiple transports.
#[derive(Debug, Default)]
pub struct ChunkReassembler {
    pending_files: HashMap<[u8; 16], HashMap<u32, FileChunk>>,
}

impl ChunkReassembler {
    pub fn new() -> Self {
        Self {
            pending_files: HashMap::new(),
        }
    }

    /// Ingest an inbound chunk. If this chunk completes the file, returns the reconstructed byte buffer.
    pub fn ingest_chunk(
        &mut self,
        chunk: FileChunk,
    ) -> Result<Option<(String, Vec<u8>)>, CoreError> {
        // Verify chunk hash integrity
        let mut hasher = Sha256::new();
        hasher.update(&chunk.data);
        let actual_hash: [u8; 32] = hasher.finalize().into();
        if actual_hash != chunk.chunk_hash {
            return Err(CoreError::EnvelopeError(
                "Corrupted chunk hash verification failed".into(),
            ));
        }

        let file_id = chunk.file_id;
        let total = chunk.total_chunks;
        let file_name = chunk.file_name.clone();

        let chunks_map = self.pending_files.entry(file_id).or_default();
        chunks_map.insert(chunk.chunk_index, chunk);

        if chunks_map.len() == total as usize {
            // All chunks received! Reassemble in order.
            let mut reassembled = Vec::new();
            for i in 0..total {
                let piece = chunks_map.get(&i).ok_or_else(|| {
                    CoreError::EnvelopeError("Missing chunk during reassembly".into())
                })?;
                reassembled.extend_from_slice(&piece.data);
            }
            self.pending_files.remove(&file_id);
            Ok(Some((file_name, reassembled)))
        } else {
            Ok(None)
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_chunking_and_reassembly() {
        let file_id = [0x55; 16];
        let original_data = vec![0x42u8; 150 * 1024]; // 150 KB -> 3 chunks (64KB, 64KB, 22KB)

        let chunks = chunk_payload(file_id, "mission_map.png", &original_data);
        assert_eq!(chunks.len(), 3);

        let mut reassembler = ChunkReassembler::new();

        // Ingest chunks out-of-order: 2, 0, 1
        assert_eq!(reassembler.ingest_chunk(chunks[2].clone()).unwrap(), None);
        assert_eq!(reassembler.ingest_chunk(chunks[0].clone()).unwrap(), None);

        let completed = reassembler
            .ingest_chunk(chunks[1].clone())
            .unwrap()
            .expect("file completed");

        assert_eq!(completed.0, "mission_map.png");
        assert_eq!(completed.1, original_data);
        assert_eq!(reassembler.pending_count(), 0);
    }
}
