#![no_std]

pub mod access_control;
pub mod escrow;
pub mod events;
pub mod governance;
pub mod multisig;
pub mod pausable;
pub mod reputation;
pub mod storage;
pub mod swap_router;
pub mod user_profile;
pub mod vault;
pub mod merkle;

#[cfg(test)]
mod test;
#[cfg(test)]
mod multisig_test;
#[cfg(test)]
mod swap_router_test;

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

// ============================================================================
// Task Management Types
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TaskStatus {
    Open = 0,
    InProgress = 1,
    Completed = 2,
    Verified = 3,
    Cancelled = 4,
    Disputed = 5,
    Resolved = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub reward: i128,
    pub assignee: Option<Address>,
    pub status: TaskStatus,
    pub created_by: Address,
    pub tags: Vec<String>,
    pub category_id: Option<u32>,
    pub deadline: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskWithMilestones {
    pub task: Task,
    pub milestones: Vec<escrow::Milestone>,
    pub total_milestone_amount: i128,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    PlatformFeeBps,
    TokenContract,
    FeeRecipient,
    Initialized,
    Task(u32),
    TaskCount,
    UserProfile(Address),
    Paused(u32),
    GovernanceConfig,
}

// ============================================================================
// Main Contract
// ============================================================================

#[contract]
pub struct TaskManagerContract;

#[contractimpl]
impl TaskManagerContract {
    // ========================================================================
    // Initialization
    // ========================================================================
    
    pub fn initialize(
        env: Env,
        admin: Address,
        platform_fee_bps: u32,
        token_contract: Address,
        fee_recipient: Address,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }
        
        // Validate fee (max 10%)
        if platform_fee_bps > 1000 {
            panic!("platform fee cannot exceed 10%");
        }
        
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PlatformFeeBps, &platform_fee_bps);
        env.storage().instance().set(&DataKey::TokenContract, &token_contract);
        env.storage().instance().set(&DataKey::FeeRecipient, &fee_recipient);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::TaskCount, &0u32);
        
        // Initialize reputation tiers
        reputation::init_reputation_tiers(env.clone());
        
        // Initialize governance config
        env.storage().persistent().set(
            &governance::GovernanceKey::Config,
            &governance::GovernanceConfig {
                voting_period: 604800, // 7 days
                quorum: 10,
                threshold: 51,
                min_reputation_to_propose: 100,
                min_reputation_to_vote: 50,
            },
        );
    }

    // ========================================================================
    // Task Management
    // ========================================================================
    
    pub fn create_task(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        reward: i128,
        tags: Vec<String>,
    ) -> u32 {
        creator.require_auth();
        
        // Check if paused
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::CreateTask,
            Some(creator.clone()),
        );
        
        if reward <= 0 {
            panic!("reward must be positive");
        }
        
        let token_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::TokenContract)
            .unwrap_or_else(|| panic!("not initialized"));
        
        // Transfer reward from creator to the contract
        let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
        token_client.transfer(&creator, &env.current_contract_address(), &reward);
        
        let mut task_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0);
        task_count += 1;
        env.storage().instance().set(&DataKey::TaskCount, &task_count);
        
        let now = env.ledger().timestamp();
        
        let task = Task {
            id: task_count,
            title: title.clone(),
            description,
            reward,
            assignee: None,
            status: TaskStatus::Open,
            created_by: creator.clone(),
            tags,
            category_id: None,
            deadline: None,
            created_at: now,
            updated_at: now,
        };
        
        env.storage().instance().set(&DataKey::Task(task_count), &task);
        
        // Lock escrow
        escrow::lock_escrow(env.clone(), task_count, reward);
        
        // Emit event
        events::emit_task_created(&env, task_count, creator, title, reward);
        
        // Update statistics
        storage::update_statistics(&env, |stats| {
            stats.total_tasks_created += 1;
            stats.total_value_locked += reward;
        });
        
        task_count
    }
    
    pub fn create_task_with_milestones(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        milestones: Vec<(String, i128)>, // (title, amount)
        tags: Vec<String>,
    ) -> u32 {
        creator.require_auth();
        
        // Calculate total reward from milestones
        let mut total_reward: i128 = 0;
        for i in 0..milestones.len() {
            let milestone = milestones.get(i).unwrap();
            total_reward += milestone.1;
        }
        
        if total_reward <= 0 {
            panic!("total milestone amount must be positive");
        }
        
        // Create the task
        let task_id = Self::create_task(
            env.clone(),
            creator.clone(),
            title,
            description,
            total_reward,
            tags,
        );
        
        // Create milestones
        for (milestone_title, amount) in milestones.iter() {
            escrow::create_milestone(
                env.clone(),
                task_id,
                milestone_title.clone(),
                amount,
                None,
            );
        }
        
        task_id
    }

    pub fn assign_task(env: Env, assignee: Address, task_id: u32) {
        assignee.require_auth();
        
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::AssignTask,
            Some(assignee.clone()),
        );
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.status != TaskStatus::Open {
            panic!("task is not open");
        }
        
        task.assignee = Some(assignee.clone());
        task.status = TaskStatus::InProgress;
        task.updated_at = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        events::emit_task_assigned(&env, task_id, assignee);
    }

    pub fn submit_work(env: Env, assignee: Address, task_id: u32, delivery_url: String) {
        assignee.require_auth();
        
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::SubmitWork,
            Some(assignee.clone()),
        );
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.assignee.as_ref() != Some(&assignee) {
            panic!("caller is not the assignee");
        }
        
        if task.status != TaskStatus::InProgress {
            panic!("task is not in progress");
        }
        
        task.status = TaskStatus::Completed;
        task.updated_at = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        events::emit_task_submitted(&env, task_id, assignee, delivery_url);
    }

    pub fn complete_task(env: Env, caller: Address, task_id: u32) {
        caller.require_auth();
        
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::CompleteTask,
            Some(caller.clone()),
        );
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.status != TaskStatus::Completed {
            panic!("task is not completed");
        }
        
        // Verify caller is creator or admin
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != task.created_by && caller != admin {
            panic!("not authorized to complete task");
        }
        
        let platform_fee_bps: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PlatformFeeBps)
            .unwrap_or(0);
        let fee_recipient: Address = env.storage().instance().get(&DataKey::FeeRecipient).unwrap();
        let token_contract: Address = env.storage().instance().get(&DataKey::TokenContract).unwrap();
        
        let fee = (task.reward * platform_fee_bps as i128) / 10000;
        let payout = task.reward - fee;
        
        let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
        
        let assignee = task.assignee.clone().unwrap_or_else(|| panic!("no assignee"));
        
        if fee > 0 {
            token_client.transfer(&env.current_contract_address(), &fee_recipient, &fee);
        }
        if payout > 0 {
            token_client.transfer(&env.current_contract_address(), &assignee, &payout);
        }
        
        // Release escrow
        escrow::release_escrow(env.clone(), task_id, task.reward);
        
        task.status = TaskStatus::Verified;
        task.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        // Award reputation
        reputation::award_reputation(
            env.clone(),
            assignee.clone(),
            reputation::points_for_event(reputation::ReputationEventType::TaskVerified),
            reputation::ReputationEventType::TaskVerified,
            Some(task_id),
            String::from_str(&env, "Task verified and completed"),
        );
        
        events::emit_task_completed(&env, task_id, assignee, payout, fee);
        
        // Update statistics
        storage::update_statistics(&env, |stats| {
            stats.total_tasks_completed += 1;
            stats.total_value_paid += payout;
            stats.total_platform_fees += fee;
        });
    }

    pub fn cancel_task(env: Env, creator: Address, task_id: u32) {
        creator.require_auth();
        
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::CancelTask,
            Some(creator.clone()),
        );
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.created_by != creator {
            panic!("not task creator");
        }
        
        if task.status != TaskStatus::Open {
            panic!("task is not open");
        }
        
        let token_contract: Address = env.storage().instance().get(&DataKey::TokenContract).unwrap();
        let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
        token_client.transfer(&env.current_contract_address(), &creator, &task.reward);
        
        // Release escrow (refund)
        escrow::release_escrow(env.clone(), task_id, task.reward);
        
        task.status = TaskStatus::Cancelled;
        task.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        events::emit_task_cancelled(&env, task_id, creator, task.reward);
        
        // Update statistics
        storage::update_statistics(&env, |stats| {
            stats.total_tasks_cancelled += 1;
        });
    }

    pub fn dispute_task(env: Env, caller: Address, task_id: u32) {
        caller.require_auth();
        
        pausable::require_not_paused(
            env.clone(),
            pausable::PauseAction::DisputeTask,
            Some(caller.clone()),
        );
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if caller != task.created_by && Some(&caller) != task.assignee.as_ref() {
            panic!("not authorized to dispute task");
        }
        
        if task.status != TaskStatus::InProgress && task.status != TaskStatus::Completed {
            panic!("task status cannot be disputed");
        }
        
        task.status = TaskStatus::Disputed;
        task.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        events::emit_task_disputed(&env, task_id, caller);
        
        // Update statistics
        storage::update_statistics(&env, |stats| {
            stats.total_tasks_disputed += 1;
        });
    }

    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        task_id: u32,
        creator_refund: i128,
        assignee_payout: i128,
    ) {
        admin.require_auth();
        
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("not admin");
        }
        
        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.status != TaskStatus::Disputed {
            panic!("task is not disputed");
        }
        
        if creator_refund + assignee_payout != task.reward {
            panic!("invalid split totals");
        }
        
        let token_contract: Address = env.storage().instance().get(&DataKey::TokenContract).unwrap();
        let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
        
        if creator_refund > 0 {
            token_client.transfer(&env.current_contract_address(), &task.created_by, &creator_refund);
        }
        if assignee_payout > 0 {
            let assignee = task.assignee.clone().unwrap_or_else(|| panic!("no assignee"));
            token_client.transfer(&env.current_contract_address(), &assignee, &assignee_payout);
            
            // Award reputation for winning dispute
            reputation::award_reputation(
                env.clone(),
                assignee.clone(),
                reputation::points_for_event(reputation::ReputationEventType::DisputeWon),
                reputation::ReputationEventType::DisputeWon,
                Some(task_id),
                String::from_str(&env, "Won dispute"),
            );
        }
        
        // Release escrow
        escrow::release_escrow(env.clone(), task_id, task.reward);
        
        task.status = TaskStatus::Resolved;
        task.updated_at = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Task(task_id), &task);
        
        events::emit_dispute_resolved(&env, task_id, creator_refund, assignee_payout);
    }
    
    // ========================================================================
    // Milestone Management
    // ========================================================================
    
    pub fn submit_milestone(
        env: Env,
        assignee: Address,
        task_id: u32,
        milestone_id: u32,
        submission_url: String,
    ) {
        assignee.require_auth();
        
        // Verify assignee
        let task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        if task.assignee.as_ref() != Some(&assignee) {
            panic!("not the assignee");
        }
        
        escrow::submit_milestone(env.clone(), task_id, milestone_id, submission_url);
        
        events::emit_milestone_submitted(&env, task_id, milestone_id, assignee);
    }
    
    pub fn approve_milestone(
        env: Env,
        caller: Address,
        task_id: u32,
        milestone_id: u32,
        feedback: Option<String>,
    ) {
        caller.require_auth();
        
        let task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        // Only creator or admin can approve
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != task.created_by && caller != admin {
            panic!("not authorized");
        }
        
        let amount = escrow::approve_milestone(env.clone(), task_id, milestone_id, feedback.clone());
        
        // Transfer milestone payment to assignee
        let token_contract: Address = env.storage().instance().get(&DataKey::TokenContract).unwrap();
        let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
        let assignee = task.assignee.clone().unwrap();
        
        token_client.transfer(&env.current_contract_address(), &assignee, &amount);
        
        events::emit_milestone_approved(&env, task_id, milestone_id, amount);
        
        // Award reputation
        reputation::award_reputation(
            env.clone(),
            assignee.clone(),
            reputation::points_for_event(reputation::ReputationEventType::MilestoneApproved),
            reputation::ReputationEventType::MilestoneApproved,
            Some(task_id),
            String::from_str(&env, "Milestone approved"),
        );
    }
    
    pub fn reject_milestone(
        env: Env,
        caller: Address,
        task_id: u32,
        milestone_id: u32,
        feedback: String,
    ) {
        caller.require_auth();
        
        let task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));
        
        // Only creator or admin can reject
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != task.created_by && caller != admin {
            panic!("not authorized");
        }
        
        escrow::reject_milestone(env.clone(), task_id, milestone_id, feedback.clone());
        
        events::emit_milestone_rejected(&env, task_id, milestone_id, feedback);
    }
    
    pub fn get_milestones(env: Env, task_id: u32) -> Vec<escrow::Milestone> {
        escrow::get_milestones_for_task(env, task_id)
    }

    // ========================================================================
    // User Profile Management
    // ========================================================================

    pub fn create_profile(env: Env, user: Address, username: String, bio: String) {
        user_profile::create_profile(env.clone(), user.clone(), username.clone(), bio.clone());
        events::emit_profile_created(&env, user, username);
    }

    pub fn update_bio(env: Env, user: Address, new_bio: String) {
        user_profile::update_bio(env.clone(), user.clone(), new_bio.clone());
        events::emit_profile_updated(&env, user, String::from_str(&env, "bio"));
    }

    pub fn reward_contribution(env: Env, admin: Address, user: Address, points: u32) {
        user_profile::reward_contribution(env.clone(), admin, user.clone(), points);
        
        let new_total = reputation::get_user_reputation(env.clone(), user.clone());
        events::emit_reputation_awarded(&env, user, points, new_total);
    }
    
    pub fn get_profile(env: Env, user: Address) -> Option<user_profile::UserProfile> {
        user_profile::get_profile(env, user)
    }
    
    // ========================================================================
    // Reputation System
    // ========================================================================
    
    pub fn get_user_reputation(env: Env, user: Address) -> u32 {
        reputation::get_user_reputation(env, user)
    }
    
    pub fn get_user_tier(env: Env, user: Address) -> String {
        reputation::get_user_tier(env, user)
    }
    
    pub fn get_leaderboard(env: Env) -> Vec<(Address, u32)> {
        reputation::get_leaderboard(env)
    }

    // ========================================================================
    // Access Control
    // ========================================================================
    
    pub fn grant_role(env: Env, admin: Address, user: Address, role: access_control::Role) {
        access_control::grant_role(env.clone(), admin.clone(), user.clone(), role.clone());
        events::emit_role_granted(&env, user, format_role(&env, &role), admin);
    }
    
    pub fn revoke_role(env: Env, admin: Address, user: Address) {
        let role_data = access_control::get_role(env.clone(), user.clone());
        let role_name = role_data
            .as_ref()
            .map(|r| format_role(&env, &r.role))
            .unwrap_or_else(|| String::from_str(&env, "none"));
        
        access_control::revoke_role(env.clone(), admin.clone(), user.clone());
        events::emit_role_revoked(&env, user, role_name, admin);
    }
    
    pub fn has_role(env: Env, user: Address, role: access_control::Role) -> bool {
        access_control::has_role(env, user, role)
    }

    // ========================================================================
    // Governance
    // ========================================================================
    
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        title: String,
        description: String,
    ) -> u32 {
        let config = governance::get_config(env.clone());
        
        let proposal_id = governance::create_proposal(
            env.clone(),
            proposer.clone(),
            title.clone(),
            description,
            Some(config.quorum),
            Some(config.threshold),
            config.min_reputation_to_propose,
        );
        
        events::emit_proposal_created(&env, proposal_id, proposer, title);
        proposal_id
    }
    
    pub fn cast_vote(
        env: Env,
        voter: Address,
        proposal_id: u32,
        vote_type: governance::VoteType,
    ) {
        let weight = reputation::get_user_reputation(env.clone(), voter.clone());
        
        governance::cast_vote(
            env.clone(),
            voter.clone(),
            proposal_id,
            vote_type,
            weight,
        );
        
        let vote_str = match vote_type {
            governance::VoteType::For => String::from_str(&env, "for"),
            governance::VoteType::Against => String::from_str(&env, "against"),
            governance::VoteType::Abstain => String::from_str(&env, "abstain"),
        };
        
        events::emit_vote_cast(&env, proposal_id, voter, vote_str, weight);
    }
    
    pub fn execute_proposal(env: Env, caller: Address, proposal_id: u32) -> bool {
        let passed = governance::execute_proposal(env.clone(), caller, proposal_id);
        events::emit_proposal_executed(&env, proposal_id, passed);
        passed
    }
    
    pub fn get_proposal(env: Env, proposal_id: u32) -> Option<governance::Proposal> {
        governance::get_proposal(env, proposal_id)
    }
    
    pub fn get_active_proposals(env: Env) -> Vec<governance::Proposal> {
        governance::get_active_proposals(env)
    }

    // ========================================================================
    // Admin Multisig
    // ========================================================================
    //
    // Note: these endpoints are prefixed `multisig_*` rather than taking the
    // bare `create_proposal` / `execute_proposal` names, which are already
    // exported above by the reputation-weighted governance module. The
    // approval-vote entry point keeps the unprefixed `vote_proposal` name,
    // which was free.

    /// Install the multisig signer set and approval threshold. Admin-only.
    pub fn configure_multisig(
        env: Env,
        admin: Address,
        signers: Vec<Address>,
        threshold: u32,
        proposal_ttl: Option<u64>,
        auto_execute: Option<bool>,
    ) {
        let signer_count = signers.len();
        multisig::configure(
            env.clone(),
            admin.clone(),
            signers,
            threshold,
            proposal_ttl,
            auto_execute,
        );
        events::emit_multisig_configured(&env, admin, signer_count, threshold);
    }

    /// Propose an admin parameter change or treasury movement. Signer-only.
    pub fn multisig_propose(
        env: Env,
        proposer: Address,
        description: String,
        action: multisig::MultisigAction,
    ) -> u32 {
        let proposal_id = multisig::propose(
            env.clone(),
            proposer.clone(),
            description.clone(),
            action,
        );

        let threshold = multisig::get_config(&env).threshold;
        events::emit_multisig_proposed(&env, proposal_id, proposer, description, threshold);

        proposal_id
    }

    /// Record an approval vote. Executes the proposal in the same call when
    /// this vote reaches the threshold and `auto_execute` is enabled.
    pub fn vote_proposal(
        env: Env,
        signer: Address,
        proposal_id: u32,
    ) -> multisig::MultisigProposalStatus {
        let status = multisig::vote_proposal(env.clone(), signer.clone(), proposal_id);

        let approvals = multisig::get_approval_count(&env, proposal_id);
        let threshold = multisig::get_config(&env).threshold;
        events::emit_multisig_approved(&env, proposal_id, signer.clone(), approvals, threshold);

        if status == multisig::MultisigProposalStatus::Executed {
            events::emit_multisig_executed(&env, proposal_id, signer);
        }

        status
    }

    /// Execute an already-approved proposal. Signer-only.
    pub fn multisig_execute_proposal(
        env: Env,
        caller: Address,
        proposal_id: u32,
    ) -> multisig::MultisigProposalStatus {
        let status = multisig::execute_proposal(env.clone(), caller.clone(), proposal_id);
        events::emit_multisig_executed(&env, proposal_id, caller);
        status
    }

    /// Cancel a proposal before execution. Proposer or admin only.
    pub fn multisig_cancel_proposal(env: Env, caller: Address, proposal_id: u32) {
        multisig::cancel_proposal(env.clone(), caller.clone(), proposal_id);
        events::emit_multisig_cancelled(&env, proposal_id, caller);
    }

    pub fn get_multisig_proposal(
        env: Env,
        proposal_id: u32,
    ) -> Option<multisig::MultisigProposal> {
        multisig::get_proposal(&env, proposal_id)
    }

    pub fn get_multisig_config(env: Env) -> multisig::MultisigConfig {
        multisig::get_config(&env)
    }

    pub fn get_multisig_approval_count(env: Env, proposal_id: u32) -> u32 {
        multisig::get_approval_count(&env, proposal_id)
    }

    pub fn has_approved_proposal(env: Env, proposal_id: u32, signer: Address) -> bool {
        multisig::has_approved(&env, proposal_id, &signer)
    }

    pub fn get_pending_multisig_proposals(env: Env) -> Vec<multisig::MultisigProposal> {
        multisig::get_pending_proposals(&env)
    }

    pub fn is_multisig_signer(env: Env, who: Address) -> bool {
        multisig::is_signer(&env, &who)
    }

    // ========================================================================
    // Pause Control
    // ========================================================================
    
    pub fn pause(env: Env, admin: Address, action: pausable::PauseAction) {
        pausable::pause(env.clone(), admin.clone(), action);
        events::emit_paused(&env, format_pause_action(&env, action), admin);
    }
    
    pub fn unpause(env: Env, admin: Address, action: pausable::PauseAction) {
        pausable::unpause(env.clone(), admin.clone(), action);
        events::emit_unpaused(&env, format_pause_action(&env, action), admin);
    }
    
    pub fn pause_all(env: Env, admin: Address) {
        pausable::pause_all(env.clone(), admin.clone());
        events::emit_paused(&env, String::from_str(&env, "all"), admin);
    }
    
    pub fn unpause_all(env: Env, admin: Address) {
        pausable::unpause_all(env.clone(), admin.clone());
        events::emit_unpaused(&env, String::from_str(&env, "all"), admin);
    }

    // ========================================================================
    // Multi-Asset Swap Router
    // ========================================================================

    pub fn configure_swap_router(
        env: Env,
        admin: Address,
        oracle: Address,
        max_hops: u32,
        default_slippage_bps: u32,
    ) {
        swap_router::configure(env.clone(), admin.clone(), oracle.clone(), max_hops, default_slippage_bps);
        events::emit_router_configured(&env, admin, oracle, max_hops, default_slippage_bps);
    }

    pub fn add_approved_stablecoin(env: Env, admin: Address, stablecoin: Address) {
        swap_router::add_approved_stablecoin(env.clone(), admin.clone(), stablecoin.clone());
        events::emit_stablecoin_approved(&env, admin, stablecoin);
    }

    pub fn remove_approved_stablecoin(env: Env, admin: Address, stablecoin: Address) {
        swap_router::remove_approved_stablecoin(env.clone(), admin.clone(), stablecoin.clone());
        events::emit_stablecoin_removed(&env, admin, stablecoin);
    }

    pub fn get_approved_stablecoins(env: Env) -> Vec<Address> {
        swap_router::get_approved_stablecoins(env)
    }

    pub fn get_swap_router_config(env: Env) -> swap_router::RouterConfig {
        swap_router::get_config(env)
    }

    /// Convert an incoming non-standard SAC token into an approved vault
    /// stablecoin via a (possibly multi-hop) DEX route, guarded by an
    /// oracle-derived minimum-return check. Refunds (rejects without pulling
    /// funds) if the route can't be resolved or has no oracle price.
    pub fn convert_incoming_deposit(
        env: Env,
        sender: Address,
        token_in: Address,
        amount_in: i128,
        route: swap_router::SwapRoute,
        slippage_bps: Option<u32>,
    ) -> swap_router::ConversionOutcome {
        let outcome = swap_router::convert_incoming_deposit(
            env.clone(),
            sender.clone(),
            token_in.clone(),
            amount_in,
            route,
            slippage_bps,
        );

        match &outcome {
            swap_router::ConversionOutcome::Converted(token_out, amount_out) => {
                events::emit_swap_executed(&env, sender, token_in, token_out.clone(), amount_in, *amount_out);
            }
            swap_router::ConversionOutcome::Refunded(reason) => {
                events::emit_swap_refunded(&env, sender, token_in, amount_in, reason.clone());
            }
        }

        outcome
    }

    pub fn get_vault_balance(env: Env, owner: Address, stablecoin: Address) -> i128 {
        swap_router::get_vault_balance(env, owner, stablecoin)
    }

    pub fn withdraw_stablecoin(env: Env, owner: Address, stablecoin: Address, amount: i128) {
        swap_router::withdraw_stablecoin(env, owner, stablecoin, amount);
    }

    pub fn get_swap_router_stats(env: Env) -> swap_router::SwapRouterStats {
        swap_router::get_stats(env)
    }

    // ========================================================================
    // Statistics & Views
    // ========================================================================

    pub fn get_task(env: Env, task_id: u32) -> Option<Task> {
        env.storage().instance().get(&DataKey::Task(task_id))
    }
    
    pub fn get_escrow_stats(env: Env) -> escrow::EscrowStats {
        escrow::get_escrow_stats(env)
    }
    
    pub fn get_statistics(env: Env) -> storage::ContractStatistics {
        storage::get_statistics(&env)
    }

    // ========================================================================
    // Multi-Stablecoin Vault
    // ========================================================================

    /// Register a SAC token address as an accepted vault currency. Admin only.
    pub fn add_supported_token(env: Env, admin: Address, token: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("not initialized"));
        if admin != stored_admin {
            panic!("only admin can add supported tokens");
        }

        vault::add_supported_token(&env, token.clone());
        events::emit_token_supported(&env, token, admin);
    }

    /// Deregister a SAC token address from the accepted vault currencies. Admin only.
    pub fn remove_supported_token(env: Env, admin: Address, token: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("not initialized"));
        if admin != stored_admin {
            panic!("only admin can remove supported tokens");
        }

        vault::remove_supported_token(&env, token.clone());
        events::emit_token_unsupported(&env, token, admin);
    }

    pub fn is_token_supported(env: Env, token: Address) -> bool {
        vault::is_supported_token(&env, &token)
    }

    pub fn get_supported_tokens(env: Env) -> Vec<Address> {
        vault::get_supported_tokens(&env)
    }

    /// Deposit `amount` of `token` into the vault. Token must already be supported.
    pub fn deposit_to_vault(env: Env, depositor: Address, token: Address, amount: i128) {
        depositor.require_auth();
        vault::deposit(&env, depositor.clone(), token.clone(), amount);
        events::emit_vault_deposit(&env, depositor, token, amount);
    }

    /// Claim `amount` of `token` out of the vault, drawing down the caller's
    /// depositor balance for that specific token.
    pub fn claim_from_vault(env: Env, claimant: Address, token: Address, amount: i128) {
        claimant.require_auth();
        vault::claim(&env, claimant.clone(), token.clone(), amount);
        events::emit_vault_claim(&env, claimant, token, amount);
    }

    pub fn get_token_vault_balance(env: Env, token: Address) -> i128 {
        vault::get_vault_balance(&env, token)
    }

    pub fn get_depositor_vault_balance(env: Env, depositor: Address, token: Address) -> i128 {
        vault::get_depositor_balance(&env, depositor, token)
    }

    // ========================================================================
    // Merkle Payroll (Vault)
    // ========================================================================

    pub fn set_payroll_root(env: Env, admin: Address, payroll_id: u32, root: soroban_sdk::BytesN<32>) {
        admin.require_auth();
        
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("not initialized"));
            
        if admin != stored_admin {
            panic!("only admin can set payroll root");
        }
        
        vault::set_payroll_root(&env, payroll_id, root);
    }

    pub fn claim_payroll(
        env: Env,
        claimant: Address,
        token: Address,
        payroll_id: u32,
        amount: i128,
        proof: Vec<soroban_sdk::BytesN<32>>,
    ) {
        claimant.require_auth();
        vault::claim_payroll(&env, claimant, token, payroll_id, amount, proof);
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_role(env: &Env, role: &access_control::Role) -> String {
    match role {
        access_control::Role::Admin => String::from_str(env, "Admin"),
        access_control::Role::Manager => String::from_str(env, "Manager"),
        access_control::Role::Moderator => String::from_str(env, "Moderator"),
        access_control::Role::Verifier => String::from_str(env, "Verifier"),
    }
}

fn format_pause_action(env: &Env, action: pausable::PauseAction) -> String {
    match action {
        pausable::PauseAction::CreateTask => String::from_str(env, "create_task"),
        pausable::PauseAction::AssignTask => String::from_str(env, "assign_task"),
        pausable::PauseAction::SubmitWork => String::from_str(env, "submit_work"),
        pausable::PauseAction::CompleteTask => String::from_str(env, "complete_task"),
        pausable::PauseAction::CancelTask => String::from_str(env, "cancel_task"),
        pausable::PauseAction::DisputeTask => String::from_str(env, "dispute_task"),
        pausable::PauseAction::Withdraw => String::from_str(env, "withdraw"),
        pausable::PauseAction::All => String::from_str(env, "all"),
    }
}
