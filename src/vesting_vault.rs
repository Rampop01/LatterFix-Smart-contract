use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::DataKey;

// ============================================================================
// Constants
// ============================================================================

/// Minimum vesting period in seconds (1 hour).
pub const MIN_VESTING_PERIOD: u64 = 3600;

/// Maximum vesting period in seconds (30 days).
pub const MAX_VESTING_PERIOD: u64 = 2_592_000;

// ============================================================================
// Types
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VestingVaultStatus {
    Active,
    Disputed,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingVault {
    pub id: u32,
    pub task_id: u32,
    pub milestone_id: u32,
    pub beneficiary: Address,
    pub amount: i128,
    pub token: Address,
    pub created_at: u64,
    pub vesting_end: u64,
    pub status: VestingVaultStatus,
    pub disputed_by: Option<Address>,
    pub dispute_reason: Option<soroban_sdk::String>,
}

#[contracttype]
pub enum VestingVaultKey {
    Vault(u32),
    VaultCount,
    TaskVaults(u32),
    BeneficiaryVaults(Address),
    MilestoneVault(u32, u32),
}

// ============================================================================
// Authorization
// ============================================================================

fn require_admin_or_creator(env: &Env, caller: &Address, task_id: u32) {
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));
    let task: crate::Task = env
        .storage()
        .instance()
        .get(&DataKey::Task(task_id))
        .unwrap_or_else(|| panic!("task not found"));
    if *caller != admin && *caller != task.created_by {
        panic!("not authorized");
    }
}

// ============================================================================
// Core Functions
// ============================================================================

/// Create a vesting vault for a milestone payout. The funds are locked for a
/// safety window during which disputes can be opened.
///
/// This function should be called INSTEAD of transferring milestone payment
/// to the beneficiary. It creates a time-lock on the escrowed funds.
pub fn create_vesting_vault(
    env: Env,
    task_id: u32,
    milestone_id: u32,
    beneficiary: Address,
    amount: i128,
    token: Address,
    vesting_period: u64,
) -> u32 {
    if amount <= 0 {
        panic!("vesting amount must be positive");
    }
    if vesting_period < MIN_VESTING_PERIOD {
        panic!("vesting period too short");
    }
    if vesting_period > MAX_VESTING_PERIOD {
        panic!("vesting period too long");
    }

    // Check milestone exists and is approved
    let milestone_key = crate::escrow::EscrowKey::Milestone(task_id, milestone_id);
    let milestone: crate::escrow::Milestone = env
        .storage()
        .persistent()
        .get(&milestone_key)
        .unwrap_or_else(|| panic!("milestone not found"));

    if milestone.status != crate::escrow::MilestoneStatus::Approved {
        panic!("milestone not approved");
    }

    // Check no existing vault for this milestone
    let vault_key = VestingVaultKey::MilestoneVault(task_id, milestone_id);
    if env.storage().persistent().has(&vault_key) {
        panic!("vault already exists for this milestone");
    }

    // Verify escrow has sufficient balance for this task
    let escrow_key = crate::escrow::EscrowKey::TaskEscrow(task_id);
    let escrow_balance: i128 = env
        .storage()
        .persistent()
        .get(&escrow_key)
        .unwrap_or(0);
    if escrow_balance < amount {
        panic!("insufficient escrow balance for vesting vault");
    }

    let vault_count: u32 = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::VaultCount)
        .unwrap_or(0);
    let vault_id = vault_count + 1;

    let now = env.ledger().timestamp();
    let vault = VestingVault {
        id: vault_id,
        task_id,
        milestone_id,
        beneficiary: beneficiary.clone(),
        amount,
        token: token.clone(),
        created_at: now,
        vesting_end: now + vesting_period,
        status: VestingVaultStatus::Active,
        disputed_by: None,
        dispute_reason: None,
    };

    // Store vault
    env.storage()
        .persistent()
        .set(&VestingVaultKey::Vault(vault_id), &vault);
    env.storage()
        .persistent()
        .set(&VestingVaultKey::VaultCount, &vault_id);
    env.storage()
        .persistent()
        .set(&vault_key, &vault_id);

    // Index by task
    let mut task_vaults: Vec<u32> = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::TaskVaults(task_id))
        .unwrap_or_else(|| Vec::new(&env));
    task_vaults.push_back(vault_id);
    env.storage()
        .persistent()
        .set(&VestingVaultKey::TaskVaults(task_id), &task_vaults);

    // Index by beneficiary
    let mut beneficiary_vaults: Vec<u32> = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::BeneficiaryVaults(beneficiary.clone()))
        .unwrap_or_else(|| Vec::new(&env));
    beneficiary_vaults.push_back(vault_id);
    env.storage()
        .persistent()
        .set(
            &VestingVaultKey::BeneficiaryVaults(beneficiary),
            &beneficiary_vaults,
        );

    vault_id
}

