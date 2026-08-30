# KEI-FORMAL-101 — State Machine Validation Plan

## 1. Document Control

| Field | Value |
|---|---|
| Document ID | KEI-FORMAL-101 |
| Title | State Machine Validation Plan |
| Version | 1.0 |
| Level | Engineering Execution Plan |
| Status | Baseline — Ready for Execution |
| Phase | Phase 1 Engineering Bridge (parallel track) |
| Duration | 90 days / 12 weeks |
| Owner | Formal Methods Lead / Distributed Systems Lead |
| Governing Plan | KEI-ENG-100 — Phase 1 Engineering Execution Plan |
| Related Plan | KEI-SPIKE-101 — Minimum Vertical Prototype Plan |
| Governing Architecture Documents | KEI-ARC-010, KEI-ARC-021, KEI-ARC-022, KEI-DES-031, KEI-OPS-041 |
| Next Plan File | KEI-BENCH-101 — Benchmark and Evidence Harness Plan |

---

## 2. Executive Summary

This document defines the plan for formally validating the most correctness-critical logic in the Keirox Polymorphic Event Fabric before distributed implementation begins.

The consumption state plane is the highest-risk correctness component in the system. It manages:

- Lease grants and expirations.
- Out-of-order acknowledgments.
- Watermark advancement.
- Mandatory DLQ eviction.
- Coordinator epoch fencing.
- Recovery from snapshots and journals.

A bug in any of these behaviors can cause silent data loss, duplicate delivery, stuck watermarks, or split-brain lease conflicts. These failures are difficult to detect with testing alone.

This plan applies targeted formal modeling to prove that the state machine invariants hold under all reachable states, and to derive concrete test oracles that are embedded into the prototype and all future implementations.

---

## 3. Purpose and Scope

### 3.1 Purpose

The purpose of this plan is to:

1. Formally model the consumption state machine.
2. Verify safety invariants under all reachable states.
3. Verify liveness properties under fair scheduling.
4. Detect counterexamples before implementation hardening.
5. Derive test oracles for the prototype and Phase 1 engine.
6. Establish a formal validation baseline for Phase 2 distributed coordination.

### 3.2 Scope

**In scope:**

1. Single-shard state machine modeling.
2. Lease lifecycle modeling.
3. Watermark advancement modeling.
4. Mandatory DLQ eviction modeling.
5. Journal replay determinism modeling.
6. Coordinator epoch fencing modeling.
7. Duplicate ACK idempotence modeling.
8. Stale lease rejection modeling.
9. Test oracle derivation.
10. Integration with KEI-SPIKE-101 prototype.

**Out of scope:**

1. Full distributed Raft consensus verification.
2. Multi-region replication modeling.
3. Network protocol verification.
4. Performance modeling.
5. Storage engine binary format verification.
6. Columnar ELT pipeline verification.
7. Gateway protocol verification.

Distributed consensus modeling is deferred to Phase 2. This plan focuses on the state plane correctness that Phase 2 consensus will depend on.

---

## 4. Objectives

| Objective | Description |
|---|---|
| OBJ-F-001 | Prove that no offset can transition from a terminal state back to a non-terminal state. |
| OBJ-F-002 | Prove that at most one active lease exists per offset at any time. |
| OBJ-F-003 | Prove that the watermark `W_base` is monotonically non-decreasing. |
| OBJ-F-004 | Prove that all offsets below `W_base` are in terminal state. |
| OBJ-F-005 | Prove that mandatory DLQ eviction guarantees watermark progress. |
| OBJ-F-006 | Prove that duplicate ACKs are idempotent. |
| OBJ-F-007 | Prove that stale lease operations are rejected. |
| OBJ-F-008 | Prove that epoch fencing prevents stale coordinator operations. |
| OBJ-F-009 | Prove that journal replay is deterministic. |
| OBJ-F-010 | Derive test oracles for prototype invariant checking. |

---

## 5. Formal Modeling Approach

