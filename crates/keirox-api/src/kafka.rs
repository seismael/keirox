//! Kafka Wire Protocol framing and API key taxonomy per `KEI-DES-032` §4.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use keirox_core::error::{KeiroxError, Result};

/// Canonical Kafka API keys supported by the Keirox Kafka Gateway per `KEI-DES-032` §4.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum KafkaApiKey {
    /// Produce request (API key 0).
    Produce = 0,
    /// Fetch request (API key 1).
    Fetch = 1,
    /// ListOffsets request (API key 2).
    ListOffsets = 2,
    /// Metadata request (API key 3).
    Metadata = 3,
    /// OffsetCommit request (API key 8).
    OffsetCommit = 8,
    /// OffsetFetch request (API key 9).
    OffsetFetch = 9,
    /// JoinGroup request (API key 11).
    JoinGroup = 11,
    /// Heartbeat request (API key 12).
    Heartbeat = 12,
    /// LeaveGroup request (API key 13).
    LeaveGroup = 13,
    /// SyncGroup request (API key 14).
    SyncGroup = 14,
}

impl KafkaApiKey {
    /// Parse raw API key integer into enum.
    pub fn from_u16(key: u16) -> Option<Self> {
        match key {
            0 => Some(Self::Produce),
            1 => Some(Self::Fetch),
            2 => Some(Self::ListOffsets),
            3 => Some(Self::Metadata),
            8 => Some(Self::OffsetCommit),
            9 => Some(Self::OffsetFetch),
            11 => Some(Self::JoinGroup),
            12 => Some(Self::Heartbeat),
            13 => Some(Self::LeaveGroup),
            14 => Some(Self::SyncGroup),
            _ => None,
        }
    }
}

/// Standard Kafka Request Header (v1/v2) containing routing metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaRequestHeader {
    /// Target Kafka API key.
    pub api_key: KafkaApiKey,
    /// Protocol API version requested by client.
    pub api_version: u16,
    /// Client-generated correlation identifier.
    pub correlation_id: u32,
    /// Client identifier string.
    pub client_id: Option<String>,
}

impl KafkaRequestHeader {
    /// Decode a Kafka request header from binary frame bytes.
    pub fn decode(buf: &mut impl Buf) -> Result<Self> {
        if buf.remaining() < 8 {
            return Err(KeiroxError::Internal(
                "Buffer too short for Kafka request header".into(),
            ));
        }

        let api_key_raw = buf.get_u16();
        let api_key = KafkaApiKey::from_u16(api_key_raw).ok_or_else(|| {
            KeiroxError::Internal(format!("Unsupported Kafka API key: {api_key_raw}"))
        })?;

        let api_version = buf.get_u16();
        let correlation_id = buf.get_u32();

        let client_id = if buf.remaining() >= 2 {
            let len = buf.get_i16();
            if len > 0 && buf.remaining() >= len as usize {
                let mut str_bytes = vec![0u8; len as usize];
                buf.copy_to_slice(&mut str_bytes);
                String::from_utf8(str_bytes).ok()
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

    /// Encode Kafka request header to bytes.
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();
        buf.put_u16(self.api_key as u16);
        buf.put_u16(self.api_version);
        buf.put_u32(self.correlation_id);

        if let Some(ref id) = self.client_id {
            buf.put_i16(id.len() as i16);
            buf.put_slice(id.as_bytes());
        } else {
            buf.put_i16(-1);
        }

        buf.freeze()
    }
}

/// Standard Kafka Response Header containing correlation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KafkaResponseHeader {
    /// Corresponds to the client request correlation ID.
    pub correlation_id: u32,
}

impl KafkaResponseHeader {
    /// Encode response header to binary bytes.
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(4);
        buf.put_u32(self.correlation_id);
        buf.freeze()
    }

    /// Decode response header from binary bytes.
    pub fn decode(buf: &mut impl Buf) -> Result<Self> {
        if buf.remaining() < 4 {
            return Err(KeiroxError::Internal(
                "Buffer too short for Kafka response header".into(),
            ));
        }
        Ok(Self {
            correlation_id: buf.get_u32(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_api_key_parsing() {
        assert_eq!(KafkaApiKey::from_u16(0), Some(KafkaApiKey::Produce));
        assert_eq!(KafkaApiKey::from_u16(1), Some(KafkaApiKey::Fetch));
        assert_eq!(KafkaApiKey::from_u16(999), None);
    }

    #[test]
    fn test_kafka_request_header_encode_decode() {
        let header = KafkaRequestHeader {
            api_key: KafkaApiKey::Produce,
            api_version: 9,
            correlation_id: 12345,
            client_id: Some("keirox-java-client".into()),
        };

        let encoded = header.encode();
        let mut slice = encoded.as_ref();
        let decoded = KafkaRequestHeader::decode(&mut slice).unwrap();

        assert_eq!(decoded.api_key, KafkaApiKey::Produce);
        assert_eq!(decoded.api_version, 9);
        assert_eq!(decoded.correlation_id, 12345);
        assert_eq!(decoded.client_id, Some("keirox-java-client".into()));
    }

    #[test]
    fn test_kafka_response_header_encode_decode() {
        let header = KafkaResponseHeader {
            correlation_id: 998877,
        };
        let encoded = header.encode();
        let mut slice = encoded.as_ref();
        let decoded = KafkaResponseHeader::decode(&mut slice).unwrap();
        assert_eq!(decoded.correlation_id, 998877);
    }
}
