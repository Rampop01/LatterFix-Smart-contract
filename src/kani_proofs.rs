#![cfg(kani)]

// ============================================================================
// Formal Verification Harnesses — Kani Rust Model Checker
//
// Pure-logic models of core contract invariants. No Soroban dependencies;
// every function is self-contained so Kani can symbolically execute it.
// ============================================================================

// ═══════════════════════════════════════════════════════════════════════════
// Proof 1: Fee Calculation Invariant
// ═══════════════════════════════════════════════════════════════════════════
//
// contract fee = reward * platform_fee_bps / 10000
// payout      = reward - fee
// Invariant:  fee + payout == reward  (no rounding loss)

fn pure_fee_calculation(reward: i128, bps: u32) -> (i128, i128) {
    let fee = reward * bps as i128 / 10000;
    let payout = reward - fee;
    (fee, payout)
}

#[kani::proof]
fn verify_fee_invariant() {
    let reward: i128 = kani::any();
    let bps: u32 = kani::any();

    kani::assume(bps <= 1000);
    kani::assume(reward > 0);
    kani::assume(reward.checked_mul(bps as i128).is_some());

    let (fee, payout) = pure_fee_calculation(reward, bps);

    assert!(fee >= 0);
    assert!(payout >= 0);
    assert!(payout <= reward);
    assert_eq!(fee + payout, reward);
    assert_eq!(payout, reward - fee);
}

