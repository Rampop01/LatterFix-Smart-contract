# #042: Vesting Cliff Schedule Calculator & Partial Release Handler

## Overview
Implement cliff vesting calculation logic supporting linear unlock schedules for employee equity/bonus tokens.

## Objectives
- Support configurable cliff duration e.g. 6 or 12 months followed by monthly linear vesting.
- Calculate claimable unlocked amount precisely based on current ledger timestamp.
- Prevent premature claims before cliff expiry.

## Acceptance Criteria
- [ ] Vesting schedule struct cliff_timestamp, end_timestamp, total_amount.
- [ ] `calculate_unlocked_amount` helper function verified.
- [ ] Unit tests for pre-cliff, mid-vesting, and post-vesting claims.
