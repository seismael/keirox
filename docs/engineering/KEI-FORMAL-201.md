# KEI-FORMAL-201 — Distributed Consensus & Multi-Node State Verification Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-FORMAL-201 |
| Title | Distributed Consensus & Multi-Node State Verification Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 2 Engineering Bridge (parallel track) |
| Duration | 90 days / 12 weeks |
| Owner | Formal Methods Lead / Distributed Systems Lead |
| Governing Plan | KEI-ENG-200 — Phase 2 Engineering Execution Plan |
| Related Plans | KEI-SPIKE-201 (Distributed Consensus Prototype), KEI-FORMAL-101 (Phase 1 State Machine Validation) |
| Governing Architecture Documents | KEI-ARC-021, KEI-ARC-022, KEI-DES-031 |
| Predecessor | KEI-FORMAL-101 (Phase 1 State Machine Validation Plan) |
| Next Plan File | KEI-BENCH-201 — Multi-Node Performance & Failover Harness Plan |

---

## 2. Executive Summary

Phase 1 formal validation (KEI-FORMAL-101) proved that the single-node consumption state machine is correct. Phase 2 introduces the most dangerous class of distributed systems bugs: **multi-node state inconsistency under failure**.

This plan defines the formal verification strategy for:

1. **Raft consensus integration** — proving that WAL segment heads replicate correctly and that leader election preserves the Golden Invariant.
2. **Coordinator epoch fencing** — proving that stale coordinator operations are always rejected and that no double-lease can occur under split-brain.
3. **State replication correctness** — proving that bitmap snapshots and lease deltas replicate consistently and that replay produces identical state.
4. **Recovery determinism** — proving that node replacement from S3 manifests + peer WAL deltas produces the same state as the original node.
5. **Split-brain safety** — proving that network partitions cannot cause conflicting lease grants or watermark regressions.

The output of this plan is a set of verified TLA+ models, derived test oracles, and runtime invariant checks that are embedded into the Phase 2 prototype and all subsequent implementations.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Formally model the two-tier Raft topology and verify consensus safety.
2. Verify coordinator epoch fencing prevents stale operations.
3. Verify state replication (bitmap snapshots + lease deltas) is consistent.
4. Verify recovery from S3 manifests + peer WAL deltas is deterministic.
5. Verify split-brain scenarios cannot cause double leases or watermark regressions.
6. Derive test oracles for multi-node invariant checking.
7. Integrate oracles into the KEI-SPIKE-201 prototype.

### 3.2 Scope

**In scope:**

1. Data Plane Raft model (WAL segment head replication).
2. Metadata & State Raft model (coordinator assignments, manifests, snapshots).
3. Coordinator epoch fencing model.
4. Bitmap snapshot replication model.
5. Lease delta replication model.
6. Recovery determinism model.
7. Split-brain safety model.
8. Test oracle derivation.
9. Integration with KEI-SPIKE-201 prototype.

**Out of scope:**

1. Single-node state machine verification (completed in KEI-FORMAL-101).
2. Network protocol verification (TCP/gRPC correctness).
3. Performance modeling.
4. Storage engine binary format verification.
5. Columnar ELT pipeline verification.
6. Multi-region replication (Phase 4).

---

## 4. Relationship to KEI-FORMAL-101

Phase 1 formal validation (KEI-FORMAL-101) established the correctness of the single-node state machine. This plan extends that work into the distributed domain.

| KEI-FORMAL-101 Scope | KEI-FORMAL-201 Extension |
|---|---|
| Single-shard state machine | Multi-node state replication |
| Lease lifecycle (single node) | Lease lifecycle under coordinator failover |
| Watermark advancement (single node) | Watermark advancement under epoch fencing |
| Journal replay determinism | Recovery from S3 manifests + peer WAL deltas |
| Epoch fencing (single coordinator) | Epoch fencing across 3-node cluster |

**Normative rule:** All Phase 1 invariants verified in KEI-FORMAL-101 MUST continue to hold in the distributed setting. Any violation discovered in Phase 2 modeling MUST be treated as a critical defect.

