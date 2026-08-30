//! Formal state invariant testing per `KEI-FORMAL-001`.

use keirox_state::{ConsumerGroupState, ConsumerState};

#[test]
fn test_complex_out_of_order_acks_and_invariants() {
    let mut state = ConsumerGroupState::with_max_retries(3);
    state.head_offset = 20;

    // Lease 0..10
    let mut tokens = Vec::new();
    for i in 0..10 {
        let tok = state.lease(i, 5000).expect("Lease must succeed");
        tokens.push((i, tok));
    }

    state
        .verify_invariants()
        .expect("Invariants must hold after leasing");

    // ACK odd offsets out of order (1, 3, 5, 7, 9)
    for &(offset, token) in &tokens {
        if offset % 2 == 1 {
            state
                .ack_fenced(offset, token)
                .expect("Fenced ack must succeed");
            assert_eq!(state.get_state(offset), ConsumerState::Acked);
        }
    }

    // Watermark should still be 0 (blocked by offset 0)
    assert_eq!(state.base_watermark, 0);
    state
        .verify_invariants()
        .expect("Invariants must hold during partial out-of-order ACKs");

    // Evict offset 0 to DLQ -> Watermark should cascade to 2!
    state.evict_dlq(0);
    assert_eq!(state.base_watermark, 2);
    state
        .verify_invariants()
        .expect("Invariants must hold after DLQ eviction");

    // ACK offset 2 -> Watermark should cascade past 2 and 3 to 4!
    let token2 = tokens[2].1;
    state
        .ack_fenced(2, token2)
        .expect("Fenced ack must succeed");
    assert_eq!(state.base_watermark, 4);
    state
        .verify_invariants()
        .expect("Invariants must hold after advancing watermark");
}

#[test]
fn test_no_double_lease_invariant() {
    let mut state = ConsumerGroupState::new();
    state.head_offset = 5;

    // Lease offset 0
    let tok1 = state.lease(0, 1000);
    assert!(tok1.is_some());

    // Second lease attempt on same offset without release MUST fail
    let tok2 = state.lease(0, 1000);
    assert!(tok2.is_none(), "Double leasing must be rejected");

    state.verify_invariants().expect("Invariants must hold");
}

#[test]
fn test_no_terminal_state_regression() {
    let mut state = ConsumerGroupState::new();
    state.head_offset = 5;

    // Offset 0 Acked
    state.ack(0);
    assert_eq!(state.get_state(0), ConsumerState::Acked);

    // Attempt to lease already Acked offset MUST fail
    let tok = state.lease(0, 1000);
    assert!(tok.is_none(), "Cannot lease terminal offset");

    state.verify_invariants().expect("Invariants must hold");
}
