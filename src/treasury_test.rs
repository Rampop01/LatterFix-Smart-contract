use soroban_sdk::unwrap::UnwrapOptimized;
#![cfg(test)]
#![allow(deprecated)]

use crate::treasury::{decay_retained_bps, MAX_DECAY_PERIODS};
use crate::{TaskManagerContract, TaskManagerContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env};

// ── Shared setup ───────────────────────────────────────────────────────────

#[allow(dead_code)]
struct Ctx {
    client: TaskManagerContractClient<'static>,
    admin: Address,
    token: Address,
}

fn setup(env: &Env) -> Ctx {
    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(env);

    client.initialize(&admin, &100u32, &token, &fee_recipient);
    client.configure_treasury(&admin, &token);

    Ctx {
        client,
        admin,
        token,
    }
}

fn fund(env: &Env, ctx: &Ctx, amount: i128) {
    StellarAssetClient::new(env, &ctx.token).mint(&ctx.admin, &amount);
    ctx.client.fund_treasury(&ctx.admin, &amount);
}

// ── decay_retained_bps: pure decay curve math ───────────────────────────────

#[test]
fn test_decay_retained_bps_halves_each_period() {
    // 50% retention per period is a straightforward halving curve.
    assert_eq!(decay_retained_bps(5000, 0), 10000);
    assert_eq!(decay_retained_bps(5000, 1), 5000);
    assert_eq!(decay_retained_bps(5000, 2), 2500);
    assert_eq!(decay_retained_bps(5000, 3), 1250);
    assert_eq!(decay_retained_bps(5000, 4), 625);
    assert_eq!(decay_retained_bps(5000, 5), 312);
}

#[test]
fn test_decay_retained_bps_zero_periods_is_fully_unvested() {
    assert_eq!(decay_retained_bps(9000, 0), 10000);
    assert_eq!(decay_retained_bps(1, 0), 10000);
}

#[test]
fn test_decay_retained_bps_slow_decay_converges_toward_zero() {
    // 90% retention per period decays slowly but still reaches (rounds down
    // to) zero well within MAX_DECAY_PERIODS.
    assert_eq!(decay_retained_bps(9000, 500), 0);
    assert!(decay_retained_bps(9000, 10) > decay_retained_bps(9000, 50));
}

#[test]
fn test_decay_retained_bps_caps_at_max_periods() {
    let at_cap = decay_retained_bps(9999, MAX_DECAY_PERIODS);
    let beyond_cap = decay_retained_bps(9999, MAX_DECAY_PERIODS + 10_000);
    assert_eq!(at_cap, beyond_cap);
}

#[test]
fn test_decay_retained_bps_zero_rate_fully_releases_after_one_period() {
    assert_eq!(decay_retained_bps(0, 0), 10000);
    assert_eq!(decay_retained_bps(0, 1), 0);
}

// ── vested_amount / decay curve over the contract ───────────────────────────

#[test]
fn test_vested_amount_follows_decay_curve_halving_schedule() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    // Before the first period elapses: nothing vested.
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 0);

    env.ledger().with_mut(|l| l.timestamp = 100);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 500);

    env.ledger().with_mut(|l| l.timestamp = 200);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 750);

    env.ledger().with_mut(|l| l.timestamp = 300);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 875);

    env.ledger().with_mut(|l| l.timestamp = 400);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 937);
}

#[test]
fn test_vested_amount_zero_before_cliff() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &500u64, // cliff
        &100u64,
        &5000u32,
    );

    env.ledger().with_mut(|l| l.timestamp = 499);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 0);

    // Cliff has passed but no period has elapsed yet.
    env.ledger().with_mut(|l| l.timestamp = 500);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 0);

    // One period past the cliff: same curve as the no-cliff case.
    env.ledger().with_mut(|l| l.timestamp = 600);
    assert_eq!(ctx.client.get_vested_amount(&schedule_id), 500);
}

// ── create_vesting_schedule: validation & over-allocation guard ────────────