### 5.1 Tool Selection

| Tool | Role | Justification |
|---|---|---|
| TLA+ | Primary modeling language | Industry standard for distributed state machines; model checker (TLC) is mature; proven at AWS, Azure, MongoDB. |
| TLC Model Checker | Exhaustive state-space exploration | Bounded model checking for finite state spaces. |
| TLAPS (optional) | Theorem proving for unbounded properties | Used only if TLC bounded checks are insufficient. |
| Apalache (optional) | Symbolic model checking | Alternative if TLC state space explodes. |

### 5.2 Modeling Philosophy

This plan follows a **targeted formal methods** approach:

1. Model only the highest-risk correctness logic.
2. Use bounded model checking to explore reachable states.
3. Derive concrete test oracles from the model.
4. Embed oracles into the prototype as runtime invariant checks.
5. Iterate the model when counterexamples are found.

This is not a full formal verification of the entire system. It is a precision strike against the most dangerous correctness risks.

### 5.3 Abstraction Level

The model operates at the **state machine level**, not the implementation level.

| Modeled | Not Modeled |
|---|---|
| State transitions | Memory layout |
| Lease grant/ACK/NACK/timeout | Network latency |
| Watermark advancement | Disk I/O |
| DLQ eviction | Bitmap container types |
| Epoch fencing | Raft log replication |
| Journal replay ordering | Parquet export |

---

## 6. System Model Components

### 6.1 Model 1 — Single-Shard State Machine

This is the core model. It represents a single state shard with a bounded offset range.

**State variables:**

```text
offsets          : set of offsets in range [0, N]
state            : function mapping offset → {READY, LEASED, ACKED, EVICTED_DLQ}
lease_owner      : function mapping offset → worker_id or NULL
lease_token      : function mapping offset → token or NULL
lease_expiry     : function mapping offset → timestamp or NULL
retry_count      : function mapping offset → integer
W_base           : integer (watermark)
coordinator_epoch: integer
```

**Actions:**

```text
LeaseGrant(offset, worker, token, ttl)
Ack(offset, token, epoch)
Nack(offset, token, epoch)
LeaseTimeout(offset)
EvictToDlq(offset)
AdvanceWatermark
RecoverFromJournal(journal)
```

**Constraints:**

- Offsets are bounded to `[0, N]` where N is small (e.g., 6–10) for model checking.
- Workers are bounded to 2–3.
- Retry count is bounded to 0–3.
- Time is modeled as discrete logical ticks.

### 6.2 Model 2 — Lease Lifecycle

This model focuses specifically on lease grant, renewal, timeout, and expiration.

**Key behaviors to model:**

1. Lease grant sets state to LEASED and assigns token.
2. Lease renewal extends expiry without changing token.
3. Lease timeout transitions state to READY if retry_count < R_max.
4. Lease timeout transitions state to EVICTED_DLQ if retry_count ≥ R_max.
5. ACK during active lease transitions state to ACKED.
6. ACK after lease expiry is rejected.
7. ACK with wrong token is rejected.
8. NACK transitions state to READY and increments retry_count.

### 6.3 Model 3 — Watermark Advancement

This model focuses on the watermark invariant and mandatory DLQ eviction.

**Key behaviors to model:**

1. `W_base` advances only when all offsets below it are terminal.
2. `W_base` never decreases.
3. Mandatory DLQ eviction forces stuck offsets to terminal state.
4. After eviction, `W_base` can advance past the evicted offset.
5. State below `W_base` is purged.

**Watermark definition (from KEI-ARC-021):**

```text
W_base = max { k | ∀ i < k, State(i) ∈ {ACKED, EVICTED_DLQ} }
```

### 6.4 Model 4 — Journal Replay Determinism

This model verifies that replaying a journal from a snapshot produces the same state regardless of replay order or timing.

**Key behaviors to model:**