---

## 5. System Model Components

### 5.1 Model 6 — Data Plane Raft (WAL Segment Heads)

This model verifies that WAL segment heads replicate correctly across a 3-node quorum.

**State variables:**

```text
nodes            : set of 3 nodes {n1, n2, n3}
leader           : current Raft leader node
term             : current Raft term
log              : function mapping node → sequence of WAL segment heads
commitIndex      : highest committed log entry
producerAcked    : set of producer writes that have been ACKed
```

**Actions:**

```text
RequestVote(candidate, term)
GrantVote(voter, candidate, term)
BecomeLeader(node, term)
AppendEntries(leader, follower, entries)
AckAppend(follower, leader)
CommitEntry(leader, index)
ProducerAck(producer, entry)
LeaderFailure(node)
FollowerFailure(node)
NetworkPartition(partitioned_nodes)
```

**Key invariants to verify:**

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-P2-RAFT-001 | Leader uniqueness | At most one leader per term |
| INV-P2-RAFT-002 | Log matching | If two logs contain an entry with the same index and term, all preceding entries match |
| INV-P2-RAFT-003 | Commit safety | A committed entry is never lost |
| INV-P2-RAFT-004 | Producer ACK safety | Producer ACK is issued only after quorum commit |
| INV-P2-RAFT-005 | No ACK without commit | If producerAcked(w), then w is committed |

### 5.2 Model 7 — Metadata & State Raft (Coordinator Assignments)

This model verifies that coordinator assignments, manifests, and state snapshots replicate correctly.

**State variables:**

```text
metaLog          : Metadata Raft log
coordinatorMap   : function mapping consumer_group → coordinator_node
manifestVersion  : current manifest version
snapshotVersion  : current bitmap snapshot version
committedWBase   : committed watermark per consumer group
```

**Actions:**

```text
AssignCoordinator(group, node)
ReassignCoordinator(group, old_node, new_node)
ReplicateSnapshot(group, bitmap_snapshot)
ReplicateLeaseDelta(group, lease_delta)
CommitWatermark(group, W_base)
MetaLeaderFailure(node)
```

**Key invariants to verify:**

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-P2-META-001 | Coordinator uniqueness | Each consumer group has at most one active coordinator |
| INV-P2-META-002 | Snapshot consistency | Replicated snapshots match source snapshots |
| INV-P2-META-003 | Lease delta ordering | Lease deltas are applied in order |
| INV-P2-META-004 | Watermark durability | Committed watermarks survive node failures |
| INV-P2-META-005 | No orphaned coordinator | Every consumer group has a coordinator or is being reassigned |

### 5.3 Model 8 — Coordinator Epoch Fencing

This model verifies that epoch fencing prevents stale coordinator operations.

**State variables:**

```text
coordinatorEpoch : function mapping consumer_group → current epoch
activeCoordinator: function mapping consumer_group → active node
staleRequests    : set of requests with old epochs
```

**Actions:**

```text
CoordinatorFailover(group, old_node, new_node)
IncrementEpoch(group)
SubmitOperation(group, request, epoch)
RejectStaleOperation(group, request)
SplitBrainPartition(group, node_a, node_b)
```

**Key invariants to verify:**

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-P2-EPOCH-001 | Epoch monotonicity | Epochs never decrease |
| INV-P2-EPOCH-002 | Stale rejection | Operations with epoch < current epoch are rejected |
| INV-P2-EPOCH-003 | No double coordinator | At most one node holds the current epoch for a group |
| INV-P2-EPOCH-004 | Split-brain fencing | Under partition, minority node's operations are rejected |
| INV-P2-EPOCH-005 | No double lease | No offset has two active leases across any partition scenario |

### 5.4 Model 9 — State Replication & Recovery

This model verifies that bitmap snapshots and lease deltas replicate correctly and that recovery is deterministic.

**State variables:**

