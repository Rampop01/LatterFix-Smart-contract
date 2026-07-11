#![cfg(test)]

use crate::{TaskManagerContract, TaskManagerContractClient, TaskStatus};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Vec};

#[test]
fn test_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(&env);

    client.initialize(&admin, &100, &token_contract, &fee_recipient);

    // Verify double-initialization fails
    let res = client.try_initialize(&admin, &100, &token_contract, &fee_recipient);
    assert!(res.is_err());
}

#[test]
fn test_create_and_complete_task_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(&env);

    client.initialize(&admin, &250, &token_contract, &fee_recipient); // 2.5% fee

    let creator = Address::generate(&env);
    let assignee = Address::generate(&env);

    // Mint tokens to creator
    let token_admin_client = StellarAssetClient::new(&env, &token_contract);
    token_admin_client.mint(&creator, &1000);

    let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token_client.balance(&creator), 1000);

    let title = String::from_str(&env, "Test Task");
    let description = String::from_str(&env, "Task Description");
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "rust"));

    // Create task
    let task_id = client.create_task(&creator, &title, &description, &1000, &tags);
    assert_eq!(task_id, 1);
    assert_eq!(token_client.balance(&creator), 0);
    assert_eq!(token_client.balance(&contract_id), 1000);

    // Assign task
    client.assign_task(&assignee, &task_id);

    // Submit work
    let delivery_url = String::from_str(&env, "https://github.com/LatterFixxx/LatterFix-Smart-contract");
    client.submit_work(&assignee, &task_id, &delivery_url);

    // Complete task (released by creator)
    client.complete_task(&creator, &task_id);

    // Balance checks: fee is 250 bps (2.5%) of 1000 = 25. Assignee gets 975.
    assert_eq!(token_client.balance(&fee_recipient), 25);
    assert_eq!(token_client.balance(&assignee), 975);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_cancel_task_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(&env);

    client.initialize(&admin, &100, &token_contract, &fee_recipient);

    let creator = Address::generate(&env);

    // Mint tokens to creator
    let token_admin_client = StellarAssetClient::new(&env, &token_contract);
    token_admin_client.mint(&creator, &500);

    let title = String::from_str(&env, "Cancel Task");
    let description = String::from_str(&env, "Will cancel this");
    let tags = Vec::new(&env);

    let task_id = client.create_task(&creator, &title, &description, &500, &tags);
    let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token_client.balance(&creator), 0);
    assert_eq!(token_client.balance(&contract_id), 500);

    client.cancel_task(&creator, &task_id);
    assert_eq!(token_client.balance(&creator), 500);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_dispute_and_resolution() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(&env);

    client.initialize(&admin, &100, &token_contract, &fee_recipient);

    let creator = Address::generate(&env);
    let assignee = Address::generate(&env);

    // Mint tokens to creator
    let token_admin_client = StellarAssetClient::new(&env, &token_contract);
    token_admin_client.mint(&creator, &1000);

    let title = String::from_str(&env, "Dispute Task");
    let description = String::from_str(&env, "Dispute test");
    let tags = Vec::new(&env);

    let task_id = client.create_task(&creator, &title, &description, &1000, &tags);
    client.assign_task(&assignee, &task_id);
    
    // Dispute
    client.dispute_task(&creator, &task_id);

    // Resolve dispute 50/50 split
    client.resolve_dispute(&admin, &task_id, &500, &500);

    let token_client = soroban_sdk::token::Client::new(&env, &token_contract);
    assert_eq!(token_client.balance(&creator), 500);
    assert_eq!(token_client.balance(&assignee), 500);
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_user_profile_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TaskManagerContract);
    let client = TaskManagerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract(token_admin);
    let fee_recipient = Address::generate(&env);

    client.initialize(&admin, &100, &token_contract, &fee_recipient);

    let user = Address::generate(&env);
    let username = String::from_str(&env, "john_doe");
    let bio = String::from_str(&env, "Rust developer");

    // Profile creation
    client.create_profile(&user, &username, &bio);

    // Verify profile info
    let profile_opt = client.get_profile(&user);
    assert!(profile_opt.is_some());
    let profile = profile_opt.unwrap();
    assert_eq!(profile.address, user);
    assert_eq!(profile.username, username);
    assert_eq!(profile.reputation, 100);
    assert_eq!(profile.completed_tasks, 0);
    assert_eq!(profile.bio, bio);

    // Bio update
    let new_bio = String::from_str(&env, "Soroban developer");
    client.update_bio(&user, &new_bio);
    let profile = client.get_profile(&user).unwrap();
    assert_eq!(profile.bio, new_bio);

    // Reputation rewards
    client.reward_contribution(&admin, &user, &25);
    let profile = client.get_profile(&user).unwrap();
    assert_eq!(profile.reputation, 125);
    assert_eq!(profile.completed_tasks, 1);
}
