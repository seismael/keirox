//! Continuous streaming consumer client with offset tracking per `KEI-DES-032` §6.

use crate::client::KeiroxClient;
use keirox_core::model::StreamId;

/// Stream record envelope with logical offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordEnvelope {
    /// Logical stream offset.
    pub offset: u64,
    /// Record payload bytes.
    pub payload: Vec<u8>,
}

/// Continuous sequential consumer client.
pub struct KeiroxConsumer {
    client: KeiroxClient,
    stream_id: StreamId,
    current_offset: u64,
}

impl KeiroxConsumer {
    /// Create a new consumer starting at `start_offset`.
    #[must_use]
    pub fn new(client: KeiroxClient, stream_id: StreamId, start_offset: u64) -> Self {
        Self {
            client,
            stream_id,
            current_offset: start_offset,
        }
    }

    /// Client reference.
    #[must_use]
    pub fn client(&self) -> &KeiroxClient {
        &self.client
    }

    /// Current consumer read cursor position.
    #[must_use]
    pub fn position(&self) -> u64 {
        self.current_offset
    }

    /// Seek consumer cursor to a specific offset.
    pub fn seek(&mut self, offset: u64) {
        self.current_offset = offset;
    }

    /// Target stream identifier.
    #[must_use]
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}