```text
sourceState      : state on primary coordinator
replicaState     : state on replica node
journalEntries   : sequence of lease deltas
snapshotLSN      : LSN of latest snapshot
recoveredState   : state after recovery
```

**Actions:**

```text
TakeSnapshot(group)
ReplicateSnapshot(group, replica)
AppendLeaseDelta(group, delta)
ReplicateDelta(group, delta, replica)
NodeCrash(node)
RecoverFromSnapshot(group, replica)
ReplayDeltas(group, replica)
VerifyStateConsistency(source, replica)
```

**Key invariants to verify:**

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-P2-REPL-001 | Snapshot fidelity | Replicated snapshot equals source snapshot at snapshot LSN |
| INV-P2-REPL-002 | Delta completeness | All deltas after snapshot LSN are replicated |
| INV-P2-REPL-003 | Replay determinism | Replay(snapshot + deltas) produces identical state |
| INV-P2-REPL-004 | Recovery correctness | Recovered state equals source state at failure time |
| INV-P2-REPL-005 | No state regression | Recovered watermark ≥ source watermark at snapshot time |

### 5.5 Model 10 — Split-Brain Safety

This model verifies that network partitions cannot cause conflicting lease grants or watermark regressions.

**State variables:**

```text
partitionState   : which nodes are partitioned
majorityNodes    : nodes in the majority partition
minorityNodes    : nodes in the minority partition
leaseGrants      : set of lease grants issued during partition
watermarkState   : watermark state across nodes
```

**Actions:**

```text
PartitionNetwork(majority, minority)
MajorityContinues(majority_nodes)
MinorityAttemptsLease(minority_nodes)
HealPartition()
ReconcileState()
```

**Key invariants to verify:**

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-P2-SB-001 | Majority authority | Only majority partition can issue leases |
| INV-P2-SB-002 | Minority fencing | Minority partition operations are rejected |
| INV-P2-SB-003 | No double lease during partition | No offset has two active leases across partitions |
| INV-P2-SB-004 | Watermark monotonicity during partition | Watermark never decreases during partition |
| INV-P2-SB-005 | Healing safety | After partition heals, state is consistent |

---

## 6. Safety Invariants Summary

The following invariants MUST hold in all reachable states across all five models.

### 6.1 Raft Consensus Invariants

| ID | Invariant | Source |
|---|---|---|
| INV-P2-RAFT-001 | Leader uniqueness per term | Raft paper |
| INV-P2-RAFT-002 | Log matching property | Raft paper |
| INV-P2-RAFT-003 | Commit safety | Raft paper |
| INV-P2-RAFT-004 | Producer ACK only after quorum commit | KEI-ARC-022 |
| INV-P2-RAFT-005 | No ACK without commit | KEI-ARC-022 |

### 6.2 Coordinator & State Invariants

| ID | Invariant | Source |
|---|---|---|
| INV-P2-META-001 | Coordinator uniqueness per group | KEI-ARC-021 |
| INV-P2-META-002 | Snapshot consistency | KEI-DES-031 |
| INV-P2-META-003 | Lease delta ordering | KEI-DES-031 |
| INV-P2-META-004 | Watermark durability | KEI-ARC-021 |
| INV-P2-META-005 | No orphaned coordinator | KEI-ARC-022 |

### 6.3 Epoch Fencing Invariants

| ID | Invariant | Source |
|---|---|---|
| INV-P2-EPOCH-001 | Epoch monotonicity | KEI-ARC-021 |
| INV-P2-EPOCH-002 | Stale rejection | KEI-ARC-021 |
| INV-P2-EPOCH-003 | No double coordinator | KEI-ARC-021 |
| INV-P2-EPOCH-004 | Split-brain fencing | KEI-ARC-022 |
| INV-P2-EPOCH-005 | No double lease | KEI-ARC-021 |

### 6.4 Replication & Recovery Invariants

