//! AMQP Protocol Translation Gateway (Direct and Default Exchange Subset) per `KEI-QUEUE-401` and `KEI-DES-035 §6`.

use crate::gateway_server::ClusterIngress;
use crate::sqs::QueueLeaseProvider;
use keirox_coordinator::EpochFencedToken;
use keirox_core::error::{KeiroxError, Result};
use keirox_core::model::{StreamId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Supported AMQP Exchange Types in the Keirox Phase 4 certified subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmqpExchangeType {
    /// Default direct exchange (routing key equals stream name).
    DefaultDirect,
    /// Direct exchange with explicit routing key binding.
    Direct,
    /// Fanout exchange broadcast.
    Fanout,
    /// Topic exchange (Unsupported in Phase 4 certified subset per ADR-070).
    TopicUnsupported,
    /// Headers exchange (Unsupported in Phase 4 certified subset per ADR-070).
    HeadersUnsupported,
}

/// AMQP basic.publish request message frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpPublishRequest {
    /// Target exchange name (empty string for default direct exchange).
    pub exchange: String,
    /// Routing key determining target micro-stream.
    pub routing_key: String,
    /// Mandatory delivery flag.
    pub mandatory: bool,
    /// Immediate delivery flag.
    pub immediate: bool,
    /// Raw message payload bytes.
    pub payload: Vec<u8>,
    /// Content type MIME descriptor (e.g. "application/json", "application/octet-stream").
    pub content_type: String,
    /// AMQP message application headers.
    pub headers: HashMap<String, String>,
}

/// AMQP basic.publish response confirmation (Publisher Confirms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpPublishConfirmation {
    /// Assigned log offset.
    pub offset: u64,
    /// Destination exchange.
    pub exchange: String,
    /// Destination routing key.
    pub routing_key: String,
}

/// AMQP Gateway Server processing AMQP client frames against Keirox runtime.
pub struct AmqpGatewayServer {
    cluster: Arc<dyn ClusterIngress>,
    lease_provider: Option<Arc<dyn QueueLeaseProvider>>,
    tenant_id: TenantId,
}

impl AmqpGatewayServer {
    /// Create a new AMQP Gateway Server.
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

    /// Map AMQP exchange + routing key to internal deterministic StreamId.
    pub fn resolve_stream(&self, exchange: &str, routing_key: &str) -> Result<StreamId> {
        if exchange == "amq.topic" || exchange.contains(".topic.") {
            return Err(KeiroxError::Internal(
                "AMQP topic exchange topology is unsupported in Phase 4 certified subset per ADR-070".into(),
            ));
        }
        if exchange == "amq.headers" {
            return Err(KeiroxError::Internal(
                "AMQP headers exchange topology is unsupported in Phase 4 certified subset per ADR-070".into(),
            ));
        }

        // Direct & Default Exchange mapping: routing_key maps deterministically to stream
        let stream_name = routing_key;

        let mut hasher = twox_hash::XxHash64::default();
        std::hash::Hasher::write(&mut hasher, stream_name.as_bytes());
        let hash = std::hash::Hasher::finish(&hasher);

        let mut raw = [0u8; 16];
        raw[..8].copy_from_slice(&self.tenant_id.0[..8]);
        raw[8..16].copy_from_slice(&hash.to_le_bytes());
        Ok(StreamId(raw))
    }

    /// Process AMQP basic.publish message frame with publisher confirmation.
    pub async fn basic_publish(&self, req: AmqpPublishRequest) -> Result<AmqpPublishConfirmation> {
        let stream_id = self.resolve_stream(&req.exchange, &req.routing_key)?;

        let offset = self
            .cluster
            .produce(self.tenant_id, stream_id, vec![req.payload])
            .await?;

        Ok(AmqpPublishConfirmation {
            offset,
            exchange: req.exchange,
            routing_key: req.routing_key,
        })
    }

    /// Process AMQP basic.ack frame with epoch fenced token.
    pub async fn basic_ack(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        if let Some(ref provider) = self.lease_provider {
            provider.ack_queue_offset(group_id, token).await?;
        }
        Ok(())
    }

    /// Process AMQP basic.nack frame (requeue to ready state).
    pub async fn basic_nack(&self, group_id: &str, token: EpochFencedToken) -> Result<()> {
        if let Some(ref provider) = self.lease_provider {
            provider.nack_queue_offset(group_id, token).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIngress;
    #[async_trait::async_trait]
    impl ClusterIngress for MockIngress {
        async fn produce(
            &self,
            _tenant_id: TenantId,
            _stream_id: StreamId,
            _records: Vec<Vec<u8>>,
        ) -> Result<u64> {
            Ok(100)
        }
    }

    #[tokio::test]
    async fn test_amqp_direct_publish_and_unsupported_rejection() {
        let tenant = TenantId([0x20; 16]);
        let gateway = AmqpGatewayServer::new(Arc::new(MockIngress), None, tenant);

        // 1. Direct publish succeeds
        let req = AmqpPublishRequest {
            exchange: "".into(),
            routing_key: "tasks.email".into(),
            mandatory: true,
            immediate: false,
            payload: b"send email notification".to_vec(),
            content_type: "text/plain".into(),
            headers: HashMap::new(),
        };

        let confirm = gateway.basic_publish(req).await.unwrap();
        assert_eq!(confirm.offset, 100);
        assert_eq!(confirm.routing_key, "tasks.email");

        // 2. Unsupported topic exchange must return explicit error per ADR-070
        let req_topic = AmqpPublishRequest {
            exchange: "amq.topic".into(),
            routing_key: "orders.europe.*".into(),
            payload: b"data".to_vec(),
            content_type: "application/json".into(),
            headers: HashMap::new(),
            mandatory: false,
            immediate: false,
        };

        let err = gateway.basic_publish(req_topic).await;
        assert!(err.is_err());
    }
}
