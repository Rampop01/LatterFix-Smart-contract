# Changelog — LatterFix Smart Contract

All notable changes to the Soroban smart contract are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — 2026-07-13

### Added
- **`zkp_attestation.rs`** — Zero-Knowledge Proof (ZKP) identity attestation module (issue #27). Lets employees prove eligibility / KYC status during payroll without revealing identity data on-chain:
  - Admin-gated verification-key registry keyed by circuit id, with structural (Groth16 `IC` arity, BLS12-381 point encoding) validation of proof payloads.
  - A `verify_groth16_pairing` host-binding wrapper isolating the pairing-equation seam, ready to delegate to the BLS12-381 pairing host function (CAP-0059) on Protocol 22.
  - Nullifier tracking with a spent-nullifier set to reject ZK proof replays, plus a `H(nullifier || public_signals)` commitment binding that prevents lifting a valid proof onto a different nullifier.
  - `test.rs` — 9 unit tests covering valid attestation, malformed/zero proofs, public-signal arity mismatch, commitment tampering, nullifier replay, and admin-only key registration.

- **`events.rs`** — All 22 event emitter functions now include `env.ledger().timestamp()` in their data tuple, enabling precise off-chain temporal indexing via Soroban RPC `getEvents`. Two new platform-level events added:
  - `emit_fee_updated` — fires when admin changes platform fee basis points
  - `emit_contract_initialized` — fires once on first `initialize()` call
- **`test.rs`** — Expanded from 5 to 8 test cases:
  - Shared `setup_initialized_contract()` helper to reduce boilerplate
  - `test_dispute_full_assignee_payout` — admin awards 100% of escrowed funds to assignee; verifies 0% fee at 0 bps configuration
  - `test_multiple_concurrent_tasks` — two simultaneous tasks created, assigned, submitted, and completed; validates aggregate fee collection and independent escrow balances
  - `test_cannot_double_assign` — verifies contract rejects a second `assign_task()` call on an already-assigned task

### Changed
- **`events.rs`** — Timestamp added to all existing event data tuples for: `task_created`, `task_assigned`, `task_submitted`, `task_completed`, `task_cancelled`, `task_disputed`, `dispute_resolved`, `profile_created`, `profile_updated`, `reputation_awarded`, all milestone events, all governance events, role events, pause events, and token transfer events.

---

## [0.2.0] — 2026-07-11

### Added
- Full modular contract architecture:
  - `lib.rs` — Main contract entrypoint with 20+ public methods
  - `escrow.rs` — Escrow state machine (Open → Assigned → Disputed → Completed/Cancelled)
  - `governance.rs` — On-chain proposals, voting (For/Against/Abstain), execution
  - `reputation.rs` — Tiered reputation system (Newcomer → Contributor → Expert → Master → Legend) with leaderboard
  - `access_control.rs` — Role-based access (Admin / Moderator / Verifier) with `require_auth()`
  - `pausable.rs` — Granular per-action and global pause/unpause
  - `user_profile.rs` — On-chain developer profiles with bio and stats
  - `storage.rs` — Centralized persistent storage key management
  - `events.rs` — Structured event emission for all state transitions

---

## [0.1.0] — 2026-07-10

### Added
- Initial monolithic `lib.rs` with task creation, assignment, completion, and cancellation.
- Basic escrow via Soroban token client.
- Unit tests using `soroban_sdk::testutils`.
