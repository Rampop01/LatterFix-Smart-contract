use soroban_sdk::unwrap::UnwrapOptimized;
use soroban_sdk::{contracttype, Address, Env, Vec};


#[contracttype]
pub enum VaultKey {
    SupportedTokens,
    VaultBalance(Address),              // token -> total held by the contract
    DepositorBalance(Address, Address), // (depositor, token) -> depositor's claimable balance
    PayrollRoot(u32),                   // payroll_id -> Merkle root
    PayrollClaimed(u32, Address),       // (payroll_id, claimant) -> bool
}

// ── Supported Token Registry ───────────────────────────────────────────────

pub fn get_supported_tokens(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&VaultKey::SupportedTokens)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn is_supported_token(env: &Env, token: &Address) -> bool {
    get_supported_tokens(env).contains(token)
}

pub fn add_supported_token(env: &Env, token: Address) {
    let mut tokens = get_supported_tokens(env);
    if !tokens.contains(&token) {
        tokens.push_back(token);
        env.storage()
            .instance()
            .set(&VaultKey::SupportedTokens, &tokens);
    }
}

pub fn remove_supported_token(env: &Env, token: Address) {
    let tokens = get_supported_tokens(env);
    let mut remaining = Vec::new(env);
    for t in tokens.iter() {
        if t != token {
            remaining.push_back(t);
        }
    }
    env.storage()
        .instance()
        .set(&VaultKey::SupportedTokens, &remaining);
}

// ── Balance Reads ───────────────────────────────────────────────────────────

pub fn get_vault_balance(env: &Env, token: Address) -> i128 {
    env.storage()
        .persistent()
        .get(&VaultKey::VaultBalance(token))
        .unwrap_or(0)
}

pub fn get_depositor_balance(env: &Env, depositor: Address, token: Address) -> i128 {
    env.storage()
        .persistent()
        .get(&VaultKey::DepositorBalance(depositor, token))
        .unwrap_or(0)
}

// ── Deposit / Claim ─────────────────────────────────────────────────────────

pub fn deposit(env: &Env, depositor: Address, token: Address, amount: i128) {
    if amount <= 0 {
        panic!();
    }
    if !is_supported_token(env, &token) {
        panic!();
    }

    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(&depositor, &env.current_contract_address(), &amount);

    let vault_key = VaultKey::VaultBalance(token.clone());
    let vault_total = get_vault_balance(env, token.clone());
    env.storage()
        .persistent()
        .set(&vault_key, &(vault_total + amount));

    let dep_key = VaultKey::DepositorBalance(depositor.clone(), token.clone());
    let dep_balance = get_depositor_balance(env, depositor, token);
    env.storage()
        .persistent()
        .set(&dep_key, &(dep_balance + amount));
}

pub fn claim(env: &Env, claimant: Address, token: Address, amount: i128) {
    if amount <= 0 {
        panic!();
    }

    let dep_key = VaultKey::DepositorBalance(claimant.clone(), token.clone());
    let dep_balance = get_depositor_balance(env, claimant.clone(), token.clone());
    if dep_balance < amount {
        panic!();
    }

    let vault_key = VaultKey::VaultBalance(token.clone());
    let vault_total = get_vault_balance(env, token.clone());

    env.storage()
        .persistent()
        .set(&dep_key, &(dep_balance - amount));
    env.storage()
        .persistent()
        .set(&vault_key, &(vault_total - amount));

    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(&env.current_contract_address(), &claimant, &amount);
}

// ── Merkle Payroll ──────────────────────────────────────────────────────────

use crate::merkle::verify_merkle_proof;
use soroban_sdk::{xdr::ToXdr, BytesN};

pub fn set_payroll_root(env: &Env, payroll_id: u32, root: BytesN<32>) {
    let key = VaultKey::PayrollRoot(payroll_id);
    env.storage().persistent().set(&key, &root);
}

pub fn claim_payroll(
    env: &Env,
    claimant: Address,
    token: Address,
    payroll_id: u32,
    amount: i128,
    proof: Vec<BytesN<32>>,
) {
    if amount <= 0 {
        panic!();
    }

    let claim_key = VaultKey::PayrollClaimed(payroll_id, claimant.clone());
    if env.storage().persistent().has(&claim_key) {
        panic!();
    }

    let root_key = VaultKey::PayrollRoot(payroll_id);
    let root: BytesN<32> = env
        .storage()
        .persistent()
        .get(&root_key)
        .unwrap_optimized();

    let leaf_data = (claimant.clone(), token.clone(), amount).to_xdr(env);
    let leaf = env.crypto().sha256(&leaf_data).into();

    if !verify_merkle_proof(env, &root, &leaf, &proof) {
        panic!();
    }

    env.storage().persistent().set(&claim_key, &true);

    let vault_key = VaultKey::VaultBalance(token.clone());
    let vault_total = get_vault_balance(env, token.clone());

    if vault_total < amount {
        panic!();
    }

    env.storage()
        .persistent()
        .set(&vault_key, &(vault_total - amount));

    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(&env.current_contract_address(), &claimant, &amount);
}
