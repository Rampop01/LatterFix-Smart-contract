# #047: Sanctioned Account Check & Dynamic Blacklist Registry

## Overview
Add dynamic address blacklist registry to block prohibited accounts from receiving payroll disbursements.

## Objectives
- Allow authorized compliance admins to add/remove blocked addresses from contract storage.
- Check recipient addresses against blacklist prior to token release.
- Revert or divert blocked payouts to compliance escrow.

## Acceptance Criteria
- [ ] Blacklist storage mapping and `is_blacklisted` guard.
- [ ] `add_to_blacklist` and `remove_from_blacklist` admin handlers.
- [ ] Unit tests verifying payment blockage for blacklisted accounts.