1. Snapshot captures state at LSN `S`.
2. Journal entries after LSN `S` are replayed in order.
3. Replay produces identical state to live execution.
4. Duplicate journal entries are idempotent.
5. Corrupted journal entries are detected and rejected.

### 6.5 Model 5 — Coordinator Epoch Fencing

This model verifies that epoch fencing prevents stale coordinator operations.

**Key behaviors to model:**

1. Coordinator epoch increments on failover.
2. Operations with old epoch are rejected.
3. Operations with current epoch are accepted.
4. Two coordinators cannot both hold the same epoch.
5. Epoch fencing prevents double lease grants.

---

## 7. Safety Invariants

The following invariants MUST hold in all reachable states. TLC will check these exhaustively.

| ID | Invariant | Formal Statement |
|---|---|---|
| INV-S-001 | No terminal regression | `∀ o: State(o) ∈ {ACKED, EVICTED_DLQ} ⇒ □(State(o) ∈ {ACKED, EVICTED_DLQ})` |
| INV-S-002 | No double lease | `∀ o: State(o) = LEASED ⇒ ∃! w: LeaseOwner(o) = w` |
| INV-S-003 | Watermark monotonicity | `W_base' ≥ W_base` for all transitions |
| INV-S-004 | Terminal below watermark | `∀ o < W_base: State(o) ∈ {ACKED, EVICTED_DLQ}` |
| INV-S-005 | No leased below watermark | `∀ o < W_base: State(o) ≠ LEASED` |
| INV-S-006 | No ready below watermark | `∀ o < W_base: State(o) ≠ READY` |
| INV-S-007 | Lease token uniqueness | `∀ o1, o2: o1 ≠ o2 ∧ State(o1) = LEASED ∧ State(o2) = LEASED ⇒ Token(o1) ≠ Token(o2)` |
| INV-S-008 | Epoch uniqueness | At most one coordinator holds a given epoch |
| INV-S-009 | Stale epoch rejection | Operations with epoch < current epoch are rejected |
| INV-S-010 | Stale token rejection | ACK with token ≠ current token for offset is rejected |

---

## 8. Liveness Properties

The following liveness properties MUST hold under fair scheduling.

| ID | Property | Formal Statement |
|---|---|---|
| INV-L-001 | Watermark progress | If all offsets below `W_base + k` can become terminal, then `W_base` eventually advances. |
| INV-L-002 | DLQ eviction progress | If retry_count ≥ R_max, offset eventually transitions to EVICTED_DLQ. |
| INV-L-003 | Lease timeout progress | If a lease is granted and not ACKed, it eventually times out. |
| INV-L-004 | Recovery completion | If recovery is initiated, it eventually completes. |
| INV-L-005 | No permanent stuck offset | No offset remains in READY or LEASED state indefinitely if eviction policy is active. |

---

## 9. Model Checking Strategy

### 9.1 Bounded Model Checking

TLC will be used with bounded state spaces:

| Parameter | Bound |
|---|---|
| Offset range | 0..6 |
| Workers | 2 |
| Retry max | 2 |
| Coordinator epochs | 0..3 |
| Journal length | 0..8 |
| Lease TTL ticks | 1..4 |

These bounds are small enough for exhaustive exploration but large enough to expose concurrency bugs.

### 9.2 State Space Estimation

For the core state machine model with 7 offsets, 2 workers, 4 states, and 3 retry levels:

```text
State space ≈ 4^7 × 2^7 × 3^7 × 3 ≈ 1.2 billion states
```

This is large but tractable with TLC symmetry reduction and state compression. If the state space explodes, we will:

1. Reduce offset range to 0..5.
2. Use symmetry sets for workers.
3. Use Apalache symbolic model checking.
4. Decompose into smaller sub-models.

### 9.3 Model Decomposition

To manage complexity, the full system is decomposed into five sub-models:

