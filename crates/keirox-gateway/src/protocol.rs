//! Kafka binary wire-protocol frames, API key definitions, and error mappings per `KEI-DES-035`.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use keirox_core::error::{KeiroxError, Result};
use std::collections::HashMap;

/// Standard Kafka API Keys supported in the certified gateway subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i16)]
pub enum KafkaApiKey {
    /// Produce messages (Key 0).
    Produce = 0,
    /// Fetch messages (Key 1).
    Fetch = 1,
    /// List partition offsets (Key 2).
    ListOffsets = 2,
    /// Metadata discovery (Key 3).
    Metadata = 3,
    /// Offset commit (Key 8).
    OffsetCommit = 8,
    /// Offset fetch (Key 9).
    OffsetFetch = 9,
    /// Find coordinator (Key 10).
    FindCoordinator = 10,
    /// API versions negotiation (Key 18).
    ApiVersions = 18,
    /// Initialize idempotent producer ID (Key 22).
    InitProducerId = 22,
    /// Unsupported API key.
    Unsupported(i16),
}

impl From<i16> for KafkaApiKey {
    fn from(val: i16) -> Self {
        match val {
            0 => Self::Produce,
            1 => Self::Fetch,
            2 => Self::ListOffsets,
            3 => Self::Metadata,
            8 => Self::OffsetCommit,
            9 => Self::OffsetFetch,
            10 => Self::FindCoordinator,
            18 => Self::ApiVersions,
            22 => Self::InitProducerId,
            other => Self::Unsupported(other),
        }
    }
}

/// Standard Kafka Error Codes mapped to Keirox error states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum KafkaErrorCode {
    /// No error (Success).
    None = 0,
    /// Topic or partition does not exist.
    UnknownTopicOrPartition = 3,
    /// Leader not available.
    LeaderNotAvailable = 5,
    /// Not leader or follower.
    NotLeaderOrFollower = 6,
    /// Request timed out.
    RequestTimedOut = 7,
    /// Invalid required acks parameter.
    InvalidRequiredAcks = 21,
    /// Unsupported API version or operation.
    UnsupportedVersion = 35,
    /// Duplicate sequence number for idempotent produce.
    DuplicateSequenceNumber = 46,
    /// Out of order sequence number for idempotent produce.
    OutOfOrderSequenceNumber = 47,
    /// General broker server failure.
    UnknownServerError = -1,
}

impl KafkaErrorCode {
    /// Convert error code to standard i16 wire representation.
    #[must_use]
    pub fn to_i16(self) -> i16 {
        self as i16
    }
}

/// Kafka request header (v1/v2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaRequestHeader {
    /// API key.
    pub api_key: KafkaApiKey,
    /// API version.
    pub api_version: i16,
    /// Correlation ID.
    pub correlation_id: i32,
    /// Client identifier.
    pub client_id: Option<String>,
}

impl KafkaRequestHeader {
    /// Decode a request header from a binary buffer.
    pub fn decode(buf: &mut Bytes) -> Result<Self> {
        if buf.remaining() < 8 {
            return Err(KeiroxError::Internal("Header buffer underflow".into()));
        }

        let api_key = KafkaApiKey::from(buf.get_i16());
        let api_version = buf.get_i16();
        let correlation_id = buf.get_i32();

        let client_id = if buf.remaining() >= 2 {
            let len = buf.get_i16();
            if len > 0 {
                if buf.remaining() < len as usize {
                    return Err(KeiroxError::Internal(
                        "Client ID buffer underflow in Kafka request header".into(),
                    ));
                }
                let mut str_bytes = vec![0u8; len as usize];
                buf.copy_to_slice(&mut str_bytes);
                Some(String::from_utf8(str_bytes).map_err(|_| {
                    KeiroxError::Internal("Malformed UTF-8 in Kafka client_id".into())
                })?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            api_key,
            api_version,
            correlation_id,
            client_id,
        })
    }
}

/// Kafka response header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaResponseHeader {
    /// Correlation ID echoing request.
    pub correlation_id: i32,
}

impl KafkaResponseHeader {
    /// Encode response header into buffer.
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_i32(self.correlation_id);
    }
}

/// Produce request record partition batch.
#[derive(Debug, Clone)]
pub struct KafkaProduceRecordBatch {
    /// Topic name.
    pub topic: String,
    /// Partition index.
    pub partition: i32,
    /// Idempotent producer ID (if enabled).
    pub producer_id: i64,
    /// Idempotent producer epoch.
    pub producer_epoch: i16,
    /// Base sequence number.
    pub base_sequence: i32,
    /// Raw payload records.
    pub records: Vec<Vec<u8>>,
}

/// Topic partition offset result.
#[derive(Debug, Clone)]
pub struct KafkaPartitionResponse {
    /// Partition index.
    pub partition: i32,
    /// Error code.
    pub error_code: KafkaErrorCode,
    /// Base offset assigned by Keirox.
    pub base_offset: i64,
    /// Append timestamp in milliseconds.
    pub log_append_time_ms: i64,
}

/// Kafka Produce Response.
#[derive(Debug, Clone)]
pub struct KafkaProduceResponse {
    /// Map of topic -> partition responses.
    pub responses: HashMap<String, Vec<KafkaPartitionResponse>>,
    /// Throttle time in milliseconds.
    pub throttle_time_ms: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_header_decode() {
        let mut buf = BytesMut::new();
        buf.put_i16(18); // ApiVersions
        buf.put_i16(3); // v3
        buf.put_i32(42); // Correlation ID
        buf.put_i16(6); // client_id len
        buf.put_slice(b"client");

        let mut bytes = buf.freeze();
        let header = KafkaRequestHeader::decode(&mut bytes).unwrap();
        assert_eq!(header.api_key, KafkaApiKey::ApiVersions);
        assert_eq!(header.api_version, 3);
        assert_eq!(header.correlation_id, 42);
        assert_eq!(header.client_id, Some("client".into()));
    }
}
