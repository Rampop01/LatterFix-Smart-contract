#![cfg(test)]

use crate::{TaskManagerContract, TaskManagerContractClient, TaskStatus};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Vec};

// ── Shared setup helper ────────────────────────────────────────────────────

fn setup_initialized_contract(env: &Env, fee_bps: u32) -> (
    TaskManagerContractClient,
    Address, // contract_id
    Address, // admin
    Address, // token_contract
    Address, // fee_recipient
) {
    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(env);

    client.initialize(&admin, &fee_bps, &token_contract, &fee_recipient);
    (client, contract_id, admin, token_contract, fee_recipient)
}

// ── Test 1: Contract initialization ────────────────────────────────────────

#[test]
fn test_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, token_contract, fee_recipient) =
        setup_initialized_contract(&env, 100);

    // Double-initialization must fail
    let res = client.try_initialize(&admin, &100, &token_contract, &fee_recipient);
    assert!(res.is_err(), "re-initialization should be rejected");
}

// ── Test 2: Full task lifecycle with fee assertion ─────────────────────────

#[test]
fn test_create_and_complete_task_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, token_contract, fee_recipient) =
        setup_initialized_contract(&env, 250); // 2.5% fee

    let creator = Address::generate(&env);
    let assignee = Address::generate(&env);

    StellarAssetClient::new(&env, &token_contract).mint(&creator, &1000);
    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token.balance(&creator), 1000);

    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "rust"));

    let task_id = client.create_task(
        &creator,
        &String::from_str(&env, "Test Task"),
        &String::from_str(&env, "Task Description"),
        &1000,
        &tags,
    );
    assert_eq!(task_id, 1);
    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&contract_id), 1000);

    client.assign_task(&assignee, &task_id);
    client.submit_work(
        &assignee,
        &task_id,
        &String::from_str(&env, "https://github.com/LatterFixxx/LatterFix-Smart-contract"),
    );
    client.complete_task(&creator, &task_id);

    // 2.5% of 1000 = 25 fee; assignee receives 975
    assert_eq!(token.balance(&fee_recipient), 25);
    assert_eq!(token.balance(&assignee), 975);
    assert_eq!(token.balance(&contract_id), 0);
}

// ── Test 3: Cancel refunds full amount ────────────────────────────────────

#[test]
fn test_cancel_task_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, token_contract, _) =
        setup_initialized_contract(&env, 100);

    let creator = Address::generate(&env);
    StellarAssetClient::new(&env, &token_contract).mint(&creator, &500);

    let task_id = client.create_task(
        &creator,
        &String::from_str(&env, "Cancel Task"),
        &String::from_str(&env, "Will cancel this"),
        &500,
        &Vec::new(&env),
    );

    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&contract_id), 500);

    client.cancel_task(&creator, &task_id);
    assert_eq!(token.balance(&creator), 500, "creator must be fully refunded");
    assert_eq!(token.balance(&contract_id), 0);
}

// ── Test 4: Dispute and 50/50 split resolution ────────────────────────────

#[test]
fn test_dispute_and_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, admin, token_contract, _) =
        setup_initialized_contract(&env, 100);

    let creator = Address::generate(&env);
    let assignee = Address::generate(&env);
    StellarAssetClient::new(&env, &token_contract).mint(&creator, &1000);

    let task_id = client.create_task(
        &creator,
        &String::from_str(&env, "Dispute Task"),
        &String::from_str(&env, "Dispute test"),
        &1000,
        &Vec::new(&env),
    );
    client.assign_task(&assignee, &task_id);
    client.dispute_task(&creator, &task_id);
    client.resolve_dispute(&admin, &task_id, &500, &500);

    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token.balance(&creator), 500, "creator gets 50%");
    assert_eq!(token.balance(&assignee), 500, "assignee gets 50%");
    assert_eq!(token.balance(&contract_id), 0);
}

// ── Test 5: User profile lifecycle ────────────────────────────────────────

#[test]
fn test_user_profile_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, _, _) = setup_initialized_contract(&env, 100);

    let user = Address::generate(&env);
    let username = String::from_str(&env, "john_doe");
    let bio = String::from_str(&env, "Rust developer");

    client.create_profile(&user, &username, &bio);

    let profile = client.get_profile(&user).expect("profile must exist after creation");
    assert_eq!(profile.address, user);
    assert_eq!(profile.username, username);
    assert_eq!(profile.reputation, 100, "starting reputation is 100");
    assert_eq!(profile.completed_tasks, 0);
    assert_eq!(profile.bio, bio);

    let new_bio = String::from_str(&env, "Soroban developer");
    client.update_bio(&user, &new_bio);
    assert_eq!(client.get_profile(&user).unwrap().bio, new_bio);

    client.reward_contribution(&admin, &user, &25);
    let updated = client.get_profile(&user).unwrap();
    assert_eq!(updated.reputation, 125, "reputation must increase by 25");
    assert_eq!(updated.completed_tasks, 1);
}

