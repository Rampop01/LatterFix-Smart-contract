## Summary

Closes #93. Implements a decay-curve reward treasury vesting & distribution manager for the protocol's community incentive pool.

- **Treasury balance management** — `configure_treasury` sets the SAC reward token (admin-only); `fund_treasury` deposits into a tracked treasury balance.
- **Decay-based release scheduler** — `create_vesting_schedule(admin, beneficiary, total_amount, start_time, cliff_seconds, period_seconds, decay_rate_bps)` schedules a per-beneficiary vesting release. Each elapsed period releases a fixed percentage of whatever remains unvested (`vested(n) = total_amount * (1 - (decay_rate_bps / 10000)^n)`), so payout is front-loaded and tapers off — matching how emission-style incentive programs (liquidity mining, early-contributor rewards) typically want to pay out, rather than a straight linear ramp.
- **Over-allocation guards** — schedule creation is rejected if it would push allocated totals past the treasury's actually-funded balance; claims are capped so a schedule's `claimed_amount` can never exceed its `total_amount`, and a live treasury-balance check runs before every payout.
- **Safe withdrawal handler** — `claim_vesting` requires beneficiary auth, follows checks-effects-interactions (state updated before the token transfer), and pays out only the newly-vested delta since the last claim.

## Changes

- `src/treasury.rs` — new module: treasury balance ledger, vesting schedule storage, decay curve math (`decay_retained_bps`, bounded by `MAX_DECAY_PERIODS` to keep the computation cost fixed regardless of how long a schedule has been left unclaimed), and the fund/create/claim entry points.
- `src/events.rs` — `emit_treasury_funded`, `emit_vesting_schedule_created`, `emit_vesting_claimed`.
- `src/lib.rs` — registers the module and exposes it on `TaskManagerContract`: `configure_treasury`, `fund_treasury`, `create_vesting_schedule`, `claim_vesting`, `get_vested_amount`, `get_claimable_amount`, `get_vesting_schedule`, `get_beneficiary_schedules`, `get_treasury_balance`, `get_treasury_allocated`.
- `src/treasury_test.rs` — 19 tests covering the decay curve math directly and end-to-end contract behavior.

## Test plan

- [x] `cargo test --lib` — 101 passed, 0 failed (19 new treasury tests + all pre-existing tests untouched).
- [x] Unit tests verify exact decay curve values (e.g. 50% retention per period: 500 / 750 / 875 / 937 vested out of 1000 at periods 1–4).
- [x] Tests verify the cliff, over-allocation rejection at schedule creation, non-admin/non-beneficiary rejection, partial double-claims, and full-vesting claim exhaustion.
- [x] `cargo clippy --lib --tests` — no new warnings beyond pre-existing repo-wide lints.