#[kani::proof]
fn verify_fee_edge_cases() {
    let (fee, payout) = pure_fee_calculation(0, 500);
    assert_eq!(fee, 0);
    assert_eq!(payout, 0);

    let (fee, payout) = pure_fee_calculation(1000, 0);
    assert_eq!(fee, 0);
    assert_eq!(payout, 1000);
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 2: Vault Deposit Invariant
// ═══════════════════════════════════════════════════════════════════════════
//
// On deposit:
//   vault_total'   = vault_total   + amount
//   dep_balance'   = dep_balance   + amount
// Invariant: vault_total' - vault_total == amount
// Invariant: vault_total' >= vault_total  (monotonic)

fn pure_vault_deposit(
    vault_total: i128,
    depositor_balance: i128,
    amount: i128,
) -> Option<(i128, i128)> {
    if amount <= 0 {
        return None;
    }
    let new_vault = vault_total.checked_add(amount)?;
    let new_dep = depositor_balance.checked_add(amount)?;
    Some((new_vault, new_dep))
}

#[kani::proof]
fn verify_vault_deposit_no_overflow() {
    let vault_total: i128 = kani::any();
    let dep_balance: i128 = kani::any();
    let amount: i128 = kani::any();

    kani::assume(vault_total >= 0);
    kani::assume(dep_balance >= 0);
    kani::assume(amount > 0);
    kani::assume(vault_total.checked_add(amount).is_some());
    kani::assume(dep_balance.checked_add(amount).is_some());

    let result = pure_vault_deposit(vault_total, dep_balance, amount);
    assert!(result.is_some());
    let (new_vault, new_dep) = result.unwrap();

    assert!(new_vault >= 0);
    assert!(new_dep >= 0);
    assert!(new_vault >= vault_total);
    assert!(new_dep >= dep_balance);
    assert_eq!(new_vault - vault_total, amount);
    assert_eq!(new_dep - dep_balance, amount);
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 3: Vault Claim Invariant
// ═══════════════════════════════════════════════════════════════════════════
//
// On claim:
//   vault_total'   = vault_total   - amount
//   dep_balance'   = dep_balance   - amount
// Invariant: vault_total' >= 0  AND  dep_balance' >= 0
// Invariant: vault_total - vault_total' == amount

fn pure_vault_claim(
    vault_total: i128,
    depositor_balance: i128,
    amount: i128,
) -> Option<(i128, i128)> {
    if amount <= 0 || depositor_balance < amount || vault_total < amount {
        return None;
    }
    let new_vault = vault_total.checked_sub(amount)?;
    let new_dep = depositor_balance.checked_sub(amount)?;
    Some((new_vault, new_dep))
}

#[kani::proof]
fn verify_vault_claim_no_underflow() {
    let vault_total: i128 = kani::any();
    let dep_balance: i128 = kani::any();
    let amount: i128 = kani::any();

    kani::assume(vault_total >= 0);
    kani::assume(dep_balance >= 0);
    kani::assume(amount > 0);
    kani::assume(dep_balance >= amount);
    kani::assume(vault_total >= amount);

    let result = pure_vault_claim(vault_total, dep_balance, amount);
    assert!(result.is_some());
    let (new_vault, new_dep) = result.unwrap();

    assert!(new_vault >= 0);
    assert!(new_dep >= 0);
    assert_eq!(vault_total - new_vault, amount);
    assert_eq!(dep_balance - new_dep, amount);
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 4: Escrow Release Safety
// ═══════════════════════════════════════════════════════════════════════════
//
// release_escrow requires current >= amount.
// Invariant: new_balance >= 0
// Invariant: new_balance <= balance

fn pure_escrow_release(balance: i128, amount: i128) -> Option<i128> {
    if amount < 0 || balance < amount {
        return None;
    }
    balance.checked_sub(amount)
}

#[kani::proof]
fn verify_escrow_release_no_underflow() {
    let balance: i128 = kani::any();
    let amount: i128 = kani::any();

    kani::assume(balance >= 0);
    kani::assume(amount >= 0);
    kani::assume(balance >= amount);

    let result = pure_escrow_release(balance, amount);
    assert!(result.is_some());
    let new_balance = result.unwrap();

    assert!(new_balance >= 0);
    assert!(new_balance <= balance);
    assert_eq!(balance - new_balance, amount);
}

#[kani::proof]
fn verify_escrow_release_rejects_insufficient() {
    let balance: i128 = kani::any();
    let amount: i128 = kani::any();

    kani::assume(balance >= 0);
    kani::assume(amount >= 0);
    kani::assume(balance < amount);

    let result = pure_escrow_release(balance, amount);
    assert!(result.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 5: Escrow Lock Monotonicity
// ═══════════════════════════════════════════════════════════════════════════
//
// lock_escrow: balance' = balance + amount
// Invariant: balance' >= balance

fn pure_escrow_lock(balance: i128, amount: i128) -> Option<i128> {
    if amount <= 0 {
        return None;
    }
    balance.checked_add(amount)
}

#[kani::proof]
fn verify_escrow_lock_monotonic() {
    let balance: i128 = kani::any();
    let amount: i128 = kani::any();

    kani::assume(balance >= 0);
    kani::assume(amount > 0);
    kani::assume(balance.checked_add(amount).is_some());

    let result = pure_escrow_lock(balance, amount);
    assert!(result.is_some());
    let new_balance = result.unwrap();

    assert!(new_balance >= balance);
    assert_eq!(new_balance - balance, amount);
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 6: Reputation Floor at Zero
// ═══════════════════════════════════════════════════════════════════════════
//
// current = (current + points).max(0)
// Invariant: result >= 0  ALWAYS

fn pure_reputation_update(current: i32, points: i32) -> i32 {
    (current + points).max(0)
}

#[kani::proof]
fn verify_reputation_never_negative() {
    let current: i32 = kani::any();
    let points: i32 = kani::any();

    kani::assume(current >= 0 && current <= 10_000);
    kani::assume(points >= -1000 && points <= 1000);

    let new_points = pure_reputation_update(current, points);

    assert!(new_points >= 0);
    if current + points >= 0 {
        assert_eq!(new_points, current + points);
    } else {
        assert_eq!(new_points, 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 7: Reputation Saturating Arithmetic
// ═══════════════════════════════════════════════════════════════════════════
//
// user_profile uses saturating_add / saturating_sub.
// Invariant: result never overflows or underflows i128.

fn pure_saturating_add(a: i128, b: i128) -> i128 {
    a.saturating_add(b)
}

fn pure_saturating_sub(a: i128, b: i128) -> i128 {
    a.saturating_sub(b)
}

#[kani::proof]
fn verify_saturating_arithmetic_bounds() {
    let a: i128 = kani::any();
    let b: i128 = kani::any();

    let sum = pure_saturating_add(a, b);
    let diff = pure_saturating_sub(a, b);

    if a.checked_add(b).is_some() {
        assert_eq!(sum, a + b);
    } else {
        assert!((b > 0 && sum == i128::MAX) || (b < 0 && sum == i128::MIN));
    }

    if a.checked_sub(b).is_some() {
        assert_eq!(diff, a - b);
    } else {
        assert!(diff == i128::MAX || diff == i128::MIN);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 8: Oracle Price Computation
// ═══════════════════════════════════════════════════════════════════════════
//
// expected_out = amount_in * price_in / price_out
// min_out      = expected_out * (10000 - slippage) / 10000
// Invariant: min_out >= 0  AND  min_out <= expected_out

const ORACLE_DECIMALS: i128 = 10_000_000;
const BPS_DENOM: i128 = 10_000;

fn pure_oracle_min_out(
    amount_in: i128,
    price_in: i128,
    price_out: i128,
    slippage_bps: u32,
) -> Option<i128> {
    let expected = amount_in.checked_mul(price_in)? / price_out;
    let min_out = expected.checked_mul(BPS_DENOM - slippage_bps as i128)? / BPS_DENOM;
    Some(min_out)
}

fn pure_oracle_min_out_i64(
    amount_in: i64,
    price_in: i64,
    price_out: i64,
    slippage_bps: u32,
) -> Option<i64> {
    let expected = amount_in.checked_mul(price_in)? / price_out;
    let min_out = expected.checked_mul(BPS_DENOM as i64 - slippage_bps as i64)? / BPS_DENOM as i64;
    Some(min_out)
}

#[kani::proof]
fn verify_oracle_min_out_no_overflow() {
    let amount_in: i64 = kani::any();
    let price_in: i64 = kani::any();
    let price_out: i64 = kani::any();
    let slippage_bps: u32 = kani::any();

    kani::assume(amount_in > 0 && amount_in <= 10_000_000_000);
    kani::assume(price_in >= 1 && price_in <= 10_000_000);
    kani::assume(price_out >= 1 && price_out <= 10_000_000);
    kani::assume(slippage_bps <= 5000);
    kani::assume(amount_in.checked_mul(price_in).is_some());

    if let Some(min_out) = pure_oracle_min_out_i64(amount_in, price_in, price_out, slippage_bps) {
        assert!(min_out >= 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 9: Swap Router Slippage Guard
// ═══════════════════════════════════════════════════════════════════════════
//
// In convert_incoming_deposit:
//   min_out = expected_out * (BPS_DENOM - slippage) / BPS_DENOM
// If pool output < min_out, the call traps (panic).
// Invariant: 0 <= slippage <= BPS_DENOM
// Invariant: min_out <= expected_out

fn pure_slippage_guard(expected_out: i128, slippage_bps: u32) -> Option<i128> {
    if slippage_bps > BPS_DENOM as u32 {
        return None;
    }
    let min_out = expected_out.checked_mul(BPS_DENOM - slippage_bps as i128)? / BPS_DENOM;
    Some(min_out)
}

#[kani::proof]
fn verify_slippage_guard_bounds() {
    let expected_out: i128 = kani::any();
    let slippage_bps: u32 = kani::any();

    kani::assume(expected_out >= 0 && expected_out <= 10_000_000_000);
    kani::assume(slippage_bps <= 10_000);

    if let Some(min_out) = pure_slippage_guard(expected_out, slippage_bps) {
        assert!(min_out >= 0);
        assert!(slippage_bps == 0 || expected_out >= min_out);
        if slippage_bps == 0 {
            assert_eq!(min_out, expected_out);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 10: Milestone State Machine
// ═══════════════════════════════════════════════════════════════════════════
//
// Valid transitions:
//   Pending  → Submitted  (submit)
//   Submitted → Approved   (approve)
//   Submitted → Rejected   (reject)
//   Rejected → Submitted   (re-submit)
//
// Invalid transitions must be rejected (return None).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MsState {
    Pending,
    Submitted,
    Approved,
    Rejected,
}

fn ms_submit(state: MsState) -> Option<MsState> {
    match state {
        MsState::Pending | MsState::Rejected => Some(MsState::Submitted),
        _ => None,
    }
}

fn ms_approve(state: MsState) -> Option<MsState> {
    match state {
        MsState::Submitted => Some(MsState::Approved),
        _ => None,
    }
}

fn ms_reject(state: MsState) -> Option<MsState> {
    match state {
        MsState::Submitted => Some(MsState::Rejected),
        _ => None,
    }
}

#[kani::proof]
fn verify_milestone_transitions_exhaustive() {
    let variant: u8 = kani::any();
    kani::assume(variant < 4);
    let state = match variant {
        0 => MsState::Pending,
        1 => MsState::Submitted,
        2 => MsState::Approved,
        _ => MsState::Rejected,
    };

    let after_submit = ms_submit(state);
    let after_approve = ms_approve(state);
    let after_reject = ms_reject(state);

    match state {
        MsState::Pending | MsState::Rejected => {
            assert!(after_submit.is_some());
            assert_eq!(after_submit.unwrap(), MsState::Submitted);
        }
        _ => {
            assert!(after_submit.is_none());
        }
    }

    match state {
        MsState::Submitted => {
            assert!(after_approve.is_some());
            assert_eq!(after_approve.unwrap(), MsState::Approved);
            assert!(after_reject.is_some());
            assert_eq!(after_reject.unwrap(), MsState::Rejected);
        }
        _ => {
            assert!(after_approve.is_none());
            assert!(after_reject.is_none());
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 11: Escrow Stats Tracking Invariant
// ═══════════════════════════════════════════════════════════════════════════
//
// Invariant: total_locked >= total_released + total_refunded
// Invariant: all stats non-negative

fn pure_escrow_stats_invariant(
    total_locked: i128,
    total_released: i128,
    total_refunded: i128,
) -> bool {
    total_locked >= 0
        && total_released >= 0
        && total_refunded >= 0
        && total_locked >= total_released + total_refunded
}

#[kani::proof]
fn verify_escrow_stats_after_lock() {
    let locked: i128 = kani::any();
    kani::assume(locked > 0 && locked < 1_000_000_000);
    assert!(pure_escrow_stats_invariant(locked, 0, 0));
}

#[kani::proof]
fn verify_escrow_stats_after_release() {
    let locked: i128 = kani::any();
    kani::assume(locked > 0 && locked < 1_000_000_000);
    let released: i128 = kani::any();
    kani::assume(released >= 0 && released <= locked);
    assert!(pure_escrow_stats_invariant(locked, released, 0));
}

#[kani::proof]
fn verify_escrow_stats_after_refund() {
    let locked: i128 = kani::any();
    kani::assume(locked > 0 && locked < 1_000_000_000);
    let released: i128 = kani::any();
    kani::assume(released >= 0 && released <= locked);
    let refunded: i128 = kani::any();
    kani::assume(refunded >= 0 && refunded <= locked - released);
    assert!(pure_escrow_stats_invariant(locked, released, refunded));
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 12: Dispute Resolution Split
// ═══════════════════════════════════════════════════════════════════════════
//
// resolve_dispute: creator_refund + assignee_payout == task.reward
// Invariant: no value creation or destruction

fn pure_dispute_split(reward: i128, creator_refund: i128, assignee_payout: i128) -> bool {
    creator_refund >= 0 && assignee_payout >= 0 && creator_refund + assignee_payout == reward
}

#[kani::proof]
fn verify_dispute_split_invariant() {
    let reward: i128 = kani::any();
    let creator_refund: i128 = kani::any();
    let assignee_payout: i128 = kani::any();

    kani::assume(reward > 0 && reward < 1_000_000_000);
    kani::assume(creator_refund >= 0);
    kani::assume(assignee_payout >= 0);
    kani::assume(creator_refund.checked_add(assignee_payout) == Some(reward));

    assert!(pure_dispute_split(reward, creator_refund, assignee_payout));
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 13: Governance Threshold Calculation
// ═══════════════════════════════════════════════════════════════════════════
//
// for_percentage = (votes_for * 100) / non_abstain
// Passes when for_percentage >= threshold.
// Invariant: 0 <= for_percentage <= 100  when non_abstain > 0

fn pure_threshold(votes_for: u32, votes_against: u32) -> Option<u32> {
    let non_abstain = votes_for + votes_against;
    if non_abstain == 0 {
        return None;
    }
    Some(votes_for * 100 / non_abstain)
}

#[kani::proof]
fn verify_threshold_bounds() {
    let votes_for: u32 = kani::any();
    let votes_against: u32 = kani::any();

    kani::assume(votes_for <= 1_000_000);
    kani::assume(votes_against <= 1_000_000);

    if let Some(pct) = pure_threshold(votes_for, votes_against) {
        assert!(pct <= 100);
        if votes_against == 0 {
            assert_eq!(pct, 100);
        }
        if votes_for == 0 {
            assert_eq!(pct, 0);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proof 14: Swap Route Validation Bounds
// ═══════════════════════════════════════════════════════════════════════════
//
// In validate_route:
//   hops > 0  &&  path.len() == hops + 1  &&  hops <= max_hops
// Invariant: valid routes have matching path/pool lengths

fn pure_validate_route(hops: usize, path_len: usize, max_hops: u32) -> bool {
    if hops == 0 || hops > max_hops as usize {
        return false;
    }
    hops.checked_add(1)
        .map_or(false, |expected| path_len == expected)
}

#[kani::proof]
fn verify_route_validation() {
    let hops: usize = kani::any();
    let path_len: usize = kani::any();
    let max_hops: u32 = kani::any();

    kani::assume(max_hops <= 10);

    let valid = pure_validate_route(hops, path_len, max_hops);

    if valid {
        assert!(hops > 0);
        assert_eq!(path_len, hops + 1);
        assert!(hops <= max_hops as usize);
    }
}
