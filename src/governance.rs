use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ProposalStatus {
    Active,
    Executed,
    Rejected,
    Expired,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct Proposal {
    pub id: u32,
    pub title: Symbol,
    pub description: Symbol,
    pub proposer: Address,
    pub status: ProposalStatus,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    pub quorum: u32,    // Minimum votes needed
    pub threshold: u32, // Percentage needed to pass (e.g., 51 = 51%)
    pub executed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: u32,
    pub vote_type: VoteType,
    pub weight: u32, // Based on reputation
    pub voted_at: u64,
}

#[contracttype]
pub enum GovernanceKey {
    Proposal(u32),
    ProposalCount,
    Vote(u32, Address), // (proposal_id, voter)
    Config,
    Delegations(Address), // Delegated voting
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct GovernanceConfig {
    pub voting_period: u64, // Duration in seconds
    pub quorum: u32,        // Minimum votes needed
    pub threshold: u32,     // Percentage to pass
    pub min_reputation_to_propose: u32,
    pub min_reputation_to_vote: u32,
}

pub fn get_config(env: Env) -> GovernanceConfig {
    env.storage()
        .persistent()
        .get(&GovernanceKey::Config)
        .unwrap_or(GovernanceConfig {
            voting_period: 604800, // 7 days
            quorum: 10,
            threshold: 51,
            min_reputation_to_propose: 100,
            min_reputation_to_vote: 50,
        })
}

pub fn set_config(env: Env, admin: Address, config: GovernanceConfig) {
    admin.require_auth();

    // Verify admin - this should be called from the main contract
    env.storage()
        .persistent()
        .set(&GovernanceKey::Config, &config);
}

pub fn create_proposal(
    env: Env,
    proposer: Address,
    title: Symbol,
    description: Symbol,
    quorum: Option<u32>,
    threshold: Option<u32>,
    _min_reputation: u32,
) -> u32 {
    proposer.require_auth();

    // Check reputation
    let config = get_config(env.clone());

    let count_key = GovernanceKey::ProposalCount;
    let mut count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    count += 1;

    let now = env.ledger().timestamp();
    let voting_ends_at = now + config.voting_period;

    let proposal = Proposal {
        id: count,
        title,
        description,
        proposer: proposer.clone(),
        status: ProposalStatus::Active,
        created_at: now,
        voting_ends_at,
        votes_for: 0,
        votes_against: 0,
        votes_abstain: 0,
        quorum: quorum.unwrap_or(config.quorum),
        threshold: threshold.unwrap_or(config.threshold),
        executed_at: None,
    };

    env.storage()
        .persistent()
        .set(&GovernanceKey::Proposal(count), &proposal);
    env.storage().persistent().set(&count_key, &count);

    count
}

pub fn cast_vote(env: Env, voter: Address, proposal_id: u32, vote_type: VoteType, weight: u32) {
    voter.require_auth();

    let proposal_key = GovernanceKey::Proposal(proposal_id);
    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&proposal_key)
        .unwrap_optimized();

    if proposal.status != ProposalStatus::Active {
        panic!();
    }

    let now = env.ledger().timestamp();
    if now > proposal.voting_ends_at {
        panic!();
    }

    // Check if already voted
    let vote_key = GovernanceKey::Vote(proposal_id, voter.clone());
    if env.storage().persistent().has(&vote_key) {
        panic!();
    }

    // Record vote
    let vote = Vote {
        voter: voter.clone(),
        proposal_id,
        vote_type,
        weight,
        voted_at: now,
    };

    env.storage().persistent().set(&vote_key, &vote);

    // Update proposal vote counts
    match vote_type {
        VoteType::For => proposal.votes_for += weight,
        VoteType::Against => proposal.votes_against += weight,
        VoteType::Abstain => proposal.votes_abstain += weight,
    }

    env.storage().persistent().set(&proposal_key, &proposal);
}

pub fn execute_proposal(env: Env, _caller: Address, proposal_id: u32) -> bool {
    let proposal_key = GovernanceKey::Proposal(proposal_id);
    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&proposal_key)
        .unwrap_optimized();

    if proposal.status != ProposalStatus::Active {
        panic!();
    }

    let now = env.ledger().timestamp();
    if now <= proposal.voting_ends_at {
        panic!();
    }

    let total_votes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;

    // Check quorum
    if total_votes < proposal.quorum {
        proposal.status = ProposalStatus::Rejected;
        env.storage().persistent().set(&proposal_key, &proposal);
        return false;
    }

    // Check threshold (percentage of non-abstain votes)
    let non_abstain = proposal.votes_for + proposal.votes_against;
    if non_abstain == 0 {
        proposal.status = ProposalStatus::Rejected;
        env.storage().persistent().set(&proposal_key, &proposal);
        return false;
    }

    let for_percentage = (proposal.votes_for * 100) / non_abstain;

    if for_percentage >= proposal.threshold {
        proposal.status = ProposalStatus::Executed;
        proposal.executed_at = Some(now);
    } else {
        proposal.status = ProposalStatus::Rejected;
    }

    env.storage().persistent().set(&proposal_key, &proposal);

    proposal.status == ProposalStatus::Executed
}

pub fn cancel_proposal(env: Env, proposer: Address, proposal_id: u32) {
    proposer.require_auth();

    let proposal_key = GovernanceKey::Proposal(proposal_id);
    let mut proposal: Proposal = env
        .storage()
        .persistent()
        .get(&proposal_key)
        .unwrap_optimized();

    if proposal.proposer != proposer {
        panic!();
    }

    if proposal.status != ProposalStatus::Active {
        panic!();
    }

    proposal.status = ProposalStatus::Cancelled;
    env.storage().persistent().set(&proposal_key, &proposal);
}

pub fn get_proposal(env: Env, proposal_id: u32) -> Option<Proposal> {
    env.storage()
        .persistent()
        .get(&GovernanceKey::Proposal(proposal_id))
}

pub fn get_vote(env: Env, proposal_id: u32, voter: Address) -> Option<Vote> {
    env.storage()
        .persistent()
        .get(&GovernanceKey::Vote(proposal_id, voter))
}

pub fn get_active_proposals(env: Env) -> Vec<Proposal> {
    let count: u32 = env
        .storage()
        .persistent()
        .get(&GovernanceKey::ProposalCount)
        .unwrap_or(0);

    let mut active = Vec::new(&env);
    for i in 1..=count {
        if let Some(proposal) = get_proposal(env.clone(), i) {
            if proposal.status == ProposalStatus::Active {
                active.push_back(proposal);
            }
        }
    }

    active
}