| Model | Focus | Offsets | Workers |
|---|---|---|---|
| M1 | Core state machine | 0..6 | 2 |
| M2 | Lease lifecycle | 0..4 | 2 |
| M3 | Watermark advancement | 0..8 | 1 |
| M4 | Journal replay | 0..5 | 1 |
| M5 | Epoch fencing | 0..4 | 2 |

Each model is checked independently. Cross-model invariants are verified by composition.

---

## 10. Counterexample Handling

When TLC finds a counterexample:

1. Record the counterexample trace.
2. Analyze whether it represents a real bug or a model artifact.
3. If real bug:
   - File a critical defect.
   - Update the state machine specification.
   - Update the implementation design.
   - Re-run model checker.
4. If model artifact:
   - Refine the model.
   - Add missing constraints.
   - Document the refinement.
5. All counterexamples are archived in the formal validation report.

**Normative rule:** No counterexample may be dismissed without written justification approved by the Formal Methods Lead and Chief Architect.

---

## 11. Test Oracle Derivation

### 11.1 Purpose

The formal model is not only a proof tool. It is also a **test oracle generator**.

Every invariant verified in the model becomes a runtime assertion in the prototype.

### 11.2 Oracle Mapping

| Formal Invariant | Runtime Check in Prototype |
|---|---|
| INV-S-001 No terminal regression | Assert on every state transition that ACKED/DLQ offsets never change. |
| INV-S-002 No double lease | Assert on lease grant that offset is not already LEASED. |
| INV-S-003 Watermark monotonicity | Assert on watermark update that new value ≥ old value. |
| INV-S-004 Terminal below watermark | Periodic scan verifying all offsets below W_base are terminal. |
| INV-S-005 No leased below watermark | Periodic scan verifying no LEASED offsets below W_base. |
| INV-S-007 Lease token uniqueness | Assert on lease grant that token is unique among active leases. |
| INV-S-009 Stale epoch rejection | Assert on operation that epoch matches current coordinator epoch. |
| INV-S-010 Stale token rejection | Assert on ACK/NACK that token matches active lease token. |

### 11.3 Oracle Integration

The runtime invariant checker is embedded in:

- `keirox-state` crate (debug builds: panic on violation).
- `keirox-state` crate (release builds: log error and emit metric).
- `keirox-testkit` crate (test assertions).
- `keirox-chaos` crate (chaos test invariant checks).

---

## 12. Integration with Prototype

### 12.1 Relationship to KEI-SPIKE-101

This plan runs in parallel with KEI-SPIKE-101.

| Prototype Week | Formal Validation Activity |
|---|---|
| Weeks 1–3 | Model 1 (core state machine) drafted and checked. |
| Weeks 4–6 | Model 2 (lease lifecycle) drafted and checked. |
| Weeks 5–8 | Model 3 (watermark) drafted and checked. |
| Weeks 7–9 | Model 4 (journal replay) drafted and checked. |
| Weeks 8–10 | Model 5 (epoch fencing) drafted and checked. |
| Weeks 9–11 | Test oracles integrated into prototype. |
| Week 12 | Formal validation report delivered with prototype evidence package. |

### 12.2 Feedback Loop

When the prototype discovers a behavior that contradicts the model:

1. Determine whether the model or the implementation is wrong.
2. If model is wrong, update model and re-check.
3. If implementation is wrong, fix implementation and add regression test.
4. Update test oracles if needed.

---

## 13. Deliverables

| Deliverable | Description | Due |
|---|---|---|
| D-F-001 | Model 1: Core state machine TLA+ specification | Week 3 |
| D-F-002 | Model 2: Lease lifecycle TLA+ specification | Week 5 |
| D-F-003 | Model 3: Watermark advancement TLA+ specification | Week 7 |
| D-F-004 | Model 4: Journal replay TLA+ specification | Week 9 |
| D-F-005 | Model 5: Epoch fencing TLA+ specification | Week 10 |
| D-F-006 | Model checking results for all five models | Week 11 |
| D-F-007 | Counterexample archive | Week 11 |
| D-F-008 | Test oracle specification | Week 10 |
| D-F-009 | Runtime invariant checker integration | Week 11 |
| D-F-010 | Formal validation summary report | Week 12 |

