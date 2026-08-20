use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

use crate::DataKey;

// On-chain multisig proposal, approval-voting, and execution ledger.
//
// This module governs *admin* transactions — parameter changes and treasury
// fund movements — and is deliberately separate from `governance`, which
// implements reputation-weighted community voting over free-form proposals.
// The two differ in every meaningful dimension:
//
//   - `governance` : anyone with enough reputation may propose/vote, votes are
//                    weighted, outcomes are advisory (no on-chain effect).
//   - `multisig`   : only registered signers may propose/approve, each signer
//                    counts once, and reaching the threshold *performs* the
//                    encoded action against contract state or the treasury.
//
// Status workflow:
//
// ```text
//   Pending ──(threshold reached)──> Approved ──(execute)──> Executed
//      │                                 │
//      └────────────(cancel)─────────────┴──> Cancelled
// ```
//
// Security properties:
//   - The approval threshold is **snapshotted at proposal creation**, so
//     rotating the signer set cannot retroactively make a live proposal
//     easier to pass.
//   - Approvals are **re-validated against the current signer set** at
//     execution time, so an approval from a since-removed signer stops
//     counting.
//   - Each signer may approve a given proposal at most once.
//   - Proposals expire after `proposal_ttl` seconds and can no longer be
//     approved or executed, bounding the window in which a stale approval
//     set stays actionable.
//
// Execution is atomic: if the encoded action traps (e.g. an underfunded
// treasury transfer), the host transaction reverts, including the approval
// that triggered it. The signer may re-approve once the cause is fixed.

// ============================================================================
// Types
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum MultisigProposalStatus {
    Pending = 0,
    Approved = 1,
    Executed = 2,
    Cancelled = 3,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub enum MultisigAction {
    SetPlatformFee(u32),
    SetFeeRecipient(Address),
    SetTokenContract(Address),
    TreasuryTransfer(Address, Address, i128),
    SetSigners(Vec<Address>, u32),
    ResolveDisputeSplit(u32, Vec<Address>, Vec<u32>),
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct MultisigApproval {
    pub signer: Address,
    pub approved_at: u64,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct MultisigProposal {
    pub id: u32,
    pub proposer: Address,
    pub description: Symbol,
    pub action: MultisigAction,
    pub status: MultisigProposalStatus,
    pub approvals: Vec<MultisigApproval>,
    pub threshold: u32,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct MultisigConfig {
    pub signers: Vec<Address>,
    pub threshold: u32,
    pub proposal_ttl: u64,
    pub auto_execute: bool,
}

#[contracttype]
pub enum MultisigKey {
    Config,
    Proposal(u32),
    ProposalCount,
}

pub const DEFAULT_PROPOSAL_TTL: u64 = 604_800;

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
        .unwrap_optimized();
    if *caller != admin {
        panic!();
    }
}

fn validate_signer_set(signers: &Vec<Address>, threshold: u32) {
    let count = signers.len();
    if count == 0 {
        panic!();
    }
    if count > MAX_SIGNERS {
        panic!();
    }
    if threshold == 0 {
        panic!();
    }
    if threshold > count {
        panic!();
    }

    // Reject duplicates: a repeated address would otherwise inflate the
    // effective signer count while contributing only one approval.
    for i in 0..count {
        let signer = signers.get(i).unwrap_optimized();
        for j in (i + 1)..count {
            if signers.get(j).unwrap_optimized() == signer {
                panic!();
            }
        }
    }
}

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

pub fn get_config(env: &Env) -> MultisigConfig {
    env.storage()
        .instance()
        .get(&MultisigKey::Config)
        .unwrap_optimized()
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
        panic!();
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

fn validate_action(env: &Env, action: &MultisigAction) {
    match action {
        MultisigAction::SetPlatformFee(bps) => {
            if *bps > 1000 {
                panic!();
            }
        }
        MultisigAction::TreasuryTransfer(_, _, amount) => {
            if *amount <= 0 {
                panic!();
            }
        }
        MultisigAction::SetSigners(signers, threshold) => {
            validate_signer_set(signers, *threshold);
        }
        MultisigAction::ResolveDisputeSplit(_task_id, recipients, shares_bps) => {
            if recipients.len() == 0 {
                panic!();
            }
            if recipients.len() != shares_bps.len() {
                panic!();
            }
            let mut total_bps: u32 = 0;
            for i in 0..shares_bps.len() {
                let bps = shares_bps.get(i).unwrap_optimized();
                total_bps = total_bps.checked_add(bps).unwrap_optimized();
            }
            if total_bps != 10000 {
                panic!();
            }
        }
        MultisigAction::SetFeeRecipient(_) | MultisigAction::SetTokenContract(_) => {
            let _ = env;
        }
    }
}

pub fn propose(
    env: Env,
    proposer: Address,
    description: Symbol,
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

pub fn vote_proposal(env: Env, signer: Address, proposal_id: u32) -> MultisigProposalStatus {
    signer.require_auth();
    require_signer(&env, &signer);

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_optimized();

    if proposal.status != MultisigProposalStatus::Pending {
        panic!();
    }

    let now = env.ledger().timestamp();
    if now > proposal.expires_at {
        panic!();
    }

    for approval in proposal.approvals.iter() {
        if approval.signer == signer {
            panic!();
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

pub fn execute_proposal(env: Env, caller: Address, proposal_id: u32) -> MultisigProposalStatus {
    caller.require_auth();
    require_signer(&env, &caller);

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_optimized();

    match proposal.status {
        MultisigProposalStatus::Approved => {}
        MultisigProposalStatus::Pending => panic!(),
        MultisigProposalStatus::Executed => panic!(),
        MultisigProposalStatus::Cancelled => panic!(),
    }

    let now = env.ledger().timestamp();
    if now > proposal.expires_at {
        panic!();
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
        panic!();
    }

    apply_action(&env, &proposal.action);

    proposal.status = MultisigProposalStatus::Executed;
    proposal.executed_at = Some(now);
    save_proposal(&env, &proposal);

    proposal.status
}

pub fn cancel_proposal(env: Env, caller: Address, proposal_id: u32) {
    caller.require_auth();

    let mut proposal = get_proposal(&env, proposal_id)
        .unwrap_optimized();

    match proposal.status {
        MultisigProposalStatus::Pending | MultisigProposalStatus::Approved => {}
        MultisigProposalStatus::Executed => panic!(),
        MultisigProposalStatus::Cancelled => panic!(),
    }

    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_optimized();

    if caller != proposal.proposer && caller != admin {
        panic!();
    }

    proposal.status = MultisigProposalStatus::Cancelled;
    save_proposal(&env, &proposal);
}

// ============================================================================
// Execution
// ============================================================================

fn apply_action(env: &Env, action: &MultisigAction) {
    match action {
        MultisigAction::SetPlatformFee(bps) => {
            if *bps > 1000 {
                panic!();
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
                panic!();
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
        MultisigAction::ResolveDisputeSplit(task_id, recipients, shares_bps) => {
            crate::resolve_dispute_split(env.clone(), *task_id, recipients.clone(), shares_bps.clone());
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
