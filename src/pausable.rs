use soroban_sdk::unwrap::UnwrapOptimized;
use crate::DataKey;
use soroban_sdk::{contracttype, Address, Env};


// ── Types ──────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PauseState {
    NotPaused,
    Paused,
}

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PauseAction {
    CreateTask,
    AssignTask,
    SubmitWork,
    CompleteTask,
    CancelTask,
    DisputeTask,
    Withdraw,
    All,
}

#[contracttype]
pub enum PauseKey {
    Action(PauseAction),
    UserPause(Address, PauseAction),
}

// ── All actions list (excludes the All sentinel) ───────────────────────────

const ALL_ACTIONS: [PauseAction; 7] = [
    PauseAction::CreateTask,
    PauseAction::AssignTask,
    PauseAction::SubmitWork,
    PauseAction::CompleteTask,
    PauseAction::CancelTask,
    PauseAction::DisputeTask,
    PauseAction::Withdraw,
];

// ── Global pause helpers ───────────────────────────────────────────────────

pub fn pause(env: Env, admin: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    env.storage()
        .instance()
        .set(&PauseKey::Action(action), &PauseState::Paused);
}

pub fn unpause(env: Env, admin: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    env.storage()
        .instance()
        .set(&PauseKey::Action(action), &PauseState::NotPaused);
}

pub fn pause_all(env: Env, admin: Address) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    // Mark the synthetic All sentinel so is_globally_paused() is O(1)
    env.storage()
        .instance()
        .set(&PauseKey::Action(PauseAction::All), &PauseState::Paused);
    for action in ALL_ACTIONS {
        env.storage()
            .instance()
            .set(&PauseKey::Action(action), &PauseState::Paused);
    }
}

pub fn unpause_all(env: Env, admin: Address) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    env.storage()
        .instance()
        .set(&PauseKey::Action(PauseAction::All), &PauseState::NotPaused);
    for action in ALL_ACTIONS {
        env.storage()
            .instance()
            .set(&PauseKey::Action(action), &PauseState::NotPaused);
    }
}

// ── User-specific pause helpers ────────────────────────────────────────────

pub fn pause_for_user(env: Env, admin: Address, user: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    env.storage()
        .instance()
        .set(&PauseKey::UserPause(user, action), &true);
}

pub fn unpause_for_user(env: Env, admin: Address, user: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap_optimized();
    if admin != stored {
        panic!();
    }
    env.storage()
        .instance()
        .remove(&PauseKey::UserPause(user, action));
}

// ── Query helpers ──────────────────────────────────────────────────────────

pub fn is_globally_paused(env: &Env) -> bool {
    let state: PauseState = env
        .storage()
        .instance()
        .get(&PauseKey::Action(PauseAction::All))
        .unwrap_or(PauseState::NotPaused);
    state == PauseState::Paused
}

pub fn is_paused(env: Env, action: PauseAction, user: Option<Address>) -> bool {
    // Fast path: whole contract suspended
    if is_globally_paused(&env) {
        return true;
    }

    // Per-action global check
    let global: PauseState = env
        .storage()
        .instance()
        .get(&PauseKey::Action(action))
        .unwrap_or(PauseState::NotPaused);
    if global == PauseState::Paused {
        return true;
    }

    // Per-user check
    if let Some(u) = user {
        env.storage()
            .instance()
            .get(&PauseKey::UserPause(u, action))
            .unwrap_or(false)
    } else {
        false
    }
}

pub fn require_not_paused(env: Env, action: PauseAction, user: Option<Address>) {
    if is_paused(env, action, user) {
        panic!();
    }
}