// ── Test 6: Dispute resolved fully in favour of assignee ─────────────────

#[test]
fn test_dispute_full_assignee_payout() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, admin, token_contract, _) =
        setup_initialized_contract(&env, 0); // 0% fee for clean assertions

    let creator = Address::generate(&env);
    let assignee = Address::generate(&env);
    StellarAssetClient::new(&env, &token_contract).mint(&creator, &800);

    let task_id = client.create_task(
        &creator,
        &String::from_str(&env, "Full Assignee Payout"),
        &String::from_str(&env, "Admin rules in contributor favour"),
        &800,
        &Vec::new(&env),
    );
    client.assign_task(&assignee, &task_id);
    client.dispute_task(&creator, &task_id);
    // Admin awards 100% to assignee
    client.resolve_dispute(&admin, &task_id, &0, &800);

    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token.balance(&assignee), 800, "assignee gets full payout");
    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&contract_id), 0);
}

// ── Test 7: Multiple tasks share the same contract instance ───────────────

#[test]
fn test_multiple_concurrent_tasks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, _, token_contract, fee_recipient) =
        setup_initialized_contract(&env, 100); // 1% fee

    let creator = Address::generate(&env);
    let a1 = Address::generate(&env);
    let a2 = Address::generate(&env);

    StellarAssetClient::new(&env, &token_contract).mint(&creator, &3000);

    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "frontend"));

    let t1 = client.create_task(
        &creator,
        &String::from_str(&env, "Task Alpha"),
        &String::from_str(&env, "First concurrent task"),
        &1000,
        &tags,
    );
    let t2 = client.create_task(
        &creator,
        &String::from_str(&env, "Task Beta"),
        &String::from_str(&env, "Second concurrent task"),
        &2000,
        &tags,
    );

    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token.balance(&contract_id), 3000, "both task rewards locked");
    assert_eq!(token.balance(&creator), 0);

    // Complete task 1 → a1
    client.assign_task(&a1, &t1);
    client.submit_work(&a1, &t1, &String::from_str(&env, "https://github.com/pr/1"));
    client.complete_task(&creator, &t1);

    // Complete task 2 → a2
    client.assign_task(&a2, &t2);
    client.submit_work(&a2, &t2, &String::from_str(&env, "https://github.com/pr/2"));
    client.complete_task(&creator, &t2);

    // 1% of 1000 = 10, 1% of 2000 = 20 → fee_recipient gets 30
    assert_eq!(token.balance(&fee_recipient), 30, "combined fees correct");
    assert_eq!(token.balance(&a1), 990);
    assert_eq!(token.balance(&a2), 1980);
    assert_eq!(token.balance(&contract_id), 0);
}

// ── Test 8: Cannot assign to an already-assigned task ────────────────────

#[test]
fn test_cannot_double_assign() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, token_contract, _) = setup_initialized_contract(&env, 100);

    let creator = Address::generate(&env);
    let a1 = Address::generate(&env);
    let a2 = Address::generate(&env);

    StellarAssetClient::new(&env, &token_contract).mint(&creator, &500);

    let task_id = client.create_task(
        &creator,
        &String::from_str(&env, "Single Assign Task"),
        &String::from_str(&env, "Only one assignee allowed"),
        &500,
        &Vec::new(&env),
    );

    client.assign_task(&a1, &task_id);
    // Second assignment to same task must fail
    let result = client.try_assign_task(&a2, &task_id);
    assert!(result.is_err(), "double-assigning a task should be rejected");
}

// ── Test 9: Deposits into different tokens are tracked on separate ledgers ─

