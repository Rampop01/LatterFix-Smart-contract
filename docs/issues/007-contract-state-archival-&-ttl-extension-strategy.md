# #007: Contract State Archival & TTL Extension Strategy

## Overview
Implement TTL (Time-To-Live) bumping for persistent storage keys (`storage.rs`) to prevent state archival on Soroban mainnet.

## Objectives
- Add automatic TTL extension logic (`env.storage().persistent().extend_ttl(...)`) on active storage reads/writes.
- Define minimum and maximum ledger thresholds for contract storage keys.
- Create explicit bump helper methods accessible by contract maintenance bots.

## Acceptance Criteria
- [ ] Storage key helper functions upgraded with TTL extension logic.
- [ ] Automated TTL bump utility function exposed for admin invocations.
- [ ] Documentation and tests verifying state persistence retention.
