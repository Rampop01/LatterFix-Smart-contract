use soroban_sdk::{contracttype, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending,
    Submitted,
    Approved,
    Rejected,
    Paid,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub task_id: u32,
    pub title: soroban_sdk::String,
    pub amount: i128,
    pub status: MilestoneStatus,
    pub due_date: Option<u64>,
    pub submission_url: Option<soroban_sdk::String>,
    pub feedback: Option<soroban_sdk::String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowStats {
    pub total_locked: i128,
    pub total_released: i128,
    pub total_refunded: i128,
    pub active_escrows: u32,
    pub completed_escrows: u32,
}

#[contracttype]
pub enum EscrowKey {
    Milestone(u32, u32), // (task_id, milestone_id)
    MilestoneCount(u32), // task_id
    EscrowStats,
    TaskEscrow(u32), // task_id -> locked amount
}

pub fn create_milestone(
    env: Env,
    task_id: u32,
    title: soroban_sdk::String,
    amount: i128,
    due_date: Option<u64>,
) -> u32 {
    let count_key = EscrowKey::MilestoneCount(task_id);
    let mut count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    count += 1;
    
    let milestone = Milestone {
        id: count,
        task_id,
        title,
        amount,
        status: MilestoneStatus::Pending,
        due_date,
        submission_url: None,
        feedback: None,
    };
    
    let key = EscrowKey::Milestone(task_id, count);
    env.storage().persistent().set(&key, &milestone);
    env.storage().persistent().set(&count_key, &count);
    
    count
}

pub fn submit_milestone(
    env: Env,
    task_id: u32,
    milestone_id: u32,
    submission_url: soroban_sdk::String,
) {
    let key = EscrowKey::Milestone(task_id, milestone_id);
    let mut milestone: Milestone = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("milestone not found"));
    
    if milestone.status != MilestoneStatus::Pending && milestone.status != MilestoneStatus::Rejected {
        panic!("milestone cannot be submitted");
    }
    
    milestone.status = MilestoneStatus::Submitted;
    milestone.submission_url = Some(submission_url);
    
    env.storage().persistent().set(&key, &milestone);
}

pub fn approve_milestone(
    env: Env,
    task_id: u32,
    milestone_id: u32,
    feedback: Option<soroban_sdk::String>,
) -> i128 {
    let key = EscrowKey::Milestone(task_id, milestone_id);
    let mut milestone: Milestone = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("milestone not found"));
    
    if milestone.status != MilestoneStatus::Submitted {
        panic!("milestone not submitted");
    }
    
    milestone.status = MilestoneStatus::Approved;
    milestone.feedback = feedback;
    
    let amount = milestone.amount;
    
    env.storage().persistent().set(&key, &milestone);
    
    // Update stats
    update_stats(&env, 0, amount, 0, 0, 0);
    
    amount
}

pub fn reject_milestone(
    env: Env,
    task_id: u32,
    milestone_id: u32,
    feedback: soroban_sdk::String,
) {
    let key = EscrowKey::Milestone(task_id, milestone_id);
    let mut milestone: Milestone = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("milestone not found"));
    
    if milestone.status != MilestoneStatus::Submitted {
        panic!("milestone not submitted");
    }
    
    milestone.status = MilestoneStatus::Rejected;
    milestone.feedback = Some(feedback);
    
    env.storage().persistent().set(&key, &milestone);
}

pub fn get_milestone(env: Env, task_id: u32, milestone_id: u32) -> Option<Milestone> {
    let key = EscrowKey::Milestone(task_id, milestone_id);
    env.storage().persistent().get(&key)
}

pub fn get_milestones_for_task(env: Env, task_id: u32) -> Vec<Milestone> {
    let count_key = EscrowKey::MilestoneCount(task_id);
    let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    
    let mut milestones = Vec::new(&env);
    for i in 1..=count {
        if let Some(milestone) = get_milestone(env.clone(), task_id, i) {
            milestones.push_back(milestone);
        }
    }
    
    milestones
}

pub fn update_stats(
    env: &Env,
    locked_delta: i128,
    released_delta: i128,
    refunded_delta: i128,
    active_delta: u32,
    completed_delta: u32,
) {
    let key = EscrowKey::EscrowStats;
    let mut stats: EscrowStats = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(EscrowStats {
            total_locked: 0,
            total_released: 0,
            total_refunded: 0,
            active_escrows: 0,
            completed_escrows: 0,
        });
    
    stats.total_locked += locked_delta;
    stats.total_released += released_delta;
    stats.total_refunded += refunded_delta;
    stats.active_escrows = (stats.active_escrows as i32 + active_delta as i32).max(0) as u32;
    stats.completed_escrows += completed_delta;
    
    env.storage().persistent().set(&key, &stats);
}

pub fn get_escrow_stats(env: Env) -> EscrowStats {
    env.storage()
        .persistent()
        .get(&EscrowKey::EscrowStats)
        .unwrap_or(EscrowStats {
            total_locked: 0,
            total_released: 0,
            total_refunded: 0,
            active_escrows: 0,
            completed_escrows: 0,
        })
}

pub fn lock_escrow(env: Env, task_id: u32, amount: i128) {
    let key = EscrowKey::TaskEscrow(task_id);
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage().persistent().set(&key, &(current + amount));
    update_stats(&env, amount, 0, 0, 1, 0);
}

pub fn release_escrow(env: Env, task_id: u32, amount: i128) {
    let key = EscrowKey::TaskEscrow(task_id);
    let current: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    
    if current < amount {
        panic!("insufficient escrow balance");
    }
    
    let new_balance = current - amount;
    env.storage().persistent().set(&key, &new_balance);
    
    if new_balance == 0 {
        update_stats(&env, 0, amount, 0, 0, 1);
    } else {
        update_stats(&env, 0, amount, 0, 0, 0);
    }
}
