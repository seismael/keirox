//! Unified single-node runtime coordinator integrating all 12 Keirox subsystems.

use arrow::record_batch::RecordBatch;
use keirox_api::proto::{AckMode, ProduceBatchResponse};
use keirox_arena::RowArena;
use keirox_arrow_elt::AdaptiveShredder;
use keirox_core::error::Result;
use keirox_core::model::{Offset, StreamId};
use keirox_index::StreamRegistryEntry;
use keirox_state::ConsumerGroupState;
use keirox_timer::TimingWheel;
use keirox_wal::framing::{BatchHeader, RecordEntry};
use keirox_wal::segment::SegmentFile;
use std::collections::HashMap;
use std::mem::size_of;
use std::path::{Path, PathBuf};

/// Unified single-node runtime coordinator.
pub struct SingleNodeRuntime {
    _wal_dir: PathBuf,
    active_segment: SegmentFile,
    stream_registry: HashMap<StreamId, StreamRegistryEntry>,
    consumer_groups: HashMap<(StreamId, u64), ConsumerGroupState>,
    timer_wheel: TimingWheel,
    shredder: AdaptiveShredder,
    arena: RowArena,
    current_time_us: u64,
}

impl SingleNodeRuntime {
    /// Initialize a new single-node runtime in the specified WAL directory.
    pub fn init<P: AsRef<Path>>(wal_dir: P) -> Result<Self> {
        let path = wal_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        let seg_path = path.join("0000000000000001.kwal");
        let active_segment = SegmentFile::create(&seg_path, 1, 1, 1, 0)?;

        Ok(Self {
            _wal_dir: path,
            active_segment,
            stream_registry: HashMap::new(),
            consumer_groups: HashMap::new(),
            timer_wheel: TimingWheel::new(1000),
            shredder: AdaptiveShredder::default(),
            arena: RowArena::with_capacity(2 * 1024 * 1024),
            current_time_us: 1000,
        })
    }

    /// Ingress batch produce path writing to immutable WAL.
    pub fn produce(
        &mut self,
        stream_id: StreamId,
        _ack_mode: AckMode,
        records: &[Vec<u8>],
    ) -> Result<ProduceBatchResponse> {
        self.arena.reset();

        let registry = self
            .stream_registry
            .entry(stream_id)
            .or_insert_with(|| StreamRegistryEntry::new(stream_id, 1));

        let base_offset = registry.head_offset;
        let last_offset = base_offset + records.len() as u64 - 1;

        // Construct RecordEntries and payload block
        let mut record_entries = Vec::with_capacity(records.len());
        let mut total_payload_len = 0usize;

        for (i, rec) in records.iter().enumerate() {
            let offset = base_offset + i as u64;
            record_entries.push(RecordEntry::new(
                stream_id.0,
                offset,
                total_payload_len as u32,
                rec.len() as u32,
                0,
            ));
            total_payload_len += rec.len();
        }

        let total_batch_size = (size_of::<BatchHeader>()
            + (record_entries.len() * size_of::<RecordEntry>())
            + total_payload_len) as u32;

        let batch_header = BatchHeader::new(
            0,
            total_batch_size,
            records.len() as u32,
            base_offset,
            last_offset,
            self.current_time_us,
            0,
        );

        // Concatenate payload in memory
        let mut payload = Vec::with_capacity(total_payload_len);
        for rec in records {
            payload.extend_from_slice(rec);
        }

        // Append to physical WAL segment
        self.active_segment
            .append_batch(&batch_header, &record_entries, &payload)?;

        // Update in-memory stream index
        registry.advance_head(last_offset + 1);

        // Update group head offsets
        for ((group_stream, _), group_state) in &mut self.consumer_groups {
            if *group_stream == stream_id {
                group_state.head_offset = last_offset + 1;
            }
        }

        Ok(ProduceBatchResponse {
            base_offset,
            last_offset,
            timestamp_us: self.current_time_us,
        })
    }

    /// Lease available records for a consumer group.
    pub fn lease_records(
        &mut self,
        stream_id: StreamId,
        group_id: u64,
        max_records: u32,
        ttl_ms: u64,
    ) -> Result<Vec<(Offset, u64)>> {
        let registry = self
            .stream_registry
            .entry(stream_id)
            .or_insert_with(|| StreamRegistryEntry::new(stream_id, 1));
        let head = registry.head_offset;

        let group_state = self
            .consumer_groups
            .entry((stream_id, group_id))
            .or_default();
        group_state.head_offset = head;

        let mut leased = Vec::new();
        let deadline = self.current_time_us + (ttl_ms * 1000);

        for offset in group_state.base_watermark..head {
            if leased.len() >= max_records as usize {
                break;
            }

            if let Some(token) = group_state.lease(offset, deadline) {
                self.timer_wheel.schedule_timeout(offset, deadline);
                leased.push((offset, token));
            }
        }

        Ok(leased)
    }

    /// Acknowledge a leased record with fencing token validation.
    pub fn ack_record(
        &mut self,
        stream_id: StreamId,
        group_id: u64,
        offset: Offset,
        token: u64,
    ) -> Result<()> {
        let group_state = self
            .consumer_groups
            .entry((stream_id, group_id))
            .or_default();

        group_state.ack_fenced(offset, token)?;
        group_state.verify_invariants()?;
        Ok(())
    }

    /// Export semi-structured records to Apache Arrow columnar format via adaptive shredder.
    pub fn export_arrow(&mut self, records: &[serde_json::Value]) -> Result<RecordBatch> {
        self.shredder.shred_json_records(records)
    }

    /// Return base watermark for a stream and consumer group.
    pub fn base_watermark(&self, stream_id: StreamId, group_id: u64) -> Offset {
        self.consumer_groups
            .get(&(stream_id, group_id))
            .map(|g| g.base_watermark)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_single_node_runtime_end_to_end() {
        let dir = tempdir().unwrap();
        let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
        let stream = StreamId([0x88; 16]);

        // 1. Produce 3 records
        let records = vec![
            b"{\"user\": \"alice\", \"score\": 100}".to_vec(),
            b"{\"user\": \"bob\", \"score\": 200}".to_vec(),
            b"{\"user\": \"carol\", \"score\": 300}".to_vec(),
        ];
        let resp = runtime
            .produce(stream, AckMode::Durable, &records)
            .expect("Produce must succeed");
        assert_eq!(resp.base_offset, 0);
        assert_eq!(resp.last_offset, 2);

        // 2. Lease records to consumer group 100
        let leased = runtime
            .lease_records(stream, 100, 2, 5000)
            .expect("Lease must succeed");
        assert_eq!(leased.len(), 2);
        assert_eq!(leased[0].0, 0);
        assert_eq!(leased[1].0, 1);

        // 3. ACK offset 0
        runtime
            .ack_record(stream, 100, 0, leased[0].1)
            .expect("Ack must succeed");
        assert_eq!(runtime.base_watermark(stream, 100), 1);

        // 4. Export to Arrow
        let json_records = vec![
            serde_json::json!({"user": "alice", "score": 100}),
            serde_json::json!({"user": "bob", "score": 200}),
        ];
        let batch = runtime
            .export_arrow(&json_records)
            .expect("Arrow export must succeed");
        assert_eq!(batch.num_rows(), 2);
    }
}
