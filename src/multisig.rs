use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::DataKey;

/// On-chain multisig proposal, approval-voting, and execution ledger.
///
/// This module governs *admin* transactions — parameter changes and treasury
/// fund movements — and is deliberately separate from `governance`, which
/// implements reputation-weighted community voting over free-form proposals.
/// The two differ in every meaningful dimension:
///
///   - `governance` : anyone with enough reputation may propose/vote, votes are
///                    weighted, outcomes are advisory (no on-chain effect).
///   - `multisig`   : only registered signers may propose/approve, each signer
///                    counts once, and reaching the threshold *performs* the
///                    encoded action against contract state or the treasury.
///
/// Status workflow:
///
/// ```text
///   Pending ──(threshold reached)──> Approved ──(execute)──> Executed
///      │                                 │
///      └────────────(cancel)─────────────┴──> Cancelled
/// ```
///
/// Security properties:
///   - The approval threshold is **snapshotted at proposal creation**, so
///     rotating the signer set cannot retroactively make a live proposal
///     easier to pass.
///   - Approvals are **re-validated against the current signer set** at
///     execution time, so an approval from a since-removed signer stops
///     counting.
///   - Each signer may approve a given proposal at most once.
///   - Proposals expire after `proposal_ttl` seconds and can no longer be
///     approved or executed, bounding the window in which a stale approval
///     set stays actionable.
///
/// Execution is atomic: if the encoded action traps (e.g. an underfunded
/// treasury transfer), the host transaction reverts, including the approval
/// that triggered it. The signer may re-approve once the cause is fixed.

// ============================================================================
// Types
// ============================================================================

/// Lifecycle state of a multisig proposal.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MultisigProposalStatus {
    /// Awaiting further approvals.
    Pending = 0,
    /// Threshold reached; the action may be executed.
    Approved = 1,
    /// Action has been applied on-chain. Terminal.
    Executed = 2,
    /// Withdrawn before execution. Terminal.
    Cancelled = 3,
}

/// The state change a proposal will perform once executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultisigAction {
    /// Update the platform fee, in basis points (max 1000 = 10%).
    SetPlatformFee(u32),
    /// Update the address that receives platform fees.
    SetFeeRecipient(Address),
    /// Update the payment token contract.
    SetTokenContract(Address),
    /// Move funds out of the contract treasury: `(token, recipient, amount)`.
    TreasuryTransfer(Address, Address, i128),
    /// Rotate the signer set and threshold: `(signers, threshold)`.
    SetSigners(Vec<Address>, u32),
}

/// A single signer's recorded approval — the on-chain ledger entry proving
/// who approved what, and when.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigApproval {
    pub signer: Address,
    pub approved_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigProposal {
    pub id: u32,
    pub proposer: Address,
    pub description: String,
    pub action: MultisigAction,
    pub status: MultisigProposalStatus,
    /// Full approval ledger, in the order approvals were received.
    pub approvals: Vec<MultisigApproval>,
    /// Approvals required, snapshotted from config at creation time.
    pub threshold: u32,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigConfig {
    pub signers: Vec<Address>,
    /// Number of distinct signer approvals required to execute.
    pub threshold: u32,
    /// Seconds a proposal stays actionable after creation.
    pub proposal_ttl: u64,
    /// Execute immediately when the approving vote reaches the threshold.
    pub auto_execute: bool,
}

#[contracttype]
pub enum MultisigKey {
    Config,
    Proposal(u32),
    ProposalCount,
}

/// Default proposal lifetime: 7 days.
pub const DEFAULT_PROPOSAL_TTL: u64 = 604_800;

/// Upper bound on the signer set, keeping approval re-validation (a linear
/// scan per approval) cheaply bounded.
pub const MAX_SIGNERS: u32 = 20;

// ============================================================================
// Configuration
// ============================================================================

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));
    if *caller != admin {
        panic!("not admin");
    }
}

/// Validate a prospective signer set and threshold, panicking if unusable.
fn validate_signer_set(signers: &Vec<Address>, threshold: u32) {
    let count = signers.len();
    if count == 0 {
        panic!("signer set cannot be empty");
    }
    if count > MAX_SIGNERS {
        panic!("signer set exceeds maximum");
    }
    if threshold == 0 {
        panic!("threshold must be greater than zero");
    }
    if threshold > count {
        panic!("threshold exceeds signer count");
    }

    // Reject duplicates: a repeated address would otherwise inflate the
    // effective signer count while contributing only one approval.
    for i in 0..count {
        let signer = signers.get(i).unwrap();
        for j in (i + 1)..count {
            if signers.get(j).unwrap() == signer {
                panic!("duplicate signer in set");
            }
        }
    }
}

/// Install the multisig signer set. Admin-only; this is the bootstrap step
/// that hands ongoing parameter/treasury authority to the signer group.
pub fn configure(
    env: Env,
    admin: Address,
    signers: Vec<Address>,
    threshold: u32,
    proposal_ttl: Option<u64>,
    auto_execute: Option<bool>,
) {
    require_admin(&env, &admin);
    validate_signer_set(&signers, threshold);

    let config = MultisigConfig {
        signers,
        threshold,
        proposal_ttl: proposal_ttl.unwrap_or(DEFAULT_PROPOSAL_TTL),
        auto_execute: auto_execute.unwrap_or(true),
    };

    env.storage().instance().set(&MultisigKey::Config, &config);
}

