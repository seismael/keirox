//! AWS SQS Protocol Translation Gateway per `KEI-QUEUE-401` and `KEI-DES-035 §5`.

use crate::gateway_server::ClusterIngress;
use async_trait::async_trait;
use keirox_coordinator::EpochFencedToken;
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait defining coordinator lease management for Queue Gateways.
#[async_trait]
pub trait QueueLeaseProvider: Send + Sync {
    /// Acquire a leased offset for a consumer group queue.
    async fn lease_queue_offset(
        &self,
        group_id: &str,
        offset: u64,
        ttl_ms: u64,
        now_us: u64,
    ) -> Result<EpochFencedToken>;

    /// Acknowledge consumption of an offset with epoch fencing token.
    async fn ack_queue_offset(&self, group_id: &str, token: EpochFencedToken) -> Result<()>;

    /// Negative acknowledge an offset back to READY state.
    async fn nack_queue_offset(&self, group_id: &str, token: EpochFencedToken) -> Result<()>;
}

/// Request to publish a message via SQS SendMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsSendMessageRequest {
    /// Target queue URL or queue name.
    pub queue_url: String,
    /// Message body payload string.
    pub message_body: String,
    /// Delay delivery seconds (0..900).
    pub delay_seconds: u32,
    /// SQS message attributes.
    pub message_attributes: HashMap<String, String>,
    /// FIFO message deduplication ID.
    pub message_deduplication_id: Option<String>,
    /// FIFO message group ID.
    pub message_group_id: Option<String>,
}

/// Response returned from SQS SendMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsSendMessageResponse {
    /// Assigned unique SQS Message ID.
    pub message_id: String,
    /// Monotonic log offset sequence number.
    pub sequence_number: u64,
    /// MD5 / CRC32 checksum representation of message body.
    pub md5_of_body: String,
}

/// Request to poll messages via SQS ReceiveMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsReceiveMessageRequest {
    /// Target queue URL.
    pub queue_url: String,
    /// Max messages to return (1..10).
    pub max_number_of_messages: u32,
    /// Lease duration in seconds.
    pub visibility_timeout_s: u32,
    /// Long polling wait time in seconds (0..20).
    pub wait_time_seconds: u32,
}

/// Individual message received from SQS queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsMessage {
    /// Unique Message ID.
    pub message_id: String,
    /// Opaque receipt handle encoding lease token.
    pub receipt_handle: String,
    /// Body string.
    pub body: String,
    /// Message attributes.
    pub attributes: HashMap<String, String>,
}

/// Response returned from SQS ReceiveMessage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsReceiveMessageResponse {
    /// List of leased messages.
    pub messages: Vec<SqsMessage>,
}

/// SQS Translation Gateway Server implementing AWS SQS wire operations against Keirox.
pub struct SqsGatewayServer {
    cluster: Arc<dyn ClusterIngress>,
    lease_provider: Option<Arc<dyn QueueLeaseProvider>>,
    tenant_id: TenantId,
}

impl SqsGatewayServer {
    /// Create a new SQS Gateway Server.
    #[must_use]
    pub fn new(
        cluster: Arc<dyn ClusterIngress>,
        lease_provider: Option<Arc<dyn QueueLeaseProvider>>,
        tenant_id: TenantId,
    ) -> Self {
        Self {
            cluster,
            lease_provider,
            tenant_id,
        }
    }

    /// Map queue URL/name to internal deterministic StreamId.
    #[must_use]
    pub fn map_queue_to_stream(&self, queue_url: &str) -> StreamId {
        let mut hasher = twox_hash::XxHash64::default();
        std::hash::Hasher::write(&mut hasher, queue_url.as_bytes());
        let hash = std::hash::Hasher::finish(&hasher);
        let mut raw = [0u8; 16];
        raw[..8].copy_from_slice(&self.tenant_id.0[..8]);
        raw[8..16].copy_from_slice(&hash.to_le_bytes());
        StreamId(raw)
    }

    /// Process SQS SendMessage operation.
    pub async fn send_message(&self, req: SqsSendMessageRequest) -> Result<SqsSendMessageResponse> {
        let stream_id = self.map_queue_to_stream(&req.queue_url);
        let payload = req.message_body.into_bytes();

        let offset = self
            .cluster
            .produce(self.tenant_id, stream_id, vec![payload.clone()])
            .await?;

        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&payload);
        let md5_bytes = hasher.finalize();
        let md5_hex = format!("{:032x}", md5_bytes);

