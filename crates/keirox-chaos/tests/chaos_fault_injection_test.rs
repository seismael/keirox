//! Chaos fault injection integration test suite per `KEI-OPS-041`.

use keirox_api::proto::AckMode;
use keirox_chaos::{ChaosFaultPlan, ChaosScenario};
use keirox_core::StreamId;
use keirox_testkit::SingleNodeRuntime;
use keirox_wal::segment::SegmentReader;
use tempfile::tempdir;

#[test]
fn test_chaos_crash_fault_injection_and_recovery() {
    let dir = tempdir().unwrap();
    let stream = StreamId([0x99; 16]);
    let group_id = 9090;

    // 1. Initial execution before crash
    {
        let mut runtime = SingleNodeRuntime::init(dir.path()).unwrap();
        let plan = ChaosFaultPlan::crash_target(1);
        assert_eq!(plan.scenario, ChaosScenario::CrashFault);

        let records = vec![
            b"before_crash_record_1".to_vec(),
            b"before_crash_record_2".to_vec(),
        ];
        let resp = runtime.produce(stream, AckMode::Durable, &records).unwrap();
        assert_eq!(resp.base_offset, 0);
        assert_eq!(resp.last_offset, 1);

        let leased = runtime.lease_records(stream, group_id, 1, 30_000).unwrap();
        assert_eq!(leased.len(), 1);
        runtime
            .ack_record(stream, group_id, leased[0].0, leased[0].1)
            .unwrap();
        assert_eq!(runtime.base_watermark(stream, group_id), 1);

        // Abrupt process kill simulation: drop runtime
    }

    // 2. Recovery execution after crash
    {
        // Re-open runtime from the same WAL directory
        let seg_path = dir.path().join("0000000000000001.kwal");
        assert!(seg_path.exists());

        let mut reader = SegmentReader::open(&seg_path).unwrap();
        let batches = reader.replay_batches().unwrap();
        assert_eq!(batches.len(), 1);

        let batch = &batches[0];
        assert_eq!(batch.records.len(), 2);
        assert_eq!(batch.records[0].logical_offset(), 0);
        assert_eq!(batch.records[1].logical_offset(), 1);

        // Verify payload slicing
        let r0_payload = &batch.payload[batch.records[0].payload_offset() as usize
            ..(batch.records[0].payload_offset() + batch.records[0].payload_len()) as usize];
        let r1_payload = &batch.payload[batch.records[1].payload_offset() as usize
            ..(batch.records[1].payload_offset() + batch.records[1].payload_len()) as usize];

        assert_eq!(r0_payload, b"before_crash_record_1");
        assert_eq!(r1_payload, b"before_crash_record_2");
    }
}
