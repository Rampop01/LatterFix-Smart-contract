use crate::storage::DEFAULT_PERSISTENT_TTL;
use crate::DataKey;
use soroban_sdk::{contracttype, Address, Env, String};

/// On-chain developer profile stored in persistent ledger storage.
///
/// Profiles are created via `create_profile()` and updated through dedicated
/// mutator functions, each requiring `require_auth()` on the owning address.
/// The admin can award or penalise reputation via `reward_contribution()` and
/// `slash_reputation()` respectively.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserProfile {
    /// The Stellar G-address that owns this profile.
    pub address: Address,
    /// Human-readable display name (stored on-chain).
    pub username: String,
    /// Reputation score; starts at 100 and changes with task outcomes.
    pub reputation: u32,
    /// Total tasks verified and completed by this contributor.
    pub completed_tasks: u32,
    /// Ledger timestamp of when the profile was first created.
    pub joined_at: u64,
    /// Free-text bio/description (max enforced by SDK string limit).
    pub bio: String,
    /// Optional IPFS/HTTPS avatar URL.
    pub avatar_url: Option<String>,
    /// Cumulative earnings in the contract's token (smallest denomination).
    pub total_earnings: i128,
    /// Ledger timestamp of the last profile mutation.
    pub last_updated: u64,
}

// ── Write helpers ──────────────────────────────────────────────────────────

/// Create a new on-chain profile. Panics if one already exists for `user`.
pub fn create_profile(env: Env, user: Address, username: String, bio: String) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    if env.storage().persistent().has(&key) {
        panic!("profile already exists");
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

/// Update the bio of an existing profile. Requires auth from the profile owner.
pub fn update_bio(env: Env, user: Address, new_bio: String) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("profile not found"));

    profile.bio = new_bio;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

/// Update the display username of an existing profile. Requires owner auth.
pub fn update_username(env: Env, user: Address, new_username: String) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("profile not found"));

    profile.username = new_username;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

/// Set or update the avatar URL. Requires owner auth.
pub fn update_avatar(env: Env, user: Address, avatar_url: String) {
    user.require_auth();

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("profile not found"));

    profile.avatar_url = Some(avatar_url);
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

/// Award reputation points and increment completed_tasks counter.
/// Also records cumulative earnings for the contributor.
/// Requires admin auth.
pub fn reward_contribution(env: Env, admin: Address, user: Address, points: u32) {
    admin.require_auth();

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored_admin {
        panic!("not admin");
    }

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("profile not found"));

    profile.reputation = profile.reputation.saturating_add(points);
    profile.completed_tasks += 1;
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

/// Record token earnings for a contributor after a successful task payout.
/// Called internally by the escrow module — no auth required (internal only).
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

/// Slash reputation as a penalty (e.g. unjustified dispute or deadline miss).
/// Reputation is floored at 0 using `saturating_sub`. Requires admin auth.
pub fn slash_reputation(env: Env, admin: Address, user: Address, penalty: u32) {
    admin.require_auth();

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored_admin {
        panic!("not admin");
    }

    let key = DataKey::UserProfile(user.clone());
    let mut profile: UserProfile = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic!("profile not found"));

    profile.reputation = profile.reputation.saturating_sub(penalty);
    profile.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(&key, &profile);
    env.storage()
        .persistent()
        .extend_ttl(&key, 100_000, DEFAULT_PERSISTENT_TTL);
}

// ── Read helpers ───────────────────────────────────────────────────────────

/// Fetch the full on-chain profile for a given address. Returns `None` if
/// no profile has been created yet.
pub fn get_profile(env: Env, user: Address) -> Option<UserProfile> {
    let key = DataKey::UserProfile(user);
    env.storage().persistent().get(&key)
}