/// Read the multisig config, panicking if it was never installed.
pub fn get_config(env: &Env) -> MultisigConfig {
    env.storage()
        .instance()
        .get(&MultisigKey::Config)
        .unwrap_or_else(|| panic!("multisig not configured"))
}

pub fn is_signer(env: &Env, who: &Address) -> bool {
    env.storage()
        .instance()
        .get::<_, MultisigConfig>(&MultisigKey::Config)
        .map(|c| c.signers.contains(who))
        .unwrap_or(false)
}

fn require_signer(env: &Env, who: &Address) {
    if !is_signer(env, who) {
        panic!("not a multisig signer");
    }
}

// ============================================================================
// Proposal lifecycle
// ============================================================================

fn save_proposal(env: &Env, proposal: &MultisigProposal) {
    let key = MultisigKey::Proposal(proposal.id);
    env.storage().persistent().set(&key, proposal);
    crate::storage::extend_persistent_ttl(
        env,
        &key,
        100_000,
        crate::storage::DEFAULT_PERSISTENT_TTL,
    );
}

/// Validate that an action is well-formed before signers spend approvals on
/// it, so a proposal cannot reach threshold only to trap at execution.
fn validate_action(env: &Env, action: &MultisigAction) {
    match action {
        MultisigAction::SetPlatformFee(bps) => {
            if *bps > 1000 {
                panic!("platform fee cannot exceed 10%");
            }
        }
        MultisigAction::TreasuryTransfer(_, _, amount) => {
            if *amount <= 0 {
                panic!("transfer amount must be positive");
            }
        }
        MultisigAction::SetSigners(signers, threshold) => {
            validate_signer_set(signers, *threshold);
        }
        MultisigAction::SetFeeRecipient(_) | MultisigAction::SetTokenContract(_) => {
            let _ = env;
        }
    }
}

/// Create a proposal. Restricted to signers — an outsider should not be able
/// to fill the ledger with proposals the signer set has to triage.
///
/// The proposer is *not* auto-approved: approval is an explicit, separately
/// authorized act, so a 1-of-N misconfiguration cannot silently execute on
/// creation alone.
pub fn propose(
    env: Env,
    proposer: Address,
    description: String,
    action: MultisigAction,
) -> u32 {
    proposer.require_auth();
    require_signer(&env, &proposer);
    validate_action(&env, &action);

    let config = get_config(&env);

    let mut count: u32 = env
        .storage()
        .persistent()
        .get(&MultisigKey::ProposalCount)
        .unwrap_or(0);
    count += 1;

    let now = env.ledger().timestamp();

    let proposal = MultisigProposal {
        id: count,
        proposer,
        description,
        action,
        status: MultisigProposalStatus::Pending,
        approvals: Vec::new(&env),
        threshold: config.threshold,
        created_at: now,
        expires_at: now + config.proposal_ttl,
        executed_at: None,
    };

    save_proposal(&env, &proposal);
    env.storage()
        .persistent()
        .set(&MultisigKey::ProposalCount, &count);

    count
}

/// Record `signer`'s approval of `proposal_id`.
///
/// Returns the proposal's status after the vote. When the threshold is met the
/// proposal moves to `Approved`, and — if `auto_execute` is enabled — the
/// action is applied in the same call, returning `Executed`.
pub fn vote_proposal(env: Env, signer: Address, proposal_id: u32) -> MultisigProposalStatus {
    signer.require_auth();
    require_signer(&env, &signer);

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_or_else(|| panic!("proposal not found"));

    if proposal.status != MultisigProposalStatus::Pending {
        panic!("proposal not pending");
    }

    let now = env.ledger().timestamp();
    if now > proposal.expires_at {
        panic!("proposal expired");
    }

    for approval in proposal.approvals.iter() {
        if approval.signer == signer {
            panic!("signer already approved");
        }
    }

    proposal.approvals.push_back(MultisigApproval {
        signer,
        approved_at: now,
    });

    // Count only approvals from addresses still in the signer set, so a
    // rotation that removed a signer drops their approval too.
    let config = get_config(&env);
    let mut valid: u32 = 0;
    for approval in proposal.approvals.iter() {
        if config.signers.contains(&approval.signer) {
            valid += 1;
        }
    }

    if valid >= proposal.threshold {
        proposal.status = MultisigProposalStatus::Approved;
    }

    save_proposal(&env, &proposal);

    if proposal.status == MultisigProposalStatus::Approved && config.auto_execute {
        apply_action(&env, &proposal.action);
        proposal.status = MultisigProposalStatus::Executed;
        proposal.executed_at = Some(now);
        save_proposal(&env, &proposal);
    }

    proposal.status
}

