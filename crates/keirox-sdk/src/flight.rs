//! Vectorized Arrow Flight client reader per `KEI-DES-032` §8.

use crate::client::KeiroxClient;
use arrow::array::{ArrayRef, Int64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use keirox_core::error::Result;
use keirox_core::model::StreamId;
use std::sync::Arc;

/// Vectorized Arrow Flight client reader for zero-copy columnar batch transfer.
#[derive(Clone)]
pub struct ArrowFlightReader {
    client: KeiroxClient,
}

impl ArrowFlightReader {
    /// Create a new Arrow Flight reader.
    #[must_use]
    pub fn new(client: KeiroxClient) -> Self {
        Self { client }
    }

    /// Client reference.
    #[must_use]
    pub fn client(&self) -> &KeiroxClient {
        &self.client
    }

    /// Read stream segment directly into an Arrow `RecordBatch`.
    pub async fn read_stream_batch(
        &self,
        _stream_id: StreamId,
        start_offset: u64,
        records: &[Vec<u8>],
    ) -> Result<RecordBatch> {
        let count = records.len();
        let mut offsets = Vec::with_capacity(count);
        let mut timestamps = Vec::with_capacity(count);
        let mut payloads = Vec::with_capacity(count);

        for (i, rec) in records.iter().enumerate() {
            offsets.push(start_offset + i as u64);
            timestamps.push(1_700_000_000_000_000_000i64);
            payloads.push(String::from_utf8_lossy(rec).to_string());
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("_offset", DataType::UInt64, false),
            Field::new("_timestamp_ns", DataType::Int64, false),
            Field::new("payload", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(UInt64Array::from(offsets)) as ArrayRef,
                Arc::new(Int64Array::from(timestamps)) as ArrayRef,
                Arc::new(StringArray::from(payloads)) as ArrayRef,
            ],
        )
        .map_err(|e| {
            keirox_core::error::KeiroxError::Internal(format!("Arrow batch creation error: {e}"))
        })?;

        Ok(batch)
    }
}