| ID | Invariant | Source |
|---|---|---|
| INV-P2-REPL-001 | Snapshot fidelity | KEI-DES-031 |
| INV-P2-REPL-002 | Delta completeness | KEI-DES-031 |
| INV-P2-REPL-003 | Replay determinism | KEI-DES-031 |
| INV-P2-REPL-004 | Recovery correctness | KEI-ARC-022 |
| INV-P2-REPL-005 | No state regression | KEI-ARC-021 |

### 6.5 Split-Brain Invariants

| ID | Invariant | Source |
|---|---|---|
| INV-P2-SB-001 | Majority authority | Raft paper |
| INV-P2-SB-002 | Minority fencing | KEI-ARC-022 |
| INV-P2-SB-003 | No double lease during partition | KEI-ARC-021 |
| INV-P2-SB-004 | Watermark monotonicity during partition | KEI-ARC-021 |
| INV-P2-SB-005 | Healing safety | KEI-ARC-022 |

---

## 7. Liveness Properties

The following liveness properties MUST hold under fair scheduling.

| ID | Property | Formal Statement |
|---|---|---|
| INV-P2-L-001 | Leader election completes | If a leader fails, a new leader is eventually elected |
| INV-P2-L-002 | Coordinator reassignment completes | If a coordinator fails, a successor is eventually assigned |
| INV-P2-L-003 | Recovery completes | If a node crashes, it eventually recovers |
| INV-P2-L-004 | Partition heals | Network partitions eventually heal |
| INV-P2-L-005 | Committed writes are readable | Committed WAL entries are eventually readable |

---

## 8. Model Checking Strategy

### 8.1 Bounded Model Checking

TLC will be used with bounded state spaces:

| Parameter | Bound |
|---|---|
| Nodes | 3 (fixed for Raft quorum) |
| Consumer groups | 2–3 |
| Offsets per group | 0..6 |
| Workers | 2 |
| Raft terms | 0..4 |
| Coordinator epochs | 0..4 |
| Journal length | 0..8 |
| Partition configurations | All 2^3 subsets |

### 8.2 Model Decomposition

| Model | Focus | Nodes | Groups | Offsets |
|---|---|---|---|---|
| M6 | Data Plane Raft | 3 | 0 | 0 |
| M7 | Metadata Raft | 3 | 2 | 0 |
| M8 | Epoch Fencing | 3 | 2 | 4 |
| M9 | State Replication | 2 | 1 | 6 |
| M10 | Split-Brain | 3 | 2 | 4 |

### 8.3 State Space Estimation

For the split-brain model (M10) with 3 nodes, 2 groups, 4 offsets, and 4 epochs:

```text
State space ≈ 3^3 × 2^2 × 4^4 × 4^2 × partition_configs
            ≈ 27 × 4 × 256 × 16 × 8
            ≈ 3.5 million states
```

This is tractable for TLC exhaustive exploration.

---

## 9. Counterexample Handling

When TLC finds a counterexample:

1. Record the counterexample trace.
2. Analyze whether it represents a real bug or a model artifact.
3. If real bug:
   - File a critical defect.
   - Update the distributed protocol design.
   - Update the implementation.
   - Re-run model checker.
4. If model artifact:
   - Refine the model.
   - Add missing constraints.
   - Document the refinement.
5. All counterexamples are archived in the formal validation report.

**Normative rule:** No counterexample may be dismissed without written justification approved by the Formal Methods Lead and Chief Architect.

---

## 10. Test Oracle Derivation

### 10.1 Purpose

Every invariant verified in the distributed models becomes a runtime assertion in the Phase 2 prototype.

### 10.2 Oracle Mapping

| Formal Invariant | Runtime Check in Prototype |
|---|---|
| INV-P2-RAFT-004 | Assert producer ACK only after quorum commit |
| INV-P2-META-001 | Assert each group has at most one coordinator |
| INV-P2-EPOCH-002 | Assert stale epoch operations are rejected |
| INV-P2-EPOCH-005 | Assert no double lease across nodes |
| INV-P2-REPL-003 | Assert replay determinism on recovery |
| INV-P2-SB-003 | Assert no double lease during partition tests |
| INV-P2-SB-004 | Assert watermark monotonicity during partition tests |

