use crate::DataKey;
use soroban_sdk::{contracttype, Address, Env};

/// Granular pause/unpause system for the LatterFix contract.
///
/// Two levels of pause are supported:
///
/// 1. **Global action pause** — disables a specific `PauseAction` for all
///    users (e.g. pause `CreateTask` during a security review).
/// 2. **User-specific pause** — blocks a particular user from performing an
///    action (e.g. ban a bad actor from `SubmitWork`).
///
/// Pause state is stored in `instance()` storage (fast, cheap, never archived).
/// Every state-changing function in `lib.rs` calls `require_not_paused()` before
/// performing its business logic.

// ── Types ──────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PauseState {
    NotPaused,
    Paused,
}

/// Every public action that can be independently paused.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PauseAction {
    CreateTask,
    AssignTask,
    SubmitWork,
    CompleteTask,
    CancelTask,
    DisputeTask,
    Withdraw,
    /// Synthetic sentinel used to check "is the whole contract paused?"
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

/// Pause a specific action globally. Requires admin auth.
pub fn pause(env: Env, admin: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
    }
    env.storage()
        .instance()
        .set(&PauseKey::Action(action), &PauseState::Paused);
}

/// Resume a specific action globally. Requires admin auth.
pub fn unpause(env: Env, admin: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
    }
    env.storage()
        .instance()
        .set(&PauseKey::Action(action), &PauseState::NotPaused);
}

/// Pause ALL contract actions in a single call. Emergency circuit-breaker.
/// Requires admin auth.
pub fn pause_all(env: Env, admin: Address) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
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

/// Unpause ALL contract actions. Requires admin auth.
pub fn unpause_all(env: Env, admin: Address) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
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

/// Block a specific user from a specific action. Requires admin auth.
pub fn pause_for_user(env: Env, admin: Address, user: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
    }
    env.storage()
        .instance()
        .set(&PauseKey::UserPause(user, action), &true);
}

/// Unblock a specific user from a specific action. Requires admin auth.
pub fn unpause_for_user(env: Env, admin: Address, user: Address, action: PauseAction) {
    admin.require_auth();
    let stored: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored {
        panic!("not admin");
    }
    env.storage()
        .instance()
        .remove(&PauseKey::UserPause(user, action));
}

// ── Query helpers ──────────────────────────────────────────────────────────

/// Returns true if the `All` sentinel is set — O(1) global pause check.
pub fn is_globally_paused(env: &Env) -> bool {
    let state: PauseState = env
        .storage()
        .instance()
        .get(&PauseKey::Action(PauseAction::All))
        .unwrap_or(PauseState::NotPaused);
    state == PauseState::Paused
}

/// Returns true if `action` is paused globally OR if `user` is individually
/// blocked for that action.
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

/// Convenience guard: panics with a descriptive message if `action` is paused.
/// Call at the top of every public mutator in `lib.rs`.
pub fn require_not_paused(env: Env, action: PauseAction, user: Option<Address>) {
    if is_paused(env, action, user) {
        panic!("action is currently paused by admin");
    }
}