#[test]
fn test_create_vesting_schedule_rejects_over_allocation() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 500);

    let res = ctx.client.try_create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &501i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );
    assert!(
        res.is_err(),
        "scheduling more than the funded balance should be rejected"
    );
}

#[test]
fn test_create_vesting_schedule_allows_up_to_funded_balance_but_not_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 500);

    ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &500i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    // The full balance is now allocated; a second schedule of any positive
    // size should be rejected even though the treasury still holds funds.
    let res = ctx.client.try_create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );
    assert!(
        res.is_err(),
        "allocating past the funded balance should be rejected"
    );
}

#[test]
fn test_create_vesting_schedule_rejects_non_converging_decay_rate() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let res = ctx.client.try_create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &10_000u32,
    );
    assert!(
        res.is_err(),
        "a decay rate of 10000 bps never converges and should be rejected"
    );
}

#[test]
fn test_create_vesting_schedule_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let outsider = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let res = ctx.client.try_create_vesting_schedule(
        &outsider,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );
    assert!(
        res.is_err(),
        "non-admin schedule creation should be rejected"
    );
}

// ── claim: safe withdrawal handling ─────────────────────────────────────────

#[test]
fn test_claim_pays_out_vested_amount_and_updates_balances() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    env.ledger().with_mut(|l| l.timestamp = 100);

    let claimed = ctx.client.claim_vesting(&beneficiary, &schedule_id);
    assert_eq!(claimed, 500);
    assert_eq!(ctx.client.get_treasury_balance(), 500);
    assert_eq!(ctx.client.get_claimable_amount(&schedule_id), 0);

    let schedule = ctx.client.get_vesting_schedule(&schedule_id).unwrap_optimized();
    assert_eq!(schedule.claimed_amount, 500);
}

#[test]
fn test_claim_twice_only_pays_the_newly_vested_delta() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    env.ledger().with_mut(|l| l.timestamp = 100);
    ctx.client.claim_vesting(&beneficiary, &schedule_id);

    env.ledger().with_mut(|l| l.timestamp = 200);
    let second_claim = ctx.client.claim_vesting(&beneficiary, &schedule_id);
    assert_eq!(second_claim, 250); // 750 vested total - 500 already claimed
}

#[test]
fn test_claim_rejects_when_nothing_vested_yet() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    let res = ctx.client.try_claim_vesting(&beneficiary, &schedule_id);
    assert!(
        res.is_err(),
        "claiming before any period elapses should be rejected"
    );
}

#[test]
fn test_claim_rejects_non_beneficiary_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);
    let outsider = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    env.ledger().with_mut(|l| l.timestamp = 100);
    let res = ctx.client.try_claim_vesting(&outsider, &schedule_id);
    assert!(
        res.is_err(),
        "claim by a non-beneficiary should be rejected"
    );
}

#[test]
fn test_claim_never_exceeds_total_allocation_even_far_past_full_vesting() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);
    let beneficiary = Address::generate(&env);

    fund(&env, &ctx, 1_000);

    let schedule_id = ctx.client.create_vesting_schedule(
        &ctx.admin,
        &beneficiary,
        &1000i128,
        &0u64,
        &0u64,
        &100u64,
        &5000u32,
    );

    // Jump far beyond the point where the curve has fully converged.
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let claimed = ctx.client.claim_vesting(&beneficiary, &schedule_id);
    assert_eq!(claimed, 1000);

    let schedule = ctx.client.get_vesting_schedule(&schedule_id).unwrap_optimized();
    assert_eq!(schedule.claimed_amount, 1000);

    // A further claim attempt has nothing left to release.
    let res = ctx.client.try_claim_vesting(&beneficiary, &schedule_id);
    assert!(
        res.is_err(),
        "claiming after full vesting/claim should be rejected"
    );
}

// ── fund_treasury ────────────────────────────────────────────────────────────

#[test]
fn test_fund_treasury_increases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    assert_eq!(ctx.client.get_treasury_balance(), 0);
    fund(&env, &ctx, 750);
    assert_eq!(ctx.client.get_treasury_balance(), 750);
    fund(&env, &ctx, 250);
    assert_eq!(ctx.client.get_treasury_balance(), 1000);
}