### 10.3 Oracle Integration

The runtime invariant checker is embedded in:

- `keirox-consensus` crate (Raft integration layer).
- `keirox-state` crate (coordinator sharding layer).
- `keirox-testkit` crate (multi-node test assertions).
- `keirox-chaos` crate (chaos test invariant checks).

---

## 11. Integration with KEI-SPIKE-201 Prototype

### 11.1 Relationship

This plan runs in parallel with KEI-SPIKE-201.

| Prototype Week | Formal Validation Activity |
|---|---|
| Weeks 1–3 | Model 6 (Data Plane Raft) drafted and checked |
| Weeks 3–5 | Model 7 (Metadata Raft) drafted and checked |
| Weeks 5–7 | Model 8 (Epoch Fencing) drafted and checked |
| Weeks 7–9 | Model 9 (State Replication) drafted and checked |
| Weeks 9–11 | Model 10 (Split-Brain) drafted and checked |
| Weeks 10–12 | Test oracles integrated into prototype |
| Week 12 | Formal validation report delivered with prototype evidence package |

### 11.2 Feedback Loop

When the prototype discovers a behavior that contradicts the model:

1. Determine whether the model or the implementation is wrong.
2. If model is wrong, update model and re-check.
3. If implementation is wrong, fix implementation and add regression test.
4. Update test oracles if needed.

---

## 12. Deliverables

| Deliverable | Description | Due |
|---|---|---|
| D-P2-F-001 | Model 6: Data Plane Raft TLA+ specification | Week 3 |
| D-P2-F-002 | Model 7: Metadata Raft TLA+ specification | Week 5 |
| D-P2-F-003 | Model 8: Epoch Fencing TLA+ specification | Week 7 |
| D-P2-F-004 | Model 9: State Replication TLA+ specification | Week 9 |
| D-P2-F-005 | Model 10: Split-Brain TLA+ specification | Week 11 |
| D-P2-F-006 | Model checking results for all five models | Week 11 |
| D-P2-F-007 | Counterexample archive | Week 11 |
| D-P2-F-008 | Distributed test oracle specification | Week 10 |
| D-P2-F-009 | Runtime invariant checker integration | Week 11 |
| D-P2-F-010 | Distributed formal validation summary report | Week 12 |

---

## 13. Acceptance Criteria

| ID | Criterion |
|---|---|
| ACC-P2-F-001 | All five TLA+ models compile and pass TLC model checking |
| ACC-P2-F-002 | All Raft consensus safety invariants hold |
| ACC-P2-F-003 | All epoch fencing invariants hold |
| ACC-P2-F-004 | All state replication invariants hold |
| ACC-P2-F-005 | All split-brain safety invariants hold |
| ACC-P2-F-006 | All liveness properties hold under fair scheduling |
| ACC-P2-F-007 | All counterexamples are resolved or documented |
| ACC-P2-F-008 | Test oracles are derived for all critical invariants |
| ACC-P2-F-009 | Runtime invariant checker is integrated into prototype |
| ACC-P2-F-010 | Prototype passes all runtime invariant checks during chaos tests |
| ACC-P2-F-011 | Formal validation report is approved by Chief Architect |

---

## 14. Dependencies

| Dependency | Source |
|---|---|
| Phase 1 state machine models | KEI-FORMAL-101 |
| Two-tier Raft topology | KEI-ARC-022 §5 |
| Coordinator epoch fencing | KEI-ARC-021 §10 |
| State replication protocol | KEI-DES-031 §18 |
| Split-brain defense | KEI-ARC-022 §8 |
| Prototype implementation | KEI-SPIKE-201 |
| Test infrastructure | KEI-OPS-041 |

---

