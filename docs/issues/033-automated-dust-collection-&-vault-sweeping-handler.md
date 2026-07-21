# #033: Automated Dust Collection & Vault Sweeping Handler

## Overview
Provide administrative sweeping handler to collect leftover sub-cent token dust in closed vaults.

## Objectives
- Identify inactive/closed escrow vaults with residual dust balances below payout threshold.
- Allow authorized admin to sweep accumulated dust into protocol treasury.
- Emit sweep telemetry event for financial auditing.

## Acceptance Criteria
- [ ] `sweep_dust` method added with threshold validation guard.
- [ ] Rejection of sweep attempts on active escrow vaults.
- [ ] Unit test verifying complete vault cleanup.
