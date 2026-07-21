# #023: Event Schema Indexing Specification & Topic Standardization

## Overview
Standardize Soroban event topics (`Symbol`, `Address`, `Val`) to streamline off-chain backend indexer consumption.

## Objectives
- Align event payload structures across all smart contract functions.
- Create explicit event dictionary documentation for backend and indexer developers.
- Ensure event topics strictly adhere to Soroban indexing conventions.

## Acceptance Criteria
- [ ] Standardized event publishing helper functions.
- [ ] Event dictionary specification document added to `docs/`.
- [ ] Integration test verifying event payload format.