## 15. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| State space explosion in distributed models | High | Medium | Decompose into sub-models; use symmetry reduction |
| Raft model too complex for TLC | Medium | Medium | Use existing Raft TLA+ specifications as starting point |
| Split-brain scenarios miss edge cases | High | Low | Enumerate all partition configurations; add adversarial scheduling |
| Prototype diverges from model | Medium | Medium | Establish feedback loop; update model when prototype changes |
| Formal methods expertise gap | High | Medium | Assign experienced TLA+ practitioner; use existing templates |
| Counterexamples reveal fundamental design flaws | Critical | Low | Architecture Review Board review; pivot if necessary |

---

## 16. TLA+ Specification Skeleton

Below is the skeleton of the Epoch Fencing model (Model 8). This will be expanded into a full specification during execution.

```text
---- MODULE KeiroxEpochFencing ----
EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    Nodes,          \* Set of 3 nodes {n1, n2, n3}
    Groups,         \* Set of consumer groups {g1, g2}
    MaxEpoch        \* Maximum coordinator epoch, e.g., 4

VARIABLES
    coordinatorEpoch,   \* Function: group -> epoch
    activeCoordinator,  \* Function: group -> node
    leaseGrants,        \* Set of (group, offset, node, epoch) tuples
    partitionState      \* Which nodes are partitioned

vars == <<coordinatorEpoch, activeCoordinator, leaseGrants, partitionState>>

TypeOK ==
    /\ coordinatorEpoch \in [Groups -> 0..MaxEpoch]
    /\ activeCoordinator \in [Groups -> Nodes]

Init ==
    /\ coordinatorEpoch = [g \in Groups |-> 0]
    /\ activeCoordinator = [g \in Groups |-> CHOOSE n \in Nodes: TRUE]
    /\ leaseGrants = {}
    /\ partitionState = {}  \* No partition initially

CoordinatorFailover(g, old_node, new_node) ==
    /\ activeCoordinator[g] = old_node
    /\ new_node # old_node
    /\ coordinatorEpoch' = [coordinatorEpoch EXCEPT ![g] = coordinatorEpoch[g] + 1]
    /\ activeCoordinator' = [activeCoordinator EXCEPT ![g] = new_node]
    /\ UNCHANGED <<leaseGrants, partitionState>>

SubmitOperation(g, node, epoch) ==
    /\ activeCoordinator[g] = node
    /\ epoch = coordinatorEpoch[g]
    /\ TRUE  \* Operation accepted

RejectStaleOperation(g, node, epoch) ==
    /\ epoch < coordinatorEpoch[g]
    /\ TRUE  \* Operation rejected

LeaseGrant(g, offset, node, epoch) ==
    /\ activeCoordinator[g] = node
    /\ epoch = coordinatorEpoch[g]
    /\ leaseGrants' = leaseGrants \cup {<<g, offset, node, epoch>>}
    /\ UNCHANGED <<coordinatorEpoch, activeCoordinator, partitionState>>

Next ==
    \E g \in Groups, old \in Nodes, new \in Nodes:
        CoordinatorFailover(g, old, new)
    \/ \E g \in Groups, n \in Nodes, e \in 0..MaxEpoch:
        SubmitOperation(g, n, e) \/ RejectStaleOperation(g, n, e)
    \/ \E g \in Groups, o \in 0..6, n \in Nodes, e \in 0..MaxEpoch:
        LeaseGrant(g, o, n, e)

Spec == Init /\ [][Next]_vars

\* Safety Invariants
EpochMonotonic ==
    \A g \in Groups:
        coordinatorEpoch[g] >= 0

NoDoubleCoordinator ==
    \A g \in Groups:
        \E! n \in Nodes: activeCoordinator[g] = n

NoDoubleLease ==
    \A g \in Groups, o \in 0..6:
        Cardinality({t \in leaseGrants: t[1] = g /\ t[2] = o}) <= 1

====
```

---

## 17. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial Distributed Consensus & Multi-Node State Verification Plan. Defines five TLA+ models (Data Plane Raft, Metadata Raft, Epoch Fencing, State Replication, Split-Brain Safety), safety and liveness invariants, model checking strategy, counterexample handling, test oracle derivation, and integration with KEI-SPIKE-201 prototype. |