/// Open a dispute on an active vesting vault. Only admin or task creator can
/// dispute during the vesting period.
pub fn dispute_vesting_vault(
    env: Env,
    caller: Address,
    vault_id: u32,
    reason: soroban_sdk::String,
) {
    let mut vault: VestingVault = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::Vault(vault_id))
        .unwrap_or_else(|| panic!("vault not found"));

    if vault.status != VestingVaultStatus::Active {
        panic!("vault is not active");
    }

    let now = env.ledger().timestamp();
    if now >= vault.vesting_end {
        panic!("vesting period has ended");
    }

    require_admin_or_creator(&env, &caller, vault.task_id);

    vault.status = VestingVaultStatus::Disputed;
    vault.disputed_by = Some(caller);
    vault.dispute_reason = Some(reason);

    env.storage()
        .persistent()
        .set(&VestingVaultKey::Vault(vault_id), &vault);
}

/// Release vested funds to the beneficiary after the vesting period ends.
/// Only callable if no dispute is active.
pub fn release_vesting_vault(env: Env, caller: Address, vault_id: u32) -> i128 {
    let mut vault: VestingVault = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::Vault(vault_id))
        .unwrap_or_else(|| panic!("vault not found"));

    if vault.status != VestingVaultStatus::Active {
        panic!("vault is not active");
    }

    let now = env.ledger().timestamp();
    if now < vault.vesting_end {
        panic!("vesting period has not ended");
    }

    // Only beneficiary or admin can release
    caller.require_auth();
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));
    if caller != vault.beneficiary && caller != admin {
        panic!("not authorized to release");
    }

    vault.status = VestingVaultStatus::Released;
    env.storage()
        .persistent()
        .set(&VestingVaultKey::Vault(vault_id), &vault);

    // Transfer tokens to beneficiary
    let token_client = soroban_sdk::token::Client::new(&env, &vault.token);
    token_client.transfer(
        &env.current_contract_address(),
        &vault.beneficiary,
        &vault.amount,
    );

    vault.amount
}

/// Refund disputed vault funds back to the task creator. Only admin can
/// refund a disputed vault.
pub fn refund_vesting_vault(env: Env, admin: Address, vault_id: u32) -> i128 {
    admin.require_auth();

    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("not initialized"));
    if admin != stored_admin {
        panic!("not admin");
    }

    let mut vault: VestingVault = env
        .storage()
        .persistent()
        .get(&VestingVaultKey::Vault(vault_id))
        .unwrap_or_else(|| panic!("vault not found"));

    if vault.status != VestingVaultStatus::Disputed {
        panic!("vault is not disputed");
    }

    let task: crate::Task = env
        .storage()
        .instance()
        .get(&DataKey::Task(vault.task_id))
        .unwrap_or_else(|| panic!("task not found"));

    vault.status = VestingVaultStatus::Refunded;
    env.storage()
        .persistent()
        .set(&VestingVaultKey::Vault(vault_id), &vault);

    // Transfer tokens back to task creator
    let token_client = soroban_sdk::token::Client::new(&env, &vault.token);
    token_client.transfer(
        &env.current_contract_address(),
        &task.created_by,
        &vault.amount,
    );

    vault.amount
}

// ============================================================================
// Views
// ============================================================================

pub fn get_vault(env: &Env, vault_id: u32) -> Option<VestingVault> {
    env.storage()
        .persistent()
        .get(&VestingVaultKey::Vault(vault_id))
}

pub fn get_task_vaults(env: &Env, task_id: u32) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&VestingVaultKey::TaskVaults(task_id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_beneficiary_vaults(env: &Env, beneficiary: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&VestingVaultKey::BeneficiaryVaults(beneficiary.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_milestone_vault_id(env: &Env, task_id: u32, milestone_id: u32) -> Option<u32> {
    env.storage()
        .persistent()
        .get(&VestingVaultKey::MilestoneVault(task_id, milestone_id))
}

pub fn is_vesting_complete(env: &Env, vault_id: u32) -> bool {
    let vault = get_vault(env, vault_id).unwrap_or_else(|| panic!("vault not found"));
    if vault.status != VestingVaultStatus::Active {
        return false;
    }
    let now = env.ledger().timestamp();
    now >= vault.vesting_end
}

pub fn get_remaining_vesting_time(env: &Env, vault_id: u32) -> u64 {
    let vault = get_vault(env, vault_id).unwrap_or_else(|| panic!("vault not found"));
    if vault.status != VestingVaultStatus::Active {
        return 0;
    }
    let now = env.ledger().timestamp();
    if now >= vault.vesting_end {
        0
    } else {
        vault.vesting_end - now
    }
}
