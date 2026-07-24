//! Storage helper module for the LatterFix TaskManager contract.
//!
//! Centralises all persistent storage keys, TTL management, and statistic
//! tracking so that every module reads/writes through a single typed interface.
//!
//! Storage tiers used in this contract:
//!   - `persistent()` — survives ledger archival; requires TTL extension
//!   - `temporary()` — cheap, auto-expires after TTL; used for nonces/sessions
//!   - `instance()`  — scoped to the contract instance; used for admin config

use soroban_sdk::{contracttype, Env, String, Vec};

// ── TTL Constants ──────────────────────────────────────────────────────────
/// Maximum persistent TTL: ~31 days at 5-second ledger close time.
pub const MAX_PERSISTENT_TTL: u32 = 5_200_000;

/// Default TTL for persistent user/task data: ~14 days.
pub const DEFAULT_PERSISTENT_TTL: u32 = 2_073_600;

/// Short-lived TTL for temporary session data: ~7 days.
pub const TEMP_SESSION_TTL: u32 = 120_960;

// ── Storage Key Enum ───────────────────────────────────────────────────────

#[contracttype]
pub enum StorageKey {
    Metadata,
    TaskList,
    Categories,
    Tags,
    Statistics,
}

// ── TTL Helpers ────────────────────────────────────────────────────────────

/// Calculate optimal TTL based on whether data is permanent.
pub fn calculate_ttl(_env: &Env, is_permanent: bool) -> u32 {
    if is_permanent {
        MAX_PERSISTENT_TTL
    } else {
        TEMP_SESSION_TTL
    }
}

/// Extend TTL for a persistent storage entry if it is below the threshold.
/// Call this after every write to prevent unexpected archival.
///
/// * `key`       — the storage key to extend
/// * `threshold` — minimum remaining ledgers before extension triggers
/// * `extend_to` — target TTL to extend to (in ledgers)
pub fn extend_persistent_ttl<K>(env: &Env, key: &K, threshold: u32, extend_to: u32)
where
    K: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    env.storage()
        .persistent()
        .extend_ttl(key, threshold, extend_to);
}

/// Extend TTL for all core contract statistics on every state change.
/// Prevents the statistics storage entry from being archived mid-operation.
pub fn refresh_statistics_ttl(env: &Env) {
    env.storage()
        .persistent()
        .extend_ttl(&StorageKey::Statistics, 100_000, DEFAULT_PERSISTENT_TTL);
}

/// Extend TTL for the categories index after any write.
pub fn refresh_categories_ttl(env: &Env) {
    env.storage()
        .persistent()
        .extend_ttl(&StorageKey::Categories, 100_000, DEFAULT_PERSISTENT_TTL);
}

// ── Storage Metadata ───────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageStats {
    pub total_entries: u32,
    pub total_size_bytes: u64,
    pub last_cleanup: u64,
}

// ── Category Management ────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub task_count: u32,
    pub created_at: u64,
}

pub fn add_category(env: &Env, name: String, description: String) -> u32 {
    let mut categories: Vec<Category> = env
        .storage()
        .persistent()
        .get(&StorageKey::Categories)
        .unwrap_or_else(|| Vec::new(env));

    let id = (categories.len()) + 1;

    categories.push_back(Category {
        id,
        name,
        description,
        task_count: 0,
        created_at: env.ledger().timestamp(),
    });

    env.storage()
        .persistent()
        .set(&StorageKey::Categories, &categories);

    refresh_categories_ttl(env);
    id
}

pub fn get_categories(env: &Env) -> Vec<Category> {
    env.storage()
        .persistent()
        .get(&StorageKey::Categories)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn increment_category_task_count(env: &Env, category_id: u32) {
    let mut categories: Vec<Category> = env
        .storage()
        .persistent()
        .get(&StorageKey::Categories)
        .unwrap_or_else(|| Vec::new(env));

    for i in 0..categories.len() {
        let mut category = categories.get(i).unwrap();
        if category.id == category_id {
            category.task_count += 1;
            categories.set(i, category);
            break;
        }
    }

    env.storage()
        .persistent()
        .set(&StorageKey::Categories, &categories);

    refresh_categories_ttl(env);
}

// ── Statistics Tracking ────────────────────────────────────────────────────

/// Aggregate statistics stored in persistent storage.
/// Updated atomically on every contract state change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ContractStatistics {
    pub total_tasks_created: u32,
    pub total_tasks_completed: u32,
    pub total_tasks_cancelled: u32,
    pub total_tasks_disputed: u32,
    pub total_value_locked: i128,
    pub total_value_paid: i128,
    pub total_platform_fees: i128,
    pub unique_users: u32,
    pub total_milestones: u32,
}

pub fn get_statistics(env: &Env) -> ContractStatistics {
    env.storage()
        .persistent()
        .get(&StorageKey::Statistics)
        .unwrap_or_default()
}

pub fn update_statistics<F>(env: &Env, updater: F)
where
    F: FnOnce(&mut ContractStatistics),
{
    let mut stats = get_statistics(env);
    updater(&mut stats);
    env.storage()
        .persistent()
        .set(&StorageKey::Statistics, &stats);
    refresh_statistics_ttl(env);
}
