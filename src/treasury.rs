//! Decentralized reward treasury: decay-curve vesting & distribution manager.
//!
//! Holds a single SAC reward token on behalf of the protocol and releases it
//! to beneficiaries along a *decay curve* rather than a straight linear
//! ramp: each elapsed vesting period releases a fixed percentage of
//! whatever remains unvested, so the release rate is front-loaded and tapers
//! off the longer a schedule runs — mirroring how emission-style incentive
//! programs (liquidity mining, early-contributor rewards, etc.) usually want
//! to pay out.
//!
//! ### Decay model
//!
//! A schedule is defined by `total_amount`, a `start_time`, an optional
//! `cliff_seconds`, a `period_seconds` step size, and a `decay_rate_bps`
//! retention rate (basis points of what's *not yet* vested that stays
//! unvested after each period). After `n` full periods past the cliff, the
//! still-unvested fraction is `(decay_rate_bps / 10000)^n`, so:
//!
//! ```text
//! vested(n) = total_amount * (1 - (decay_rate_bps / 10000)^n)
//! ```
//!
//! `decay_rate_bps` must be strictly less than 10000 so the curve actually
//! converges to `total_amount`; period count is capped at
//! `MAX_DECAY_PERIODS` to keep the exponentiation loop bounded regardless of
//! how long a schedule has been left unclaimed.
//!
//! ### Over-allocation guard
//!
//! `create_vesting_schedule` rejects any schedule whose `total_amount` would
//! push the sum of all schedules' `total_amount` past the treasury's funded
//! balance, so the treasury can never promise more than it holds. `claim`
//! additionally re-checks the live treasury balance and never lets a
//! schedule's `claimed_amount` exceed its `total_amount`, so rounding in the
//! decay curve can't be exploited to over-withdraw.

use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::DataKey;

// ============================================================================
// Constants
// ============================================================================

const BPS_DENOMINATOR: i128 = 10_000;

/// Upper bound on the number of decay periods applied when computing a
/// schedule's vested amount. Bounds the cost of `decay_retained_bps`
/// regardless of how long ago a schedule started; past this many periods the
/// curve has converged close enough to zero that the remainder rounds down
/// to fully vested anyway.
pub const MAX_DECAY_PERIODS: u32 = 500;

// ============================================================================
// Types
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingSchedule {
    pub id: u32,
    pub beneficiary: Address,
    pub total_amount: i128,
    pub claimed_amount: i128,
    pub start_time: u64,
    pub cliff_seconds: u64,
    pub period_seconds: u64,
    /// Basis points (0..10000) of the still-unvested balance that remains
    /// unvested after each elapsed period. Lower = faster decay = more
    /// released per period.
    pub decay_rate_bps: u32,
    pub created_at: u64,
}

#[contracttype]
pub enum TreasuryKey {
    Token,
    Balance,
    AllocatedTotal,
    ScheduleCount,
    Schedule(u32),
    BeneficiarySchedules(Address),
}

// ============================================================================
// Authorization
// ============================================================================

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));
    if *caller != admin {
        panic!("not admin");
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Set the SAC token this treasury holds and pays out. Admin-only.
pub fn configure_treasury(env: Env, admin: Address, token: Address) {
    require_admin(&env, &admin);
    env.storage().instance().set(&TreasuryKey::Token, &token);
}

fn get_treasury_token(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&TreasuryKey::Token)
        .unwrap_or_else(|| panic!("treasury not configured"))
}

// ============================================================================
// Balance tracking
// ============================================================================

pub fn get_treasury_balance(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&TreasuryKey::Balance)
        .unwrap_or(0)
}

fn set_treasury_balance(env: &Env, balance: i128) {
    env.storage()
        .instance()
        .set(&TreasuryKey::Balance, &balance);
}

/// Sum of `total_amount` across every vesting schedule ever created. Used as
/// the over-allocation ceiling against the funded balance.
pub fn get_allocated_total(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&TreasuryKey::AllocatedTotal)
        .unwrap_or(0)
}

