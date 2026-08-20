use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, Address, Env, Map, Symbol, Vec};

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct ReputationEvent {
    pub user: Address,
    pub points: i32,
    pub event_type: ReputationEventType,
    pub timestamp: u64,
    pub reference_id: Option<u32>, // Task ID or other reference
    pub description: Symbol,
}

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReputationEventType {
    TaskCompleted,
    TaskVerified,
    TaskCancelled,
    DisputeWon,
    DisputeLost,
    MilestoneApproved,
    BonusReward,
    Penalty,
    ManualAdjustment,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct ReputationTier {
    pub name: Symbol,
    pub min_points: u32,
    pub max_points: u32,
    pub badge_url: Option<Symbol>,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct UserReputationSummary {
    pub user: Address,
    pub total_points: u32,
    pub tier: Symbol,
    pub tasks_completed: u32,
    pub tasks_verified: u32,
    pub disputes_won: u32,
    pub disputes_lost: u32,
    pub milestones_completed: u32,
    pub total_events: u32,
}

#[contracttype]
pub enum ReputationKey {
    UserPoints(Address),
    UserEvents(Address),
    EventCount(Address),
    ReputationTiers,
    Leaderboard,
}

// Default reputation tiers
pub fn init_reputation_tiers(env: Env) {
    let tiers: Vec<ReputationTier> = {
        let mut v = Vec::new(&env);
        v.push_back(ReputationTier {
            name: Symbol::new(&env, "Newcomer"),
            min_points: 0,
            max_points: 99,
            badge_url: None,
        });
        v.push_back(ReputationTier {
            name: Symbol::new(&env, "Contributor"),
            min_points: 100,
            max_points: 499,
            badge_url: None,
        });
        v.push_back(ReputationTier {
            name: Symbol::new(&env, "Expert"),
            min_points: 500,
            max_points: 999,
            badge_url: None,
        });
        v.push_back(ReputationTier {
            name: Symbol::new(&env, "Master"),
            min_points: 1000,
            max_points: 2499,
            badge_url: None,
        });
        v.push_back(ReputationTier {
            name: Symbol::new(&env, "Legend"),
            min_points: 2500,
            max_points: u32::MAX,
            badge_url: None,
        });
        v
    };

    env.storage()
        .persistent()
        .set(&ReputationKey::ReputationTiers, &tiers);
}

pub fn award_reputation(
    env: Env,
    user: Address,
    points: i32,
    event_type: ReputationEventType,
    reference_id: Option<u32>,
    description: Symbol,
) {
    let key = ReputationKey::UserPoints(user.clone());
    let mut current: i32 = env.storage().persistent().get(&key).unwrap_or(100i32); // Starting reputation

    current = (current + points).max(0); // Floor at 0
    env.storage().persistent().set(&key, &current);

    // Record the event
    let _event = ReputationEvent {
        user: user.clone(),
        points,
        event_type,
        timestamp: env.ledger().timestamp(),
        reference_id,
        description,
    };

    let event_count_key = ReputationKey::EventCount(user.clone());
    let mut event_count: u32 = env
        .storage()
        .persistent()
        .get(&event_count_key)
        .unwrap_or(0);
    event_count += 1;
    env.storage()
        .persistent()
        .set(&event_count_key, &event_count);

    // Update leaderboard
    update_leaderboard(&env, user, current as u32);
}

pub fn get_user_reputation(env: Env, user: Address) -> u32 {
    let key = ReputationKey::UserPoints(user);
    env.storage().persistent().get(&key).unwrap_or(100) as u32
}

pub fn get_user_tier(env: Env, user: Address) -> Symbol {
    let points = get_user_reputation(env.clone(), user);

    let tiers: Vec<ReputationTier> = env
        .storage()
        .persistent()
        .get(&ReputationKey::ReputationTiers)
        .unwrap_or_else(|| {
            // Initialize if not exists
            init_reputation_tiers(env.clone());
            env.storage()
                .persistent()
                .get(&ReputationKey::ReputationTiers)
                .unwrap_optimized()
        });

    for tier in tiers.iter() {
        if points >= tier.min_points && points <= tier.max_points {
            return tier.name;
        }
    }

    Symbol::new(&env, "Unknown")
}

pub fn update_leaderboard(env: &Env, user: Address, points: u32) {
    let key = ReputationKey::Leaderboard;
    let mut leaderboard: Map<Address, u32> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Map::new(env));

    leaderboard.set(user, points);
    env.storage().persistent().set(&key, &leaderboard);
}

pub fn get_leaderboard(env: Env) -> Vec<(Address, u32)> {
    let key = ReputationKey::Leaderboard;
    let leaderboard: Map<Address, u32> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Map::new(&env));

    let mut result = Vec::new(&env);
    for (user, points) in leaderboard.iter() {
        result.push_back((user, points));
    }

    result
}

pub fn get_event_count(env: Env, user: Address) -> u32 {
    let key = ReputationKey::EventCount(user);
    env.storage().persistent().get(&key).unwrap_or(0)
}

// Points awarded for different actions
pub fn points_for_event(event_type: ReputationEventType) -> i32 {
    match event_type {
        ReputationEventType::TaskCompleted => 25,
        ReputationEventType::TaskVerified => 50,
        ReputationEventType::TaskCancelled => -10,
        ReputationEventType::DisputeWon => 30,
        ReputationEventType::DisputeLost => -25,
        ReputationEventType::MilestoneApproved => 15,
        ReputationEventType::BonusReward => 100,
        ReputationEventType::Penalty => -50,
        ReputationEventType::ManualAdjustment => 0, // Custom amount
    }
}
