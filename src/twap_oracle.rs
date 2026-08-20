use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, Env, Symbol, Vec};


// ──────────────────────────────────────────────────────────────────────────
// Data Types
// ──────────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct PriceObservation {
    pub timestamp: u64,
    pub cumulative_price: i128,
    pub price: i128,
    pub ledger_sequence: u32,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct TwapConfig {
    pub primary_pool: Symbol,
    pub secondary_oracle: Option<Symbol>,
    pub min_observation_count: u32,
    pub max_deviation_bps: u32,
    pub observation_window_secs: u64,
    pub min_liquidity_threshold: i128,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub struct TwapResult {
    pub price: i128,
    pub oldest_timestamp: u64,
    pub newest_timestamp: u64,
    pub observation_count: u32,
    pub used_fallback: bool,
    pub avg_deviation_bps: u32,
}

#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub enum TwapStorageKey {
    Config,
    Observations(Symbol),
    LastObservation(Symbol),
    PoolLiquidity(Symbol),
    FallbackPrice(Symbol),
}

// ──────────────────────────────────────────────────────────────────────────
// Configuration Management
// ──────────────────────────────────────────────────────────────────────────

pub fn initialize_twap_config(
    env: Env,
    primary_pool: Symbol,
    secondary_oracle: Option<Symbol>,
    min_observation_count: u32,
    max_deviation_bps: u32,
    observation_window_secs: u64,
    min_liquidity_threshold: i128,
) {
    let config = TwapConfig {
        primary_pool,
        secondary_oracle,
        min_observation_count,
        max_deviation_bps,
        observation_window_secs,
        min_liquidity_threshold,
    };

    env.storage()
        .persistent()
        .set(&TwapStorageKey::Config, &config);
}

pub fn get_twap_config(env: Env) -> TwapConfig {
    env.storage()
        .persistent()
        .get(&TwapStorageKey::Config)
        .unwrap_optimized()
}

// ──────────────────────────────────────────────────────────────────────────
// Observation Recording
// ──────────────────────────────────────────────────────────────────────────

pub fn record_price_observation(
    env: Env,
    asset_pair: Symbol,
    cumulative_price: i128,
    raw_price: i128,
    timestamp: u64,
    ledger_sequence: u32,
) {
    let observation = PriceObservation {
        timestamp,
        cumulative_price,
        price: raw_price,
        ledger_sequence,
    };

    let key = TwapStorageKey::Observations(asset_pair.clone());
    let mut observations: Vec<PriceObservation> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(&env));

    observations.push_back(observation.clone());

    // Store last observation for quick access
    let last_key = TwapStorageKey::LastObservation(asset_pair);
    env.storage()
        .persistent()
        .set(&last_key, &observation);

    env.storage().persistent().set(&key, &observations);
}

pub fn update_pool_liquidity(
    env: Env,
    asset_pair: Symbol,
    liquidity: i128,
) {
    let key = TwapStorageKey::PoolLiquidity(asset_pair);
    env.storage().persistent().set(&key, &liquidity);
}

pub fn set_fallback_price(
    env: Env,
    asset_pair: Symbol,
    price: i128,
    timestamp: u64,
) {
    let observation = PriceObservation {
        timestamp,
        cumulative_price: price,
        price,
        ledger_sequence: env.ledger().sequence(),
    };

    let key = TwapStorageKey::FallbackPrice(asset_pair);
    env.storage().persistent().set(&key, &observation);
}

// ──────────────────────────────────────────────────────────────────────────
// Outlier Detection & Filtering
// ──────────────────────────────────────────────────────────────────────────

fn calculate_median(_env: &Env, prices: &Vec<i128>) -> i128 {
    if prices.is_empty() {
        panic!();
    }

    let len = prices.len();
    
    // For simplicity with Soroban's Vec limitations, we'll use a simple approach
    // Find median by sorting conceptually (for small datasets)
    let mut sorted = prices.clone();
    
    // Bubble sort (acceptable for small observation sets)
    for i in 0..len {
        for j in i + 1..len {
            if sorted.get(j).unwrap_optimized() < sorted.get(i).unwrap_optimized() {
                let temp = sorted.get(i).unwrap_optimized();
                sorted.set(i, sorted.get(j).unwrap_optimized());
                sorted.set(j, temp);
            }
        }
    }

    if len % 2 == 1 {
        sorted.get(len / 2).unwrap_optimized()
    } else {
        let mid1 = sorted.get(len / 2 - 1).unwrap_optimized();
        let mid2 = sorted.get(len / 2).unwrap_optimized();
        (mid1 + mid2) / 2
    }
}

fn filter_outliers(
    env: &Env,
    observations: &Vec<PriceObservation>,
    max_deviation_bps: u32,
) -> (Vec<PriceObservation>, u32) {
    if observations.len() < 2 {
        return (observations.clone(), 0);
    }

    // Extract prices for median calculation
    let mut prices = Vec::new(env);
    for i in 0..observations.len() {
        prices.push_back(observations.get(i).unwrap_optimized().price);
    }

    let median = calculate_median(env, &prices);
    
    let mut filtered = Vec::new(env);
    let mut total_deviation_bps: i128 = 0;
    let mut count: i128 = 0;

    for i in 0..observations.len() {
        let obs = observations.get(i).unwrap_optimized();
        let price = obs.price;

        // Calculate deviation in basis points (10000 bps = 100%)
        let deviation_bps = if price > 0 && median > 0 {
            let diff = if price > median {
                price - median
            } else {
                median - price
            };
            ((diff * 10000) / median) as u32
        } else {
            0
        };

        // Include observation if within tolerance
        if deviation_bps <= max_deviation_bps {
            filtered.push_back(obs);
            total_deviation_bps += deviation_bps as i128;
            count += 1;
        }
    }

    let avg_deviation_bps = if count > 0 {
        ((total_deviation_bps / count) as u32).min(10000)
    } else {
        0
    };

    (filtered, avg_deviation_bps)
}

