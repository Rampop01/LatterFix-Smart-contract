//! Timelocked contract-WASM upgrade proposal, veto, and rollback ledger.
//!
//! Every upgrade of this contract's underlying WASM code must pass through a
//! single mandatory pending window before it can take effect, giving users
//! and the emergency guardian time to inspect the proposed code and react.
//!
//! Status workflow:
//!
//! ```text
//!   Pending ──(timelock elapses, admin executes)──> Executed
//!      │
//!      └──────────(guardian or admin vetoes)───────> Vetoed
//! ```
//!
//! Only one upgrade proposal is tracked at a time — a second `propose_upgrade`
//! call is rejected while one is still `Pending`. This mirrors how the WASM
//! hash itself works: the contract only ever has one "next" version in
//! flight, so a single-slot design keeps the state machine (and its
//! authorization story) simple without losing any of the acceptance
//! criteria's requirements.
//!
//! ### Rollback semantics
//!
//! This module does not implement a separate "rollback" entry point. Instead,
//! every hash the contract is *actually* upgraded to is appended to an
//! append-only history log (`get_upgrade_history`). To roll back, the admin
//! simply calls `propose_upgrade` again with a previously recorded hash and
//! takes it through the exact same timelock + (optional) guardian-veto flow
//! as any other upgrade. This was chosen deliberately over a distinct
//! `rollback_upgrade` bypass: a rollback is just as capable of reintroducing
//! a bug or being abused as a forward upgrade, so it gets no less scrutiny —
//! the timelock and guardian veto apply equally. The history log's job is
//! purely to make past hashes discoverable so the admin/guardian know what to
//! propose back to; it grants no special authority of its own.
//!
//! ### Guardian role
//!
//! Rather than introducing a parallel role system, the emergency guardian is
//! modeled as a new `access_control::Role::Guardian` value on top of the
//! contract's existing role registry (`grant_role` / `revoke_role` /
//! `has_role` in `access_control.rs`). Guardians are installed the same way
//! any other role is: `grant_role(admin, guardian_address, Role::Guardian)`.
//! `veto_upgrade` accepts either the contract admin (so an operator can
//! self-correct a mistaken proposal without waiting on a third party) or any
//! address holding `Role::Guardian` (the actual emergency-stop path against a
//! malicious or compromised admin-issued proposal).

use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

use crate::{access_control, DataKey};

// ============================================================================
// Types
// ============================================================================

/// Lifecycle state of the (single) pending upgrade proposal slot.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum UpgradeStatus {
    /// Proposed and timelocked; not yet executable.
    Pending = 0,
    /// Timelock elapsed and the WASM swap was applied. Terminal.
    Executed = 1,
    /// Cancelled by the admin or an emergency guardian before execution. Terminal.
    Vetoed = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    pub wasm_hash: BytesN<32>,
    pub proposed_by: Address,
    pub proposed_at: u64,
    /// Ledger timestamp (seconds) at or after which `execute_upgrade` may
    /// succeed.
    pub ready_at: u64,
    pub status: UpgradeStatus,
    pub executed_at: Option<u64>,
    pub vetoed_by: Option<Address>,
}

/// A single entry in the append-only log of WASM hashes this contract has
/// actually been upgraded to. Used to support emergency rollback: the admin
/// can look up a previous hash here and re-propose it through the normal
/// timelock flow.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeHistoryEntry {
    pub wasm_hash: BytesN<32>,
    pub applied_at: u64,
    pub applied_by: Address,
}

#[contracttype]
pub enum UpgradeKey {
    /// The single in-flight (or most recently resolved) upgrade proposal.
    Pending,
    /// Configured timelock delay, in seconds. Falls back to
    /// `DEFAULT_TIMELOCK_SECONDS` when unset.
    TimelockSeconds,
    /// Append-only `Vec<UpgradeHistoryEntry>` of applied upgrades.
    History,
}

/// Default mandatory pending window: 48 hours.
pub const DEFAULT_TIMELOCK_SECONDS: u64 = 172_800;

/// Safety floor for the configurable timelock: 1 hour. Prevents the admin
/// from configuring the delay down to (near) zero and defeating the whole
/// point of a mandatory inspection window.
pub const MIN_TIMELOCK_SECONDS: u64 = 3_600;

/// Upper bound on retained history entries, keeping the log's storage
/// footprint bounded on a long-lived contract.
pub const MAX_HISTORY_LEN: u32 = 50;

// ============================================================================
// Authorization helpers
// ============================================================================

fn stored_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"))
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != stored_admin(env) {
        panic!("not admin");
    }
}

/// Gate for `veto_upgrade`: the contract admin, or any address holding
/// `access_control::Role::Guardian`.
fn require_guardian_or_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller == stored_admin(env) {
        return;
    }
    if !access_control::has_role(env.clone(), caller.clone(), access_control::Role::Guardian) {
        panic!("not an emergency guardian");
    }
}

// ============================================================================
// Timelock configuration
// ============================================================================

/// Currently configured timelock delay, in seconds.
pub fn get_timelock_seconds(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&UpgradeKey::TimelockSeconds)
        .unwrap_or(DEFAULT_TIMELOCK_SECONDS)
}

