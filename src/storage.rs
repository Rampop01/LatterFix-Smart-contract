use soroban_sdk::{contracttype, Env, String, Vec};

/// Storage helper module for managing contract state
/// Provides type-safe storage operations with TTL management

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageStats {
    pub total_entries: u32,
    pub total_size_bytes: u64,
    pub last_cleanup: u64,
}

#[contracttype]
pub enum StorageKey {
    Metadata,
    TaskList,
    Categories,
    Tags,
    Statistics,
}

/// TTL Configuration for different storage types
/// Persistent storage has a maximum TTL of ~5.2 million ledgers (~31 days)
pub const MAX_PERSISTENT_TTL: u32 = 5_200_000;

/// Calculate optimal TTL based on data type
pub fn calculate_ttl(_env: &Env, is_permanent: bool) -> u32 {
    if is_permanent {
        MAX_PERSISTENT_TTL
    } else {
        // Temporary data - 7 days worth of ledgers (~5 second ledger close time)
        // 7 days * 24 hours * 60 minutes * 60 seconds / 5 seconds ≈ 120,960 ledgers
        120_960
    }
}

/// Category management for tasks
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub task_count: u32,
}

pub fn add_category(env: &Env, name: String, description: String) -> u32 {
    let mut categories: Vec<Category> = env
        .storage()
        .persistent()
        .get(&StorageKey::Categories)
        .unwrap_or_else(|| Vec::new(env));
    
    let id = (categories.len() as u32) + 1;
    
    categories.push_back(Category {
        id,
        name,
        description,
        task_count: 0,
    });
    
    env.storage()
        .persistent()
        .set(&StorageKey::Categories, &categories);
    
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
}

/// Statistics tracking
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
}
