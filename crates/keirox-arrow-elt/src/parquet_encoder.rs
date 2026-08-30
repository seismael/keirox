//! Parquet file encoding and serialization for shredded Arrow batches per `KEI-DES-034`.

use arrow::record_batch::RecordBatch;
use keirox_core::error::{KeiroxError, Result};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::Path;

/// High-performance Parquet file writer for vectorized Arrow record batches.
pub struct ParquetEncoder;

impl ParquetEncoder {
    /// Encode and write an Arrow `RecordBatch` directly to a Parquet file on disk.
    pub fn write_batch<P: AsRef<Path>>(batch: &RecordBatch, output_path: P) -> Result<u64> {
        let file = File::create(output_path)?;
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
            .map_err(|e| KeiroxError::Internal(format!("Failed to create Parquet writer: {e}")))?;

        writer.write(batch).map_err(|e| {
            KeiroxError::Internal(format!("Failed to write Arrow batch to Parquet: {e}"))
        })?;

        let file_metadata = writer
            .close()
            .map_err(|e| KeiroxError::Internal(format!("Failed to close Parquet writer: {e}")))?;

        Ok(file_metadata.num_rows as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shredder::AdaptiveShredder;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use tempfile::tempdir;

    #[test]
    fn test_parquet_encoder_write_and_read_back() {
        let dir = tempdir().unwrap();
        let parquet_path = dir.path().join("output.parquet");

        let mut shredder = AdaptiveShredder::default();
        let records = vec![
            serde_json::json!({"id": 1, "customer": "alice", "active": true}),
            serde_json::json!({"id": 2, "customer": "bob", "active": false}),
        ];

        let batch = shredder.shred_json_records(&records).unwrap();
        let written_rows = ParquetEncoder::write_batch(&batch, &parquet_path).unwrap();
        assert_eq!(written_rows, 2);

        // Read back from Parquet file to verify compatibility
        let file = File::open(&parquet_path).unwrap();
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();

        let mut total_rows = 0;
        for batch_res in reader {
            let b = batch_res.unwrap();
            total_rows += b.num_rows();
        }
        assert_eq!(total_rows, 2);
    }
}
