use soroban_sdk::{contracttype, Address, Env, Vec};

/// Multi-stablecoin vault module.
///
/// Lets the contract hold balances in more than one SAC token (USDC, ORGUSD,
/// EURT, etc.) within the same instance, with separate ledgers per token and
/// per depositor so funds never mix across asset types.

#[contracttype]
pub enum VaultKey {
    SupportedTokens,
    VaultBalance(Address),              // token -> total held by the contract
    DepositorBalance(Address, Address), // (depositor, token) -> depositor's claimable balance
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
        env.storage().instance().set(&VaultKey::SupportedTokens, &tokens);
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
    env.storage().instance().set(&VaultKey::SupportedTokens, &remaining);
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

/// Deposit `amount` of `token` into the vault on behalf of `depositor`.
/// Requires `token` to already be registered as supported.
pub fn deposit(env: &Env, depositor: Address, token: Address, amount: i128) {
    if amount <= 0 {
        panic!("deposit amount must be positive");
    }
    if !is_supported_token(env, &token) {
        panic!("token not supported");
    }

    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(&depositor, &env.current_contract_address(), &amount);

    let vault_key = VaultKey::VaultBalance(token.clone());
    let vault_total = get_vault_balance(env, token.clone());
    env.storage().persistent().set(&vault_key, &(vault_total + amount));

    let dep_key = VaultKey::DepositorBalance(depositor.clone(), token.clone());
    let dep_balance = get_depositor_balance(env, depositor, token);
    env.storage().persistent().set(&dep_key, &(dep_balance + amount));
}

/// Claim `amount` of `token` out of the vault for `claimant`, drawing down
/// their depositor balance for that specific token.
pub fn claim(env: &Env, claimant: Address, token: Address, amount: i128) {
    if amount <= 0 {
        panic!("claim amount must be positive");
    }

    let dep_key = VaultKey::DepositorBalance(claimant.clone(), token.clone());
    let dep_balance = get_depositor_balance(env, claimant.clone(), token.clone());
    if dep_balance < amount {
        panic!("insufficient vault balance for this token");
    }

    let vault_key = VaultKey::VaultBalance(token.clone());
    let vault_total = get_vault_balance(env, token.clone());

    env.storage().persistent().set(&dep_key, &(dep_balance - amount));
    env.storage().persistent().set(&vault_key, &(vault_total - amount));

    let token_client = soroban_sdk::token::Client::new(env, &token);
    token_client.transfer(&env.current_contract_address(), &claimant, &amount);
}
