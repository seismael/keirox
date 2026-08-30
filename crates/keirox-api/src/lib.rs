//! # Keirox API
//!
//! Client RPC protocols and wire translations per `KEI-DES-032`.

#![deny(missing_docs)]

/// Protocol definitions and request/response messages.
pub mod proto;

pub use proto::{
    AckMode, AcknowledgeRequest, LeaseRecordsRequest, ProduceBatchRequest, ProduceBatchResponse,
};