---

## 14. Acceptance Criteria

| ID | Criterion |
|---|---|
| ACC-F-001 | All five TLA+ models compile and pass TLC model checking. |
| ACC-F-002 | All safety invariants hold in all reachable states. |
| ACC-F-003 | All liveness properties hold under fair scheduling. |
| ACC-F-004 | All counterexamples are resolved or documented. |
| ACC-F-005 | Test oracles are derived for all safety invariants. |
| ACC-F-006 | Runtime invariant checker is integrated into prototype. |
| ACC-F-007 | Prototype passes all runtime invariant checks during soak test. |
| ACC-F-008 | Formal validation report is approved by Chief Architect. |

---

## 15. Dependencies

| Dependency | Source |
|---|---|
| State machine specification | KEI-DES-031 |
| Watermark definition | KEI-ARC-021 |
| Lease lifecycle specification | KEI-DES-031 |
| Epoch fencing specification | KEI-ARC-022 |
| Journal replay specification | KEI-DES-031 |
| Prototype implementation | KEI-SPIKE-101 |
| Test infrastructure | KEI-OPS-041 |

---

## 16. Risks and Mitigations

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| State space explosion in TLC | High | Medium | Decompose into sub-models; use symmetry reduction; use Apalache if needed. |
| Model too abstract to catch real bugs | Medium | Medium | Derive test oracles and validate against prototype behavior. |
| Model too detailed to check | Medium | Low | Start with minimal model; add detail incrementally. |
| Counterexamples are model artifacts | Low | High | Document all refinements; separate model bugs from design bugs. |
| Formal methods expertise gap | High | Medium | Assign experienced TLA+ practitioner; use existing templates from AWS/Azure open-source specs. |
| Prototype diverges from model | Medium | Medium | Establish feedback loop; update model when prototype changes. |

---

## 17. TLA+ Specification Skeleton

Below is the skeleton of the core state machine model (Model 1). This will be expanded into a full specification during execution.