fn set_allocated_total(env: &Env, allocated: i128) {
    env.storage()
        .instance()
        .set(&TreasuryKey::AllocatedTotal, &allocated);
}

/// Deposit `amount` of the treasury token from `funder` into the treasury.
/// Returns the new total treasury balance.
pub fn fund_treasury(env: Env, funder: Address, amount: i128) -> i128 {
    funder.require_auth();

    if amount <= 0 {
        panic!("fund amount must be positive");
    }

    let token = get_treasury_token(&env);
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    token_client.transfer(&funder, &env.current_contract_address(), &amount);

    let balance = get_treasury_balance(&env) + amount;
    set_treasury_balance(&env, balance);
    balance
}

// ============================================================================
// Decay curve
// ============================================================================

/// Fraction (in basis points, 0..=10000) of a schedule that is still
/// *unvested* after `periods` full decay steps at retention rate
/// `rate_bps` per step. `periods` is capped at `MAX_DECAY_PERIODS`.
///
/// `retained(0) = 10000`; each step multiplies the running fraction by
/// `rate_bps / 10000`, so `retained` decreases monotonically toward zero.
pub fn decay_retained_bps(rate_bps: u32, periods: u32) -> u32 {
    let steps = periods.min(MAX_DECAY_PERIODS);
    let rate = rate_bps as i128;

    let mut retained: i128 = BPS_DENOMINATOR;
    for _ in 0..steps {
        if retained == 0 {
            break;
        }
        retained = (retained * rate) / BPS_DENOMINATOR;
    }
    retained as u32
}

fn vested_amount_at(schedule: &VestingSchedule, now: u64) -> i128 {
    let cliff_end = schedule.start_time + schedule.cliff_seconds;
    if now < cliff_end {
        return 0;
    }

    let elapsed = now - cliff_end;
    let periods_elapsed_u64 = elapsed / schedule.period_seconds;
    let periods_elapsed = if periods_elapsed_u64 > MAX_DECAY_PERIODS as u64 {
        MAX_DECAY_PERIODS
    } else {
        periods_elapsed_u64 as u32
    };

    let retained_bps = decay_retained_bps(schedule.decay_rate_bps, periods_elapsed);
    let vested_fraction_bps = (BPS_DENOMINATOR as u32 - retained_bps) as i128;

    let vested = schedule
        .total_amount
        .checked_mul(vested_fraction_bps)
        .unwrap_or_else(|| panic!("vested amount overflow"))
        / BPS_DENOMINATOR;

    if vested > schedule.total_amount {
        schedule.total_amount
    } else {
        vested
    }
}

/// Amount of `schedule_id`'s total that has vested as of the current ledger
/// timestamp, ignoring how much has already been claimed.
pub fn vested_amount(env: &Env, schedule_id: u32) -> i128 {
    let schedule = get_vesting_schedule(env, schedule_id)
        .unwrap_or_else(|| panic!("vesting schedule not found"));
    vested_amount_at(&schedule, env.ledger().timestamp())
}

/// Amount of `schedule_id` currently claimable: vested minus already claimed.
pub fn claimable_amount(env: &Env, schedule_id: u32) -> i128 {
    let schedule = get_vesting_schedule(env, schedule_id)
        .unwrap_or_else(|| panic!("vesting schedule not found"));
    let vested = vested_amount_at(&schedule, env.ledger().timestamp());
    vested - schedule.claimed_amount
}

// ============================================================================
// Schedule lifecycle
// ============================================================================

fn next_schedule_id(env: &Env) -> u32 {
    let count: u32 = env
        .storage()
        .instance()
        .get(&TreasuryKey::ScheduleCount)
        .unwrap_or(0);
    let id = count + 1;
    env.storage()
        .instance()
        .set(&TreasuryKey::ScheduleCount, &id);
    id
}

