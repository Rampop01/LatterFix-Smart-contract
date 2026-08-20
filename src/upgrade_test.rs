use soroban_sdk::unwrap::UnwrapOptimized;
#![cfg(test)]
#![allow(deprecated)]

use crate::access_control::Role;
use crate::upgrade::{UpgradeStatus, DEFAULT_TIMELOCK_SECONDS, MIN_TIMELOCK_SECONDS};
use crate::{TaskManagerContract, TaskManagerContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, BytesN, Env};

// ── Shared setup ───────────────────────────────────────────────────────────

#[allow(dead_code)]
struct Ctx {
    client: TaskManagerContractClient<'static>,
    admin: Address,
}

fn setup(env: &Env) -> Ctx {
    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(env);

    client.initialize(&admin, &100u32, &token, &fee_recipient);

    Ctx { client, admin }
}

fn uploaded_wasm_hash(env: &Env) -> BytesN<32> {
    let empty_wasm: &[u8] = &[];
    env.deployer().upload_contract_wasm(empty_wasm)
}

fn placeholder_wasm_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[7u8; 32])
}

// ── propose_upgrade ─────────────────────────────────────────────────────────

#[test]
fn test_propose_upgrade_sets_pending_state_with_timelock() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let hash = placeholder_wasm_hash(&env);
    let proposal = ctx.client.propose_upgrade(&ctx.admin, &hash);

    assert_eq!(proposal.status, UpgradeStatus::Pending);
    assert_eq!(proposal.wasm_hash, hash);
    assert_eq!(proposal.proposed_by, ctx.admin);
    assert_eq!(
        proposal.ready_at,
        proposal.proposed_at + DEFAULT_TIMELOCK_SECONDS
    );

    let pending = ctx.client.get_pending_upgrade().unwrap_optimized();
    assert_eq!(pending, proposal);
}

#[test]
fn test_propose_upgrade_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let outsider = Address::generate(&env);
    let hash = placeholder_wasm_hash(&env);

    let res = ctx.client.try_propose_upgrade(&outsider, &hash);
    assert!(res.is_err(), "non-admin propose should be rejected");
}

#[test]
fn test_propose_upgrade_rejects_second_pending_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    ctx.client
        .propose_upgrade(&ctx.admin, &placeholder_wasm_hash(&env));

    let second_hash = BytesN::from_array(&env, &[9u8; 32]);
    let res = ctx.client.try_propose_upgrade(&ctx.admin, &second_hash);
    assert!(
        res.is_err(),
        "a second pending proposal should be rejected while one is already pending"
    );
}

// ── execute_upgrade ──────────────────────────────────────────────────────────

#[test]
fn test_execute_upgrade_rejects_before_timelock_elapses() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    ctx.client
        .propose_upgrade(&ctx.admin, &placeholder_wasm_hash(&env));

    // Immediately after proposing — must be rejected.
    let res = ctx.client.try_execute_upgrade(&ctx.admin);
    assert!(res.is_err(), "premature execution should be rejected");

    // One second short of the timelock — still rejected.
    env.ledger()
        .with_mut(|l| l.timestamp += DEFAULT_TIMELOCK_SECONDS - 1);
    let res = ctx.client.try_execute_upgrade(&ctx.admin);
    assert!(
        res.is_err(),
        "execution one second before ready_at should still be rejected"
    );
}

#[test]
fn test_execute_upgrade_succeeds_after_timelock_elapses() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let hash = uploaded_wasm_hash(&env);
    let proposal = ctx.client.propose_upgrade(&ctx.admin, &hash);

    env.ledger().with_mut(|l| l.timestamp = proposal.ready_at);

    let applied_hash = ctx.client.execute_upgrade(&ctx.admin);
    assert_eq!(applied_hash, hash);

    let pending = ctx.client.get_pending_upgrade().unwrap_optimized();
    assert_eq!(pending.status, UpgradeStatus::Executed);
    assert!(pending.executed_at.is_some());

    // Historical hash log records the applied upgrade for future rollback.
    let history = ctx.client.get_upgrade_history();
    assert_eq!(history.len(), 1);
    let entry = history.get(0).unwrap_optimized();
    assert_eq!(entry.wasm_hash, hash);
    assert_eq!(entry.applied_by, ctx.admin);
}