/// Execute an `Approved` proposal, applying its action on-chain.
///
/// Any signer may trigger execution — the authority came from the approvals
/// already on the ledger, not from the caller. Used when `auto_execute` is
/// disabled, or to retry an execution whose earlier attempt trapped.
pub fn execute_proposal(env: Env, caller: Address, proposal_id: u32) -> MultisigProposalStatus {
    caller.require_auth();
    require_signer(&env, &caller);

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_or_else(|| panic!("proposal not found"));

    match proposal.status {
        MultisigProposalStatus::Approved => {}
        MultisigProposalStatus::Pending => panic!("proposal not yet approved"),
        MultisigProposalStatus::Executed => panic!("proposal already executed"),
        MultisigProposalStatus::Cancelled => panic!("proposal cancelled"),
    }

    let now = env.ledger().timestamp();
    if now > proposal.expires_at {
        panic!("proposal expired");
    }

    // Re-check against the live signer set: approvals from signers removed
    // since the vote must not carry the proposal over the threshold.
    let config = get_config(&env);
    let mut valid: u32 = 0;
    for approval in proposal.approvals.iter() {
        if config.signers.contains(&approval.signer) {
            valid += 1;
        }
    }
    if valid < proposal.threshold {
        panic!("approvals no longer meet threshold");
    }

    apply_action(&env, &proposal.action);

    proposal.status = MultisigProposalStatus::Executed;
    proposal.executed_at = Some(now);
    save_proposal(&env, &proposal);

    proposal.status
}

/// Cancel a proposal before execution. Allowed for the original proposer or
/// the contract admin.
pub fn cancel_proposal(env: Env, caller: Address, proposal_id: u32) {
    caller.require_auth();

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_or_else(|| panic!("proposal not found"));

    match proposal.status {
        MultisigProposalStatus::Pending | MultisigProposalStatus::Approved => {}
        MultisigProposalStatus::Executed => panic!("proposal already executed"),
        MultisigProposalStatus::Cancelled => panic!("proposal already cancelled"),
    }

    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));

    if caller != proposal.proposer && caller != admin {
        panic!("only proposer or admin can cancel");
    }

    proposal.status = MultisigProposalStatus::Cancelled;
    save_proposal(&env, &proposal);
}

// ============================================================================
// Execution
// ============================================================================

/// Apply a proposal's encoded action to contract state.
///
/// Called only after threshold approval has been verified. Traps on failure,
/// reverting the enclosing transaction.
fn apply_action(env: &Env, action: &MultisigAction) {
    match action {
        MultisigAction::SetPlatformFee(bps) => {
            if *bps > 1000 {
                panic!("platform fee cannot exceed 10%");
            }
            env.storage().instance().set(&DataKey::PlatformFeeBps, bps);
        }
        MultisigAction::SetFeeRecipient(recipient) => {
            env.storage().instance().set(&DataKey::FeeRecipient, recipient);
        }
        MultisigAction::SetTokenContract(token) => {
            env.storage().instance().set(&DataKey::TokenContract, token);
        }
        MultisigAction::TreasuryTransfer(token, recipient, amount) => {
            if *amount <= 0 {
                panic!("transfer amount must be positive");
            }
            soroban_sdk::token::Client::new(env, token).transfer(
                &env.current_contract_address(),
                recipient,
                amount,
            );
        }
        MultisigAction::SetSigners(signers, threshold) => {
            validate_signer_set(signers, *threshold);
            let mut config = get_config(env);
            config.signers = signers.clone();
            config.threshold = *threshold;
            env.storage().instance().set(&MultisigKey::Config, &config);
        }
    }
}

// ============================================================================
// Views
// ============================================================================

pub fn get_proposal(env: &Env, proposal_id: u32) -> Option<MultisigProposal> {
    env.storage()
        .persistent()
        .get(&MultisigKey::Proposal(proposal_id))
}

pub fn get_proposal_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&MultisigKey::ProposalCount)
        .unwrap_or(0)
}

/// Number of approvals on `proposal_id` that are still backed by a current
/// signer — the figure compared against the threshold.
pub fn get_approval_count(env: &Env, proposal_id: u32) -> u32 {
    let proposal = match get_proposal(env, proposal_id) {
        Some(p) => p,
        None => return 0,
    };
    let config = get_config(env);
    let mut valid: u32 = 0;
    for approval in proposal.approvals.iter() {
        if config.signers.contains(&approval.signer) {
            valid += 1;
        }
    }
    valid
}

pub fn has_approved(env: &Env, proposal_id: u32, signer: &Address) -> bool {
    match get_proposal(env, proposal_id) {
        Some(p) => p.approvals.iter().any(|a| a.signer == *signer),
        None => false,
    }
}

/// All proposals still awaiting approvals and not yet expired.
pub fn get_pending_proposals(env: &Env) -> Vec<MultisigProposal> {
    let count = get_proposal_count(env);
    let now = env.ledger().timestamp();
    let mut pending = Vec::new(env);

    for i in 1..=count {
        if let Some(proposal) = get_proposal(env, i) {
            if proposal.status == MultisigProposalStatus::Pending && now <= proposal.expires_at {
                pending.push_back(proposal);
            }
        }
    }

    pending
}