/// Create a decay-curve vesting schedule for `beneficiary`. Admin-only.
///
/// Rejected if `total_amount` would push the sum of all schedules' totals
/// past the treasury's currently funded balance (over-allocation guard), or
/// if `decay_rate_bps` is >= 10000 (the curve would never converge).
pub fn create_vesting_schedule(
    env: Env,
    admin: Address,
    beneficiary: Address,
    total_amount: i128,
    start_time: u64,
    cliff_seconds: u64,
    period_seconds: u64,
    decay_rate_bps: u32,
) -> u32 {
    require_admin(&env, &admin);

    if total_amount <= 0 {
        panic!("total amount must be positive");
    }
    if period_seconds == 0 {
        panic!("period seconds must be positive");
    }
    if decay_rate_bps >= BPS_DENOMINATOR as u32 {
        panic!("decay rate must be less than 10000 bps to converge");
    }

    let balance = get_treasury_balance(&env);
    let allocated = get_allocated_total(&env);
    let available = balance - allocated;
    if total_amount > available {
        panic!("insufficient unallocated treasury balance");
    }

    let id = next_schedule_id(&env);
    let schedule = VestingSchedule {
        id,
        beneficiary: beneficiary.clone(),
        total_amount,
        claimed_amount: 0,
        start_time,
        cliff_seconds,
        period_seconds,
        decay_rate_bps,
        created_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&TreasuryKey::Schedule(id), &schedule);
    crate::storage::extend_persistent_ttl(
        &env,
        &TreasuryKey::Schedule(id),
        100_000,
        crate::storage::DEFAULT_PERSISTENT_TTL,
    );

    set_allocated_total(&env, allocated + total_amount);

    let mut ids = get_beneficiary_schedule_ids(&env, &beneficiary);
    ids.push_back(id);
    env.storage()
        .persistent()
        .set(&TreasuryKey::BeneficiarySchedules(beneficiary), &ids);

    id
}

/// Claim everything currently vested-but-unclaimed on `schedule_id`.
/// Beneficiary-only. Returns the amount transferred.
pub fn claim(env: Env, beneficiary: Address, schedule_id: u32) -> i128 {
    beneficiary.require_auth();

    let mut schedule = get_vesting_schedule(&env, schedule_id)
        .unwrap_or_else(|| panic!("vesting schedule not found"));

    if schedule.beneficiary != beneficiary {
        panic!("caller is not the schedule beneficiary");
    }

    let vested = vested_amount_at(&schedule, env.ledger().timestamp());
    let claimable = vested - schedule.claimed_amount;
    if claimable <= 0 {
        panic!("nothing to claim yet");
    }

    // Over-allocation guard: a schedule can never pay out more than its
    // total_amount, no matter how the decay curve rounds.
    let new_claimed = schedule.claimed_amount + claimable;
    if new_claimed > schedule.total_amount {
        panic!("claim would exceed total vested allocation");
    }

    let balance = get_treasury_balance(&env);
    if claimable > balance {
        panic!("insufficient treasury balance");
    }

    // Effects before interactions.
    schedule.claimed_amount = new_claimed;
    env.storage()
        .persistent()
        .set(&TreasuryKey::Schedule(schedule_id), &schedule);
    set_treasury_balance(&env, balance - claimable);

    let token = get_treasury_token(&env);
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    token_client.transfer(&env.current_contract_address(), &beneficiary, &claimable);

    claimable
}

// ============================================================================
// Views
// ============================================================================

pub fn get_vesting_schedule(env: &Env, schedule_id: u32) -> Option<VestingSchedule> {
    env.storage()
        .persistent()
        .get(&TreasuryKey::Schedule(schedule_id))
}

fn get_beneficiary_schedule_ids(env: &Env, beneficiary: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&TreasuryKey::BeneficiarySchedules(beneficiary.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_beneficiary_schedules(env: Env, beneficiary: Address) -> Vec<u32> {
    get_beneficiary_schedule_ids(&env, &beneficiary)
}