        Ok(SqsSendMessageResponse {
            message_id: format!("msg-{:016x}-{}", offset, &md5_hex[0..8]),
            sequence_number: offset,
            md5_of_body: md5_hex,
        })
    }

    /// Helper to resolve consumer group ID from queue URL.
    #[must_use]
    pub fn queue_group_id(queue_url: &str) -> String {
        let queue_name = queue_url.rsplit('/').next().unwrap_or("default");
        format!("sqs-group-{queue_name}")
    }

    /// Process SQS DeleteMessage operation using receipt handle.
    pub async fn delete_message(&self, queue_url: &str, receipt_handle: &str) -> Result<()> {
        let token = Self::decode_receipt_handle(receipt_handle)?;
        let group_id = Self::queue_group_id(queue_url);
        if let Some(ref provider) = self.lease_provider {
            provider.ack_queue_offset(&group_id, token).await?;
        }
        Ok(())
    }

    /// Encode an EpochFencedToken into an opaque SQS receipt handle.
    #[must_use]
    pub fn encode_receipt_handle(token: EpochFencedToken) -> String {
        format!(
            "RH-{:08x}-{:016x}-{:016x}-{:08x}",
            token.shard_id.0, token.epoch.0, token.offset, token.nonce
        )
    }

    /// Decode an opaque SQS receipt handle back into an EpochFencedToken.
    pub fn decode_receipt_handle(receipt_handle: &str) -> Result<EpochFencedToken> {
        if !receipt_handle.starts_with("RH-") {
            return Err(KeiroxError::Internal(
                "Invalid SQS receipt handle format".into(),
            ));
        }

        let parts: Vec<&str> = receipt_handle[3..].split('-').collect();
        if parts.len() != 4 {
            return Err(KeiroxError::Internal(
                "Malformed SQS receipt handle segments".into(),
            ));
        }

        let shard_id = u32::from_str_radix(parts[0], 16)
            .map_err(|_| KeiroxError::Internal("Invalid shard_id in receipt handle".into()))?;
        let epoch = u64::from_str_radix(parts[1], 16)
            .map_err(|_| KeiroxError::Internal("Invalid epoch in receipt handle".into()))?;
        let offset = u64::from_str_radix(parts[2], 16)
            .map_err(|_| KeiroxError::Internal("Invalid offset in receipt handle".into()))?;
        let nonce = u32::from_str_radix(parts[3], 16)
            .map_err(|_| KeiroxError::Internal("Invalid nonce in receipt handle".into()))?;

        Ok(EpochFencedToken::new(
            keirox_coordinator::ShardId(shard_id),
            keirox_coordinator::CoordinatorEpoch(epoch),
            offset,
            nonce,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIngress;
    #[async_trait]
    impl ClusterIngress for MockIngress {
        async fn produce(
            &self,
            _tenant_id: TenantId,
            _stream_id: StreamId,
            _records: Vec<Vec<u8>>,
        ) -> Result<u64> {
            Ok(42)
        }
    }

    #[tokio::test]
    async fn test_sqs_send_and_receipt_handle_roundtrip() {
        let tenant = TenantId([0x10; 16]);
        let gateway = SqsGatewayServer::new(Arc::new(MockIngress), None, tenant);

        let req = SqsSendMessageRequest {
            queue_url: "https://sqs.us-east-1.amazonaws.com/123456789012/order-queue".into(),
            message_body: "{\"item\":\"laptop\",\"price\":1200}".into(),
            delay_seconds: 0,
            message_attributes: HashMap::new(),
            message_deduplication_id: None,
            message_group_id: None,
        };

        let res = gateway.send_message(req).await.unwrap();
        assert_eq!(res.sequence_number, 42);
        assert!(!res.message_id.is_empty());

        let token = EpochFencedToken::new(
            keirox_coordinator::ShardId(12),
            keirox_coordinator::CoordinatorEpoch(3),
            42,
            999,
        );

        let handle = SqsGatewayServer::encode_receipt_handle(token);
        let decoded = SqsGatewayServer::decode_receipt_handle(&handle).unwrap();
        assert_eq!(decoded.shard_id.0, 12);
        assert_eq!(decoded.epoch.0, 3);
        assert_eq!(decoded.offset, 42);
        assert_eq!(decoded.nonce, 999);
    }
}