// ──────────────────────────────────────────────────────────────────────────
// TWAP Calculation
// ──────────────────────────────────────────────────────────────────────────

pub fn calculate_twap(
    env: Env,
    asset_pair: Symbol,
) -> TwapResult {
    let config = get_twap_config(env.clone());

    // Retrieve all observations for this asset pair
    let obs_key = TwapStorageKey::Observations(asset_pair.clone());
    let all_observations: Vec<PriceObservation> = env
        .storage()
        .persistent()
        .get(&obs_key)
        .unwrap_or_else(|| Vec::new(&env));

    // Check if we have sufficient observations
    if all_observations.len() < config.min_observation_count {
        // Fall back to secondary oracle if available
        if config.secondary_oracle.is_some() {
            return calculate_twap_fallback(env, asset_pair, config);
        } else {
            panic!();
        }
    }

    // Filter observations within the observation window
    let current_timestamp = env.ledger().timestamp();
    let window_start = current_timestamp.saturating_sub(config.observation_window_secs);

    let mut window_observations = Vec::new(&env);
    for i in 0..all_observations.len() {
        let obs = all_observations.get(i).unwrap_optimized();
        if obs.timestamp >= window_start && obs.timestamp <= current_timestamp {
            window_observations.push_back(obs);
        }
    }

    // Verify we still have enough observations
    if window_observations.len() < config.min_observation_count {
        // Fall back to secondary oracle
        if config.secondary_oracle.is_some() {
            return calculate_twap_fallback(env, asset_pair, config);
        } else {
            panic!();
        }
    }

    // Filter outliers
    let (filtered_observations, avg_deviation_bps) =
        filter_outliers(&env, &window_observations, config.max_deviation_bps);

    if filtered_observations.is_empty() {
        // All observations were outliers, fall back
        if config.secondary_oracle.is_some() {
            return calculate_twap_fallback(env, asset_pair, config);
        } else {
            panic!();
        }
    }

    // Compute TWAP using cumulative prices
    let first_obs = filtered_observations.get(0).unwrap_optimized();
    let last_obs = filtered_observations.get(filtered_observations.len() - 1).unwrap_optimized();

    let time_delta = if last_obs.timestamp > first_obs.timestamp {
        last_obs.timestamp - first_obs.timestamp
    } else {
        1 // Prevent division by zero
    };

    let cumulative_delta = last_obs.cumulative_price - first_obs.cumulative_price;

    let twap_price = if time_delta > 0 {
        cumulative_delta / (time_delta as i128)
    } else {
        last_obs.price
    };

    TwapResult {
        price: twap_price,
        oldest_timestamp: first_obs.timestamp,
        newest_timestamp: last_obs.timestamp,
        observation_count: filtered_observations.len(),
        used_fallback: false,
        avg_deviation_bps,
    }
}

fn calculate_twap_fallback(
    env: Env,
    asset_pair: Symbol,
    _config: TwapConfig,
) -> TwapResult {
    let fallback_key = TwapStorageKey::FallbackPrice(asset_pair.clone());
    let fallback_obs: PriceObservation = env
        .storage()
        .persistent()
        .get(&fallback_key)
        .unwrap_optimized();

    TwapResult {
        price: fallback_obs.price,
        oldest_timestamp: fallback_obs.timestamp,
        newest_timestamp: fallback_obs.timestamp,
        observation_count: 1,
        used_fallback: true,
        avg_deviation_bps: 0,
    }
}

pub fn get_last_twap(
    env: Env,
    asset_pair: Symbol,
) -> Option<PriceObservation> {
    let key = TwapStorageKey::LastObservation(asset_pair);
    env.storage().persistent().get(&key)
}

pub fn prune_old_observations(
    env: Env,
    asset_pair: Symbol,
    retention_secs: u64,
) -> u32 {
    let _config = get_twap_config(env.clone());
    let obs_key = TwapStorageKey::Observations(asset_pair.clone());
    let all_observations: Vec<PriceObservation> = env
        .storage()
        .persistent()
        .get(&obs_key)
        .unwrap_or_else(|| Vec::new(&env));

    let cutoff_timestamp = env.ledger().timestamp()
        .saturating_sub(retention_secs);

    let mut kept_observations = Vec::new(&env);
    let mut pruned_count = 0u32;

    for i in 0..all_observations.len() {
        let obs = all_observations.get(i).unwrap_optimized();
        if obs.timestamp >= cutoff_timestamp {
            kept_observations.push_back(obs);
        } else {
            pruned_count += 1;
        }
    }

    if pruned_count > 0 {
        env.storage()
            .persistent()
            .set(&obs_key, &kept_observations);
    }

    pruned_count
}

// ──────────────────────────────────────────────────────────────────────────
// Liquidity & Health Checks
// ──────────────────────────────────────────────────────────────────────────

pub fn is_pool_liquid_enough(
    env: Env,
    asset_pair: Symbol,
) -> bool {
    let config = get_twap_config(env.clone());
    let liquidity_key = TwapStorageKey::PoolLiquidity(asset_pair);
    let current_liquidity: i128 = env
        .storage()
        .persistent()
        .get(&liquidity_key)
        .unwrap_or(0);

    current_liquidity >= config.min_liquidity_threshold
}

pub fn get_pool_liquidity(
    env: Env,
    asset_pair: Symbol,
) -> i128 {
    let liquidity_key = TwapStorageKey::PoolLiquidity(asset_pair);
    env.storage()
        .persistent()
        .get(&liquidity_key)
        .unwrap_or(0)
}