#[test]
fn test_multi_stablecoin_vault_separate_ledgers() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, contract_id, admin, _, _) = setup_initialized_contract(&env, 100);

    let usdc_admin = Address::generate(&env);
    let usdc = env.register_stellar_asset_contract(usdc_admin);
    let eurt_admin = Address::generate(&env);
    let eurt = env.register_stellar_asset_contract(eurt_admin);

    client.add_supported_token(&admin, &usdc);
    client.add_supported_token(&admin, &eurt);
    assert!(client.is_token_supported(&usdc));
    assert!(client.is_token_supported(&eurt));

    let employer = Address::generate(&env);
    StellarAssetClient::new(&env, &usdc).mint(&employer, &1000);
    StellarAssetClient::new(&env, &eurt).mint(&employer, &500);

    client.deposit_to_vault(&employer, &usdc, &1000);
    client.deposit_to_vault(&employer, &eurt, &500);

    // Ledgers must not mix across token types
    assert_eq!(client.get_token_vault_balance(&usdc), 1000);
    assert_eq!(client.get_token_vault_balance(&eurt), 500);
    assert_eq!(client.get_depositor_vault_balance(&employer, &usdc), 1000);
    assert_eq!(client.get_depositor_vault_balance(&employer, &eurt), 500);

    let usdc_token = soroban_sdk::token::Client::new(&env, &usdc);
    let eurt_token = soroban_sdk::token::Client::new(&env, &eurt);
    assert_eq!(usdc_token.balance(&contract_id), 1000);
    assert_eq!(eurt_token.balance(&contract_id), 500);
}

// ── Test 10: Claiming draws down only the claimed token's ledger ──────────

#[test]
fn test_vault_claim_reduces_correct_token_only() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, _, _) = setup_initialized_contract(&env, 100);

    let usdc_admin = Address::generate(&env);
    let usdc = env.register_stellar_asset_contract(usdc_admin);
    let eurt_admin = Address::generate(&env);
    let eurt = env.register_stellar_asset_contract(eurt_admin);

    client.add_supported_token(&admin, &usdc);
    client.add_supported_token(&admin, &eurt);

    let worker = Address::generate(&env);
    StellarAssetClient::new(&env, &usdc).mint(&worker, &300);
    StellarAssetClient::new(&env, &eurt).mint(&worker, &200);

    client.deposit_to_vault(&worker, &usdc, &300);
    client.deposit_to_vault(&worker, &eurt, &200);

    client.claim_from_vault(&worker, &usdc, &300);

    assert_eq!(client.get_depositor_vault_balance(&worker, &usdc), 0);
    assert_eq!(client.get_depositor_vault_balance(&worker, &eurt), 200, "EURT balance must be untouched by a USDC claim");
    assert_eq!(client.get_token_vault_balance(&usdc), 0);
    assert_eq!(client.get_token_vault_balance(&eurt), 200);

    let usdc_token = soroban_sdk::token::Client::new(&env, &usdc);
    assert_eq!(usdc_token.balance(&worker), 300, "worker received the claimed USDC back");
}

// ── Test 11: Deposit rejected for a token that isn't registered ──────────

#[test]
fn test_vault_deposit_rejects_unsupported_token() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, _, _, _) = setup_initialized_contract(&env, 100);

    let orgusd_admin = Address::generate(&env);
    let orgusd = env.register_stellar_asset_contract(orgusd_admin);

    let depositor = Address::generate(&env);
    StellarAssetClient::new(&env, &orgusd).mint(&depositor, &100);

    // ORGUSD was never added via add_supported_token
    let result = client.try_deposit_to_vault(&depositor, &orgusd, &100);
    assert!(result.is_err(), "depositing an unsupported token must be rejected");
}

// ── Test 12: Claim beyond depositor's balance is rejected ────────────────

#[test]
fn test_vault_claim_rejects_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, _, _) = setup_initialized_contract(&env, 100);

    let usdc_admin = Address::generate(&env);
    let usdc = env.register_stellar_asset_contract(usdc_admin);
    client.add_supported_token(&admin, &usdc);

    let depositor = Address::generate(&env);
    StellarAssetClient::new(&env, &usdc).mint(&depositor, &50);
    client.deposit_to_vault(&depositor, &usdc, &50);

    let result = client.try_claim_from_vault(&depositor, &usdc, &51);
    assert!(result.is_err(), "claiming more than the deposited balance must be rejected");
}

// ── Test 13: Removing a supported token blocks further deposits ──────────

#[test]
fn test_vault_removed_token_blocks_new_deposits() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _, admin, _, _) = setup_initialized_contract(&env, 100);

    let usdc_admin = Address::generate(&env);
    let usdc = env.register_stellar_asset_contract(usdc_admin);
    client.add_supported_token(&admin, &usdc);
    assert!(client.is_token_supported(&usdc));

    client.remove_supported_token(&admin, &usdc);
    assert!(!client.is_token_supported(&usdc));

    let depositor = Address::generate(&env);
    StellarAssetClient::new(&env, &usdc).mint(&depositor, &10);
    let result = client.try_deposit_to_vault(&depositor, &usdc, &10);
    assert!(result.is_err(), "deposits must be rejected after a token is removed");
}
