#![no_std]

pub mod user_profile;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

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
}

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
}

#[contract]
pub struct TaskManagerContract;

#[contractimpl]
impl TaskManagerContract {
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
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::PlatformFeeBps, &platform_fee_bps);
        env.storage().instance().set(&DataKey::TokenContract, &token_contract);
        env.storage().instance().set(&DataKey::FeeRecipient, &fee_recipient);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::TaskCount, &0u32);
    }

    pub fn create_task(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        reward: i128,
        tags: Vec<String>,
    ) -> u32 {
        creator.require_auth();

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

        let mut task_count: u32 = env.storage().instance().get(&DataKey::TaskCount).unwrap_or(0);
        task_count += 1;
        env.storage().instance().set(&DataKey::TaskCount, &task_count);

        let task = Task {
            id: task_count,
            title,
            description,
            reward,
            assignee: None,
            status: TaskStatus::Open,
            created_by: creator,
            tags,
        };

        env.storage().instance().set(&DataKey::Task(task_count), &task);

        task_count
    }

    pub fn assign_task(env: Env, assignee: Address, task_id: u32) {
        assignee.require_auth();

        let mut task: Task = env
            .storage()
            .instance()
            .get(&DataKey::Task(task_id))
            .unwrap_or_else(|| panic!("task not found"));

        if task.status != TaskStatus::Open {
            panic!("task is not open");
        }

        task.assignee = Some(assignee);
        task.status = TaskStatus::InProgress;

        env.storage().instance().set(&DataKey::Task(task_id), &task);
    }

    pub fn submit_work(env: Env, assignee: Address, task_id: u32, _delivery_url: String) {
        assignee.require_auth();

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

        env.storage().instance().set(&DataKey::Task(task_id), &task);
    }

    pub fn complete_task(env: Env, caller: Address, task_id: u32) {
        caller.require_auth();

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

        let platform_fee_bps: u32 = env.storage().instance().get(&DataKey::PlatformFeeBps).unwrap_or(0);
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

        task.status = TaskStatus::Verified;
        env.storage().instance().set(&DataKey::Task(task_id), &task);
    }

    pub fn cancel_task(env: Env, creator: Address, task_id: u32) {
        creator.require_auth();

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

        task.status = TaskStatus::Cancelled;
        env.storage().instance().set(&DataKey::Task(task_id), &task);
    }

    pub fn dispute_task(env: Env, caller: Address, task_id: u32) {
        caller.require_auth();

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
        env.storage().instance().set(&DataKey::Task(task_id), &task);
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
        }

        task.status = TaskStatus::Resolved;
        env.storage().instance().set(&DataKey::Task(task_id), &task);
    }

    pub fn create_profile(env: Env, user: Address, username: String, bio: String) {
        user_profile::create_profile(env, user, username, bio);
    }

    pub fn update_bio(env: Env, user: Address, new_bio: String) {
        user_profile::update_bio(env, user, new_bio);
    }

    pub fn reward_contribution(env: Env, admin: Address, user: Address, points: u32) {
        user_profile::reward_contribution(env, admin, user, points);
    }

    pub fn get_profile(env: Env, user: Address) -> Option<user_profile::UserProfile> {
        user_profile::get_profile(env, user)
    }
}
