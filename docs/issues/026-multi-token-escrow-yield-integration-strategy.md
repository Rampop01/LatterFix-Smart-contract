# #026: Multi-Token Escrow Yield Integration Strategy

## Overview
Implement yield-generating vault integration for tokens held in payroll lockup prior to distribution.

## Objectives
- Allow escrowed funds to earn yield via integrated Soroban yield pools while waiting for payout release dates.
- Ensure principal salary balance is strictly protected and isolated from yield fluctuations.
- Distribute earned interest back to employer account or protocol fee collector on payout completion.

## Acceptance Criteria
- [ ] Vault wrapper supporting yield pool deposit and redeem operations.
- [ ] Principal preservation guard asserting withdrawal amount >= original deposit.
- [ ] Unit tests for yield allocation calculation.