/// Reconfigure the timelock delay. Admin-only; rejects values below the
/// `MIN_TIMELOCK_SECONDS` safety floor.
pub fn set_timelock_seconds(env: Env, admin: Address, seconds: u64) -> u64 {
    require_admin(&env, &admin);
    if seconds < MIN_TIMELOCK_SECONDS {
        panic!("timelock below minimum safety floor");
    }
    env.storage()
        .instance()
        .set(&UpgradeKey::TimelockSeconds, &seconds);
    seconds
}

// ============================================================================
// Proposal lifecycle
// ============================================================================

/// Propose upgrading the contract to `new_wasm_hash`. Admin-only.
///
/// Stores the proposal in the `Pending` state with `ready_at` set to now plus
/// the configured timelock. Rejected while another proposal is still
/// `Pending` — resolve it (execute or veto) first.
pub fn propose_upgrade(env: Env, admin: Address, new_wasm_hash: BytesN<32>) -> UpgradeProposal {
    require_admin(&env, &admin);

    if let Some(existing) = get_pending_upgrade(&env) {
        if existing.status == UpgradeStatus::Pending {
            panic!("an upgrade proposal is already pending");
        }
    }

    let now = env.ledger().timestamp();
    let ready_at = now + get_timelock_seconds(&env);

    let proposal = UpgradeProposal {
        wasm_hash: new_wasm_hash,
        proposed_by: admin,
        proposed_at: now,
        ready_at,
        status: UpgradeStatus::Pending,
        executed_at: None,
        vetoed_by: None,
    };

    env.storage()
        .instance()
        .set(&UpgradeKey::Pending, &proposal);
    proposal
}

/// Cancel the currently pending upgrade proposal before its timelock elapses.
/// Callable by the contract admin (self-correction) or any address holding
/// `Role::Guardian` (emergency stop). No effect can be reversed once a
/// proposal is `Executed`.
pub fn veto_upgrade(env: Env, guardian: Address) -> UpgradeProposal {
    require_guardian_or_admin(&env, &guardian);

    let mut proposal = get_pending_upgrade(&env).unwrap_or_else(|| panic!("no pending upgrade"));

    if proposal.status != UpgradeStatus::Pending {
        panic!("upgrade proposal is not pending");
    }

    proposal.status = UpgradeStatus::Vetoed;
    proposal.vetoed_by = Some(guardian);
    env.storage()
        .instance()
        .set(&UpgradeKey::Pending, &proposal);
    proposal
}

/// Execute the pending upgrade proposal, swapping the contract's live WASM.
/// Admin-only; reverts if the timelock has not yet elapsed, or if the
/// proposal was already executed or vetoed.
///
/// Records the applied hash in the rollback history log before performing
/// the swap, then invokes `env.deployer().update_current_contract_wasm`.
pub fn execute_upgrade(env: Env, caller: Address) -> BytesN<32> {
    require_admin(&env, &caller);

    let mut proposal = get_pending_upgrade(&env).unwrap_or_else(|| panic!("no pending upgrade"));

    match proposal.status {
        UpgradeStatus::Pending => {}
        UpgradeStatus::Executed => panic!("upgrade proposal already executed"),
        UpgradeStatus::Vetoed => panic!("upgrade proposal was vetoed"),
    }

    let now = env.ledger().timestamp();
    if now < proposal.ready_at {
        panic!("timelock has not elapsed");
    }

    // Record the outgoing hash in history before swapping code, so the log
    // always reflects hashes that were actually applied on-chain.
    push_history(&env, proposal.wasm_hash.clone(), caller.clone(), now);

    env.deployer()
        .update_current_contract_wasm(proposal.wasm_hash.clone());

    proposal.status = UpgradeStatus::Executed;
    proposal.executed_at = Some(now);
    env.storage()
        .instance()
        .set(&UpgradeKey::Pending, &proposal);

    proposal.wasm_hash
}

// ============================================================================
// History
// ============================================================================

fn push_history(env: &Env, wasm_hash: BytesN<32>, applied_by: Address, applied_at: u64) {
    let mut history: Vec<UpgradeHistoryEntry> = env
        .storage()
        .persistent()
        .get(&UpgradeKey::History)
        .unwrap_or_else(|| Vec::new(env));

    history.push_back(UpgradeHistoryEntry {
        wasm_hash,
        applied_at,
        applied_by,
    });

    // Bound storage growth: keep only the most recent MAX_HISTORY_LEN entries.
    if history.len() > MAX_HISTORY_LEN {
        let drop = history.len() - MAX_HISTORY_LEN;
        let mut trimmed = Vec::new(env);
        for i in drop..history.len() {
            trimmed.push_back(history.get(i).unwrap());
        }
        history = trimmed;
    }

    env.storage()
        .persistent()
        .set(&UpgradeKey::History, &history);
    crate::storage::extend_persistent_ttl(
        env,
        &UpgradeKey::History,
        100_000,
        crate::storage::DEFAULT_PERSISTENT_TTL,
    );
}

// ============================================================================
// Views
// ============================================================================

pub fn get_pending_upgrade(env: &Env) -> Option<UpgradeProposal> {
    env.storage().instance().get(&UpgradeKey::Pending)
}

/// Append-only log of WASM hashes this contract has actually been upgraded
/// to, oldest first (subject to `MAX_HISTORY_LEN` truncation).
pub fn get_upgrade_history(env: &Env) -> Vec<UpgradeHistoryEntry> {
    env.storage()
        .persistent()
        .get(&UpgradeKey::History)
        .unwrap_or_else(|| Vec::new(env))
}
