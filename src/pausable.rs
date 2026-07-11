use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PauseState {
    NotPaused,
    Paused,
    PausedForUser, // Simplified - use separate mapping for user-specific pauses
}

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
    All,
}

#[contracttype]
pub enum PauseKey {
    Action(PauseAction),
    UserPause(Address, PauseAction),
}

pub fn pause(env: Env, _admin: Address, action: PauseAction) {
    let key = PauseKey::Action(action);
    env.storage().instance().set(&key, &PauseState::Paused);
}

pub fn unpause(env: Env, _admin: Address, action: PauseAction) {
    let key = PauseKey::Action(action);
    env.storage().instance().set(&key, &PauseState::NotPaused);
}

pub fn pause_for_user(env: Env, _admin: Address, user: Address, action: PauseAction) {
    let key = PauseKey::UserPause(user, action);
    env.storage().instance().set(&key, &true);
}

pub fn unpause_for_user(env: Env, _admin: Address, user: Address, action: PauseAction) {
    let key = PauseKey::UserPause(user, action);
    env.storage().instance().remove(&key);
}

pub fn is_paused(env: Env, action: PauseAction, user: Option<Address>) -> bool {
    // Check global pause first
    let global_key = PauseKey::Action(action);
    let global_state: PauseState = env
        .storage()
        .instance()
        .get(&global_key)
        .unwrap_or(PauseState::NotPaused);
    
    if global_state == PauseState::Paused {
        return true;
    }
    
    // Check user-specific pause
    if let Some(u) = user {
        let user_key = PauseKey::UserPause(u, action);
        env.storage().instance().get(&user_key).unwrap_or(false)
    } else {
        false
    }
}

pub fn require_not_paused(env: Env, action: PauseAction, user: Option<Address>) {
    if is_paused(env.clone(), action, user) {
        panic!("contract action is paused");
    }
}

pub fn pause_all(env: Env, admin: Address) {
    for action in [
        PauseAction::CreateTask,
        PauseAction::AssignTask,
        PauseAction::SubmitWork,
        PauseAction::CompleteTask,
        PauseAction::CancelTask,
        PauseAction::DisputeTask,
        PauseAction::Withdraw,
    ] {
        pause(env.clone(), admin.clone(), action);
    }
}

pub fn unpause_all(env: Env, admin: Address) {
    for action in [
        PauseAction::CreateTask,
        PauseAction::AssignTask,
        PauseAction::SubmitWork,
        PauseAction::CompleteTask,
        PauseAction::CancelTask,
        PauseAction::DisputeTask,
        PauseAction::Withdraw,
    ] {
        unpause(env.clone(), admin.clone(), action);
    }
}
