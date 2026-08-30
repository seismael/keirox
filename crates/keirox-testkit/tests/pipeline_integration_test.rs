//! End-to-end integration test demonstrating the Golden Invariant across crates:
//! `RowArena` -> `BatchHeader` -> `StreamRegistryEntry` -> `ConsumerGroupState` -> `TimingWheel` -> `AdaptiveShredder`.

use keirox_arena::RowArena;
use keirox_arrow_elt::AdaptiveShredder;
use keirox_core::{StreamId, TenantId};
use keirox_index::StreamRegistryEntry;
use keirox_state::{ConsumerGroupState, ConsumerState};
use keirox_timer::TimingWheel;
use keirox_wal::framing::BatchHeader;

#[test]
fn test_end_to_end_single_node_pipeline_flow() {
    let tenant = TenantId([1; 16]);
    let stream = StreamId([2; 16]);

    // 1. Hot Ingress over pre-allocated RowArena (zero heap allocations)
    let mut arena = RowArena::with_capacity(4096);
    let row_buffer = arena.alloc(512).expect("Must allocate from arena");
    row_buffer[0..4].copy_from_slice(b"test");

    // 2. Physical WAL Batch Append (Immutable Log)
    let batch = BatchHeader::new(0, 512, 3, 0, 2, 1700000000, 0xCAFEBABE);
    assert!(batch.is_valid());

    // 3. Stream Registry Index Update
    let mut registry = StreamRegistryEntry::new(stream, 1);
    registry.advance_head(batch.last_offset);
    assert_eq!(registry.head_offset, 2);

    // 4. Consumption State Overlay (Streaming & Queuing on same log)
    let mut consumer_group = ConsumerGroupState::new();
    consumer_group.head_offset = batch.last_offset;

    // Lease offset 0, 1, 2 to workers with lease deadlines
    let mut timer_wheel = TimingWheel::new(1000);
    assert!(consumer_group.lease(0, 1050));
    assert!(consumer_group.lease(1, 1100));
    assert!(consumer_group.lease(2, 1050));

    timer_wheel.schedule_timeout(0, 1050);
    timer_wheel.schedule_timeout(1, 1100);
    timer_wheel.schedule_timeout(2, 1050);

    // Out-of-order ACK for offset 2
    consumer_group.ack(2);
    assert_eq!(consumer_group.get_state(2), ConsumerState::Acked);
    assert_eq!(consumer_group.base_watermark, 0); // Blocked by offset 0 & 1

    // Worker processing offset 0 times out!
    let expired = timer_wheel.advance_to(1060);
    assert!(expired.contains(&0));

    // Poison-pill offset 0 evicted to Virtual DLQ
    consumer_group.evict_dlq(0);
    assert_eq!(consumer_group.base_watermark, 1); // Advances past DLQ offset 0

    // Worker 1 ACKs offset 1
    consumer_group.ack(1);
    // Watermark jumps past offset 1 and 2 to 3!
    assert_eq!(consumer_group.base_watermark, 3);

    // 5. Internalized ELT Adaptive Shredder
    let mut shredder = AdaptiveShredder::default();
    assert!(shredder.try_promote_field("user_id"));
    assert!(shredder.try_promote_field("amount"));
    assert_eq!(shredder.promoted_count(), 2);

    println!(
        "Golden Invariant proven for tenant: {}, stream: {}",
        tenant, stream
    );
}