#[test]
fn test_execute_upgrade_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let hash = uploaded_wasm_hash(&env);
    let proposal = ctx.client.propose_upgrade(&ctx.admin, &hash);
    env.ledger().with_mut(|l| l.timestamp = proposal.ready_at);

    let outsider = Address::generate(&env);
    let res = ctx.client.try_execute_upgrade(&outsider);
    assert!(res.is_err(), "non-admin execute should be rejected");
}

#[test]
fn test_execute_upgrade_rejects_when_already_executed() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let hash = uploaded_wasm_hash(&env);
    let proposal = ctx.client.propose_upgrade(&ctx.admin, &hash);
    env.ledger().with_mut(|l| l.timestamp = proposal.ready_at);
    ctx.client.execute_upgrade(&ctx.admin);

    let res = ctx.client.try_execute_upgrade(&ctx.admin);
    assert!(res.is_err(), "double execution should be rejected");
}

// ── veto_upgrade ─────────────────────────────────────────────────────────────

#[test]
fn test_veto_upgrade_by_guardian_blocks_execution() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let guardian = Address::generate(&env);
    ctx.client
        .grant_role(&ctx.admin, &guardian, &Role::Guardian);

    let hash = placeholder_wasm_hash(&env);
    let proposal = ctx.client.propose_upgrade(&ctx.admin, &hash);

    let vetoed = ctx.client.veto_upgrade(&guardian);
    assert_eq!(vetoed.status, UpgradeStatus::Vetoed);
    assert_eq!(vetoed.vetoed_by, Some(guardian));

    // Even once the timelock would have elapsed, a vetoed proposal can never
    // be executed.
    env.ledger().with_mut(|l| l.timestamp = proposal.ready_at);
    let res = ctx.client.try_execute_upgrade(&ctx.admin);
    assert!(res.is_err(), "execution after veto should be rejected");
}

#[test]
fn test_veto_upgrade_allows_admin_self_correction() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    ctx.client
        .propose_upgrade(&ctx.admin, &placeholder_wasm_hash(&env));

    let vetoed = ctx.client.veto_upgrade(&ctx.admin);
    assert_eq!(vetoed.status, UpgradeStatus::Vetoed);
    assert_eq!(vetoed.vetoed_by, Some(ctx.admin.clone()));
}

#[test]
fn test_veto_upgrade_rejects_unauthorized_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    ctx.client
        .propose_upgrade(&ctx.admin, &placeholder_wasm_hash(&env));

    let outsider = Address::generate(&env);
    let res = ctx.client.try_veto_upgrade(&outsider);
    assert!(
        res.is_err(),
        "veto by a non-guardian, non-admin caller should be rejected"
    );
}

#[test]
fn test_veto_upgrade_rejects_when_no_pending_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let res = ctx.client.try_veto_upgrade(&ctx.admin);
    assert!(
        res.is_err(),
        "veto without a pending proposal should be rejected"
    );
}

// ── Timelock configuration ───────────────────────────────────────────────────

#[test]
fn test_set_upgrade_timelock_changes_future_proposals() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let new_timelock: u64 = 7 * 24 * 3600; // 7 days
    ctx.client.set_upgrade_timelock(&ctx.admin, &new_timelock);
    assert_eq!(ctx.client.get_upgrade_timelock(), new_timelock);

    let proposal = ctx
        .client
        .propose_upgrade(&ctx.admin, &placeholder_wasm_hash(&env));
    assert_eq!(proposal.ready_at, proposal.proposed_at + new_timelock);
}

#[test]
fn test_set_upgrade_timelock_rejects_below_safety_floor() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let res = ctx
        .client
        .try_set_upgrade_timelock(&ctx.admin, &(MIN_TIMELOCK_SECONDS - 1));
    assert!(
        res.is_err(),
        "timelock below the safety floor should be rejected"
    );
}

#[test]
fn test_set_upgrade_timelock_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let ctx = setup(&env);

    let outsider = Address::generate(&env);
    let res = ctx
        .client
        .try_set_upgrade_timelock(&outsider, &(7 * 24 * 3600));
    assert!(res.is_err(), "non-admin timelock change should be rejected");
}
