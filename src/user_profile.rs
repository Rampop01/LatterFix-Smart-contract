use soroban_sdk::unwrap::UnwrapOptimized;
use crate::storage::DEFAULT_PERSISTENT_TTL;
use crate::DataKey;
use soroban_sdk::{contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct UserProfile {
    pub address: Address,
    pub username: Symbol,
    pub reputation: u32,
    pub completed_tasks: u32,
    pub joined_at: u64,
    pub bio: Symbol,
    pub avatar_url: Option<Symbol>,
    pub total_earnings: i128,
    pub last_updated: u64,
}

// ── Write helpers ──────────────────────────────────────────────────────────

pub fn create_profile(env: Env, user: Address, username: Symbol, bio: Symbol) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    if env.storage().persistent().has(&key) {
        panic!();
    }

    let profile = UserProfile {
        address: user.clone(),
        username,
        reputation: 100,
        completed_tasks: 0,
        joined_at: env.ledger().timestamp(),
        bio,
        avatar_url: None,
        total_earnings: 0,
        last_updated: env.ledger().timestamp(),
    };

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

pub fn update_bio(env: Env, user: Address, new_bio: Symbol) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_optimized();

    profile.bio = new_bio;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

pub fn update_username(env: Env, user: Address, new_username: Symbol) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_optimized();

    profile.username = new_username;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

pub fn update_avatar(env: Env, user: Address, avatar_url: Symbol) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_optimized();

    profile.avatar_url = Some(avatar_url);
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

pub fn reward_contribution(env: Env, admin: Address, user: Address, points: u32) {
    admin.require_auth();

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored_admin {
        panic!();
    }

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_optimized();

    profile.reputation = profile.reputation.saturating_add(points);
    profile.completed_tasks += 1;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

pub fn record_earnings(env: &Env, user: &Address, amount: i128) {
    let key = DataKey::UserProfile(user.clone());
    if let Some(mut profile) = env.storage().persistent().get::<DataKey, UserProfile>(&key) {
        profile.total_earnings = profile.total_earnings.saturating_add(amount);
        profile.last_updated = env.ledger().timestamp();
        env.storage().persistent().set(&key, &profile);
        env.storage()
            .persistent()
            .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
    }
}

pub fn slash_reputation(env: Env, admin: Address, user: Address, penalty: u32) {
    admin.require_auth();

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored_admin {
        panic!();
    }

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_optimized();

    profile.reputation = profile.reputation.saturating_sub(penalty);
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

// ── Read helpers ───────────────────────────────────────────────────────────

pub fn get_profile(env: Env, user: Address) -> Option<UserProfile> {
    let key = DataKey::UserProfile(user);
    env.storage().persistent().get(&key)
}
