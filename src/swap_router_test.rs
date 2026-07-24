#![cfg(test)]
#![allow(deprecated)]

use crate::swap_router::{ConversionOutcome, SwapRoute};
use crate::{TaskManagerContract, TaskManagerContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Vec};

// ── Mock DEX pool ──────────────────────────────────────────────────────────
//
// Mimics a Uniswap-V2-style AMM pair: it expects to have already received
// `amount_in` of `token_in` (the router transfers it in before calling
// `swap`), applies a fixed exchange rate, and pays `token_out` out of its own
// reserves. Must be pre-funded with `token_out` in tests via `mint`.

#[contract]
pub struct MockPool;

#[contractimpl]
impl MockPool {
    pub fn init(env: Env, rate_bps: i128) {
        env.storage()
            .instance()
            .set(&symbol_short!("rate"), &rate_bps);
    }

    pub fn swap(
        env: Env,
        amount_in: i128,
        min_amount_out: i128,
        _token_in: Address,
        token_out: Address,
        to: Address,
    ) -> i128 {
        let rate: i128 = env
            .storage()
            .instance()
            .get(&symbol_short!("rate"))
            .unwrap_or(10_000);
        let amount_out = amount_in * rate / 10_000;

        if amount_out < min_amount_out {
            panic!("mock pool: insufficient output");
        }

        soroban_sdk::token::Client::new(&env, &token_out).transfer(
            &env.current_contract_address(),
            &to,
            &amount_out,
        );

        amount_out
    }
}

// ── Mock oracle ────────────────────────────────────────────────────────────

#[contracttype]
pub enum MockOracleKey {
    Price(Address),
}

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn set_price(env: Env, asset: Address, price: i128) {
        env.storage()
            .instance()
            .set(&MockOracleKey::Price(asset), &price);
    }

    pub fn price(env: Env, asset: Address) -> Option<i128> {
        env.storage().instance().get(&MockOracleKey::Price(asset))
    }
}

// ── Shared setup ───────────────────────────────────────────────────────────

const ONE: i128 = 10_000_000; // oracle price scale, 10^ORACLE_PRICE_DECIMALS

fn setup(
    env: &Env,
) -> (
    TaskManagerContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(env);

    client.initialize(&admin, &100u32, &token_contract, &fee_recipient);

    let oracle_id = env.register_contract(None, MockOracle);
    client.configure_swap_router(&admin, &oracle_id, &4u32, &500u32); // 5% default slippage

    (client, contract_id, admin, oracle_id)
}

fn new_token(env: &Env) -> Address {
    let admin = Address::generate(env);
    env.register_stellar_asset_contract(admin)
}

fn new_pool(env: &Env, rate_bps: i128) -> Address {
    let pool_id = env.register_contract(None, MockPool);
    let pool_client = MockPoolClient::new(env, &pool_id);
    pool_client.init(&rate_bps);
    pool_id
}

// ── Test 1: direct (single-hop) conversion succeeds ────────────────────────

#[test]
fn test_direct_swap_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, admin, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let stablecoin = new_token(&env);
    client.add_approved_stablecoin(&admin, &stablecoin);

    oracle.set_price(&token_in, &ONE);
    oracle.set_price(&stablecoin, &ONE);

    let pool = new_pool(&env, 10_000); // 1:1 rate
    StellarAssetClient::new(&env, &stablecoin).mint(&pool, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool.clone());
    let route = SwapRoute { path, pools };

    let outcome = client.convert_incoming_deposit(&sender, &token_in, &1_000, &route, &None);

    match outcome {
        ConversionOutcome::Converted(token_out, amount_out) => {
            assert_eq!(token_out, stablecoin);
            assert_eq!(amount_out, 1_000);
        }
        ConversionOutcome::Refunded(_) => panic!("expected a successful conversion"),
    }

    let token_in_client = soroban_sdk::token::Client::new(&env, &token_in);
    assert_eq!(token_in_client.balance(&sender), 0);
    assert_eq!(token_in_client.balance(&contract_id), 0);
    assert_eq!(client.get_vault_balance(&sender, &stablecoin), 1_000);

    let stats = client.get_swap_router_stats();
    assert_eq!(stats.total_conversions, 1);
    assert_eq!(stats.total_refunds, 0);
    assert_eq!(stats.total_stablecoin_out, 1_000);
}

// ── Test 2: multi-hop conversion succeeds ───────────────────────────────────

#[test]
fn test_multi_hop_swap_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let token_mid = new_token(&env);
    let stablecoin = new_token(&env);
    client.add_approved_stablecoin(&admin, &stablecoin);

    oracle.set_price(&token_in, &ONE);
    oracle.set_price(&stablecoin, &ONE);

    // Hop 1: token_in -> token_mid at 98%. Hop 2: token_mid -> stablecoin at 98%.
    let pool1 = new_pool(&env, 9_800);
    let pool2 = new_pool(&env, 9_800);
    StellarAssetClient::new(&env, &token_mid).mint(&pool1, &10_000);
    StellarAssetClient::new(&env, &stablecoin).mint(&pool2, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(token_mid.clone());
    path.push_back(stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool1);
    pools.push_back(pool2);
    let route = SwapRoute { path, pools };

    // 1000 * 0.98 = 980; 980 * 0.98 = 960.4 -> 960 (integer division)
    let outcome =
        client.convert_incoming_deposit(&sender, &token_in, &1_000, &route, &Some(500u32));

    match outcome {
        ConversionOutcome::Converted(token_out, amount_out) => {
            assert_eq!(token_out, stablecoin);
            assert_eq!(amount_out, 960);
        }
        ConversionOutcome::Refunded(_) => panic!("expected a successful multi-hop conversion"),
    }

    assert_eq!(client.get_vault_balance(&sender, &stablecoin), 960);
}

