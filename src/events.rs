use soroban_sdk::{Address, Env, String, symbol_short};

/// Event types for the TaskManager contract
/// Using Soroban's event system for off-chain indexing

// Task Events
pub fn emit_task_created(
    env: &Env,
    task_id: u32,
    creator: Address,
    title: String,
    reward: i128,
) {
    env.events().publish(
        (symbol_short!("task_cre"), task_id),
        (creator, title, reward),
    );
}

pub fn emit_task_assigned(
    env: &Env,
    task_id: u32,
    assignee: Address,
) {
    env.events().publish(
        (symbol_short!("task_assg"), task_id),
        assignee,
    );
}

pub fn emit_task_submitted(
    env: &Env,
    task_id: u32,
    assignee: Address,
    delivery_url: String,
) {
    env.events().publish(
        (symbol_short!("task_subm"), task_id),
        (assignee, delivery_url),
    );
}

pub fn emit_task_completed(
    env: &Env,
    task_id: u32,
    assignee: Address,
    payout: i128,
    fee: i128,
) {
    env.events().publish(
        (symbol_short!("task_comp"), task_id),
        (assignee, payout, fee),
    );
}

pub fn emit_task_cancelled(
    env: &Env,
    task_id: u32,
    creator: Address,
    refund: i128,
) {
    env.events().publish(
        (symbol_short!("task_canc"), task_id),
        (creator, refund),
    );
}

pub fn emit_task_disputed(
    env: &Env,
    task_id: u32,
    caller: Address,
) {
    env.events().publish(
        (symbol_short!("task_disp"), task_id),
        caller,
    );
}

pub fn emit_dispute_resolved(
    env: &Env,
    task_id: u32,
    creator_refund: i128,
    assignee_payout: i128,
) {
    env.events().publish(
        (symbol_short!("disp_resl"), task_id),
        (creator_refund, assignee_payout),
    );
}

// Profile Events
pub fn emit_profile_created(
    env: &Env,
    user: Address,
    username: String,
) {
    env.events().publish(
        (symbol_short!("prof_cre"), user),
        username,
    );
}

pub fn emit_profile_updated(
    env: &Env,
    user: Address,
    field: String,
) {
    env.events().publish(
        (symbol_short!("prof_upd"), user),
        field,
    );
}

pub fn emit_reputation_awarded(
    env: &Env,
    user: Address,
    points: u32,
    new_total: u32,
) {
    env.events().publish(
        (symbol_short!("rep_award"), user),
        (points, new_total),
    );
}

// Milestone Events
pub fn emit_milestone_created(
    env: &Env,
    task_id: u32,
    milestone_id: u32,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("mile_cre"), (task_id, milestone_id)),
        amount,
    );
}

pub fn emit_milestone_submitted(
    env: &Env,
    task_id: u32,
    milestone_id: u32,
    assignee: Address,
) {
    env.events().publish(
        (symbol_short!("mile_subm"), (task_id, milestone_id)),
        assignee,
    );
}

pub fn emit_milestone_approved(
    env: &Env,
    task_id: u32,
    milestone_id: u32,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("mile_appr"), (task_id, milestone_id)),
        amount,
    );
}

pub fn emit_milestone_rejected(
    env: &Env,
    task_id: u32,
    milestone_id: u32,
    feedback: String,
) {
    env.events().publish(
        (symbol_short!("mile_rej"), (task_id, milestone_id)),
        feedback,
    );
}

// Governance Events
pub fn emit_proposal_created(
    env: &Env,
    proposal_id: u32,
    proposer: Address,
    title: String,
) {
    env.events().publish(
        (symbol_short!("prop_cre"), proposal_id),
        (proposer, title),
    );
}

pub fn emit_vote_cast(
    env: &Env,
    proposal_id: u32,
    voter: Address,
    vote_type: String,
    weight: u32,
) {
    env.events().publish(
        (symbol_short!("vote_cast"), (proposal_id, voter)),
        (vote_type, weight),
    );
}

pub fn emit_proposal_executed(
    env: &Env,
    proposal_id: u32,
    passed: bool,
) {
    env.events().publish(
        (symbol_short!("prop_exec"), proposal_id),
        passed,
    );
}

// Access Control Events
pub fn emit_role_granted(
    env: &Env,
    user: Address,
    role: String,
    granted_by: Address,
) {
    env.events().publish(
        (symbol_short!("role_gr"), user),
        (role, granted_by),
    );
}

pub fn emit_role_revoked(
    env: &Env,
    user: Address,
    role: String,
    revoked_by: Address,
) {
    env.events().publish(
        (symbol_short!("role_rev"), user),
        (role, revoked_by),
    );
}

// Pause Events
pub fn emit_paused(
    env: &Env,
    action: String,
    admin: Address,
) {
    env.events().publish(
        (symbol_short!("paused"), action),
        admin,
    );
}

pub fn emit_unpaused(
    env: &Env,
    action: String,
    admin: Address,
) {
    env.events().publish(
        (symbol_short!("unpaused"), action),
        admin,
    );
}

// Transfer Events
pub fn emit_tokens_locked(
    env: &Env,
    task_id: u32,
    from: Address,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("lock"), task_id),
        (from, amount),
    );
}

pub fn emit_tokens_released(
    env: &Env,
    task_id: u32,
    to: Address,
    amount: i128,
) {
    env.events().publish(
        (symbol_short!("release"), task_id),
        (to, amount),
    );
}
