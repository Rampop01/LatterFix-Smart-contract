# Changelog — LatterFix Smart Contract

All notable changes to the Soroban smart contract are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — 2026-07-23

### Added
- **`multisig.rs`** — On-chain multisig proposal, approval-voting, and execution ledger for privileged admin transactions (#043). Distinct from `governance.rs`, which remains reputation-weighted community voting with no on-chain effect.
  - Status workflow: `Pending → Approved → Executed`, with `Cancelled` reachable from either non-terminal state.
  - `MultisigAction` encodes what a proposal performs: `SetPlatformFee`, `SetFeeRecipient`, `SetTokenContract`, `TreasuryTransfer`, and `SetSigners` (signer-set rotation).
  - Approval threshold is snapshotted at proposal creation, so rotating the signer set cannot retroactively lower the bar for a live proposal.
  - Approvals are re-validated against the current signer set at execution, so an approval from a since-removed signer stops counting.
  - Proposals expire after a configurable TTL (default 7 days); one approval per signer; actions are validated at proposal time so signers never spend approvals on a proposal that could only trap at execution.
  - New endpoints: `configure_multisig`, `multisig_propose`, `vote_proposal`, `multisig_execute_proposal`, `multisig_cancel_proposal`, plus views `get_multisig_proposal`, `get_multisig_config`, `get_multisig_approval_count`, `has_approved_proposal`, `get_pending_multisig_proposals`, `is_multisig_signer`.
  - The executor is exported as `multisig_execute_proposal` rather than `execute_proposal`, since the latter is already bound to `governance::execute_proposal`; renaming it would break existing clients.
- **`events.rs`** — Five multisig event emitters on dedicated `ms_*` topics, so off-chain indexers can separate privileged admin transactions from community proposals: `ms_cfg`, `ms_prop`, `ms_vote`, `ms_exec`, `ms_cancl`.
- **`multisig_test.rs`** — 24 tests covering the proposal lifecycle: configuration validation, threshold execution, treasury movements, signer rotation, stale-approval discounting, threshold snapshotting, cancellation, and expiry.

### Added
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
