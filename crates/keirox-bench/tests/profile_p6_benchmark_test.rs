//! Profile P6 Multi-Region disaster recovery replication benchmark test per `KEI-OPS-041` and `KEI-MR-401`.

use keirox_bench::{BenchmarkConfig, BenchmarkRunner, WorkloadProfile};
use keirox_consensus::{MultiRegionReplicator, RegionId, RegionRole};
use keirox_core::model::{StreamId, TenantId};

#[test]
fn test_profile_p6_multi_region_replication_benchmark() {
    let tenant = TenantId([0x66; 16]);
    let stream = StreamId([0x77; 16]);
    let primary = MultiRegionReplicator::new(RegionId(1), RegionRole::Primary);
    let replica = MultiRegionReplicator::new(RegionId(2), RegionRole::SecondaryReplica);

    let config = BenchmarkConfig::for_profile(WorkloadProfile::P6MultiRegionReplication);
    let payload = vec![0x66; config.payload_size_bytes];

    // Measure 500 WAN replication batch creations and replica ingestions
    let result = BenchmarkRunner::measure(&config, 500, |op_idx| {
        let batch = primary
            .create_replication_batch(
                tenant,
                stream,
                op_idx,
                vec![payload.clone()],
                1_700_000_000 + op_idx,
            )
            .expect("Batch creation must succeed");
        replica
            .apply_replication_batch(&batch, 1_700_000_000 + op_idx + 10)
            .expect("Replica application must succeed");
    });

    assert_eq!(result.total_operations, 500);
    assert!(result.ops_per_sec > 0.0);
    assert!(result.p50_latency_us > 0);
}