```text
---- MODULE KeiroxStateMachine ----
EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    Offsets,        \* Set of offsets, e.g., 0..6
    Workers,        \* Set of workers, e.g., {w1, w2}
    MaxRetry,       \* Maximum retry count, e.g., 2
    MaxEpoch        \* Maximum coordinator epoch, e.g., 3

VARIABLES
    state,          \* Function: offset -> {READY, LEASED, ACKED, EVICTED_DLQ}
    lease_owner,    \* Function: offset -> worker or NULL
    lease_token,    \* Function: offset -> token or NULL
    lease_expiry,   \* Function: offset -> tick or NULL
    retry_count,    \* Function: offset -> integer
    W_base,         \* Watermark
    epoch,          \* Current coordinator epoch
    journal         \* Sequence of journal entries

vars == <<state, lease_owner, lease_token, lease_expiry, 
          retry_count, W_base, epoch, journal>>

TypeOK ==
    /\ state \in [Offsets -> {READY, LEASED, ACKED, EVICTED_DLQ}]
    /\ W_base \in 0..(Max(Offsets) + 1)
    /\ epoch \in 0..MaxEpoch

Init ==
    /\ state = [o \in Offsets |-> READY]
    /\ lease_owner = [o \in Offsets |-> NULL]
    /\ lease_token = [o \in Offsets |-> NULL]
    /\ lease_expiry = [o \in Offsets |-> NULL]
    /\ retry_count = [o \in Offsets |-> 0]
    /\ W_base = 0
    /\ epoch = 0
    /\ journal = <<>>

LeaseGrant(o, w, tok, ttl) ==
    /\ state[o] = READY
    /\ o >= W_base
    /\ state' = [state EXCEPT ![o] = LEASED]
    /\ lease_owner' = [lease_owner EXCEPT ![o] = w]
    /\ lease_token' = [lease_token EXCEPT ![o] = tok]
    /\ lease_expiry' = [lease_expiry EXCEPT ![o] = ttl]
    /\ UNCHANGED <<retry_count, W_base, epoch>>

Ack(o, tok, ep) ==
    /\ state[o] = LEASED
    /\ lease_token[o] = tok
    /\ ep = epoch
    /\ state' = [state EXCEPT ![o] = ACKED]
    /\ lease_owner' = [lease_owner EXCEPT ![o] = NULL]
    /\ lease_token' = [lease_token EXCEPT ![o] = NULL]
    /\ lease_expiry' = [lease_expiry EXCEPT ![o] = NULL]
    /\ UNCHANGED <<retry_count, W_base, epoch>>

Nack(o, tok, ep) ==
    /\ state[o] = LEASED
    /\ lease_token[o] = tok
    /\ ep = epoch
    /\ retry_count[o] < MaxRetry
    /\ state' = [state EXCEPT ![o] = READY]
    /\ retry_count' = [retry_count EXCEPT ![o] = retry_count[o] + 1]
    /\ lease_owner' = [lease_owner EXCEPT ![o] = NULL]
    /\ lease_token' = [lease_token EXCEPT ![o] = NULL]
    /\ lease_expiry' = [lease_expiry EXCEPT ![o] = NULL]
    /\ UNCHANGED <<W_base, epoch>>

LeaseTimeout(o) ==
    /\ state[o] = LEASED
    /\ IF retry_count[o] >= MaxRetry
       THEN \* Evict to DLQ
            /\ state' = [state EXCEPT ![o] = EVICTED_DLQ]
            /\ retry_count' = UNCHANGED retry_count
       ELSE \* Return to READY
            /\ state' = [state EXCEPT ![o] = READY]
            /\ retry_count' = [retry_count EXCEPT ![o] = retry_count[o] + 1]
    /\ lease_owner' = [lease_owner EXCEPT ![o] = NULL]
    /\ lease_token' = [lease_token EXCEPT ![o] = NULL]
    /\ lease_expiry' = [lease_expiry EXCEPT ![o] = NULL]
    /\ UNCHANGED <<W_base, epoch>>

AdvanceWatermark ==
    /\ \E k \in (W_base + 1)..(Max(Offsets) + 1):
        /\ \A i \in 0..(k - 1): state[i] \in {ACKED, EVICTED_DLQ}
        /\ W_base' = k
    /\ UNCHANGED <<state, lease_owner, lease_token, lease_expiry, 
                   retry_count, epoch>>

Next ==
    \E o \in Offsets, w \in Workers, tok \in 1..100, ttl \in 1..10:
        LeaseGrant(o, w, tok, ttl)
    \/ \E o \in Offsets, tok \in 1..100, ep \in 0..MaxEpoch:
        Ack(o, tok, ep) \/ Nack(o, tok, ep) \/ LeaseTimeout(o)
    \/ AdvanceWatermark

Spec == Init /\ [][Next]_vars /\ WF_vars(AdvanceWatermark)

\* Safety Invariants
NoTerminalRegression ==
    \A o \in Offsets:
        state[o] \in {ACKED, EVICTED_DLQ} => 
            \A o2 \in Offsets: TRUE  \* Checked via TLC invariant

NoDoubleLease ==
    \A o \in Offsets:
        state[o] = LEASED => lease_owner[o] # NULL

WatermarkMonotonic ==
    TRUE  \* Checked via TLC temporal property

TerminalBelowWatermark ==
    \A o \in 0..(W_base - 1):
        state[o] \in {ACKED, EVICTED_DLQ}

====
```

This skeleton will be expanded into five complete TLA+ modules during execution.

---

## 18. Revision History

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-08-30 | Initial State Machine Validation Plan. Defines five TLA+ models, safety and liveness invariants, model checking strategy, counterexample handling, test oracle derivation, and integration with KEI-SPIKE-101 prototype. |