// ── Test 3: unresolved route (unapproved destination) is refunded up front ─

#[test]
fn test_refund_on_unresolved_route() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let not_a_stablecoin = new_token(&env); // never added to the approved list
    oracle.set_price(&token_in, &ONE);
    oracle.set_price(&not_a_stablecoin, &ONE);

    let pool = new_pool(&env, 10_000);
    StellarAssetClient::new(&env, &not_a_stablecoin).mint(&pool, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(not_a_stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool);
    let route = SwapRoute { path, pools };

    let outcome = client.convert_incoming_deposit(&sender, &token_in, &1_000, &route, &None);

    assert!(matches!(outcome, ConversionOutcome::Refunded(_)));

    let token_in_client = soroban_sdk::token::Client::new(&env, &token_in);
    assert_eq!(
        token_in_client.balance(&sender),
        1_000,
        "sender funds must never be pulled"
    );

    let stats = client.get_swap_router_stats();
    assert_eq!(stats.total_refunds, 1);
    assert_eq!(stats.total_conversions, 0);
}

// ── Test 4: missing oracle price is refunded up front ──────────────────────

#[test]
fn test_refund_on_missing_oracle_price() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let stablecoin = new_token(&env);
    client.add_approved_stablecoin(&admin, &stablecoin);

    // No price set for token_in.
    oracle.set_price(&stablecoin, &ONE);

    let pool = new_pool(&env, 10_000);
    StellarAssetClient::new(&env, &stablecoin).mint(&pool, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool);
    let route = SwapRoute { path, pools };

    let outcome = client.convert_incoming_deposit(&sender, &token_in, &1_000, &route, &None);

    assert!(matches!(outcome, ConversionOutcome::Refunded(_)));

    let token_in_client = soroban_sdk::token::Client::new(&env, &token_in);
    assert_eq!(token_in_client.balance(&sender), 1_000);
}

// ── Test 5: output below the oracle-derived minimum reverts the whole call ─

#[test]
fn test_swap_reverts_when_below_minimum_return() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let stablecoin = new_token(&env);
    client.add_approved_stablecoin(&admin, &stablecoin);

    oracle.set_price(&token_in, &ONE);
    oracle.set_price(&stablecoin, &ONE); // oracle says 1:1

    // Pool only pays out 50% — well below the 5% slippage tolerance.
    let pool = new_pool(&env, 5_000);
    StellarAssetClient::new(&env, &stablecoin).mint(&pool, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool);
    let route = SwapRoute { path, pools };

    let result = client.try_convert_incoming_deposit(&sender, &token_in, &1_000, &route, &None);
    assert!(result.is_err(), "swap below minimum return must revert");

    // Because the host reverts the whole invocation, the sender's balance is
    // untouched — the deposit pull never took effect.
    let token_in_client = soroban_sdk::token::Client::new(&env, &token_in);
    assert_eq!(token_in_client.balance(&sender), 1_000);
}

// ── Test 6: only admin may configure the router / approve stablecoins ──────

#[test]
fn test_only_admin_can_configure_router() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _admin, oracle_id) = setup(&env);
    let not_admin = Address::generate(&env);
    let stablecoin = new_token(&env);

    let result = client.try_add_approved_stablecoin(&not_admin, &stablecoin);
    assert!(
        result.is_err(),
        "non-admin must not be able to approve stablecoins"
    );

    let result = client.try_configure_swap_router(&not_admin, &oracle_id, &4u32, &500u32);
    assert!(
        result.is_err(),
        "non-admin must not be able to reconfigure the router"
    );
}

// ── Test 7: withdrawing a converted vault balance pays out the stablecoin ──

#[test]
fn test_withdraw_stablecoin_after_conversion() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, oracle_id) = setup(&env);
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let token_in = new_token(&env);
    let stablecoin = new_token(&env);
    client.add_approved_stablecoin(&admin, &stablecoin);

    oracle.set_price(&token_in, &ONE);
    oracle.set_price(&stablecoin, &ONE);

    let pool = new_pool(&env, 10_000);
    StellarAssetClient::new(&env, &stablecoin).mint(&pool, &10_000);

    let sender = Address::generate(&env);
    StellarAssetClient::new(&env, &token_in).mint(&sender, &1_000);

    let mut path = Vec::new(&env);
    path.push_back(token_in.clone());
    path.push_back(stablecoin.clone());
    let mut pools = Vec::new(&env);
    pools.push_back(pool);
    let route = SwapRoute { path, pools };

    client.convert_incoming_deposit(&sender, &token_in, &1_000, &route, &None);
    client.withdraw_stablecoin(&sender, &stablecoin, &1_000);

    let stablecoin_client = soroban_sdk::token::Client::new(&env, &stablecoin);
    assert_eq!(stablecoin_client.balance(&sender), 1_000);
    assert_eq!(client.get_vault_balance(&sender, &stablecoin), 0);
}
