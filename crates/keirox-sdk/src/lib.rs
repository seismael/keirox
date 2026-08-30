//! # Keirox Native Client SDK
//!
//! High-performance native client library providing producer, consumer, queue worker,
//! and Arrow Flight vectorized reader interfaces per `KEI-ARC-023` and `KEI-DES-032`.

#![deny(missing_docs)]

pub mod client;
pub mod consumer;
pub mod flight;
pub mod producer;
pub mod task_queue;

pub use client::{ClusterClientTransport, KeiroxClient, KeiroxClientConfig};
pub use consumer::{KeiroxConsumer, RecordEnvelope};
pub use flight::ArrowFlightReader;
pub use producer::KeiroxProducer;
pub use task_queue::KeiroxQueueClient;
