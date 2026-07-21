# #017: Storage Key Collision Detection & Static Analysis Tooling

## Overview
Develop static analysis linting in `tooling/` to detect storage key symbol collisions across contract modules.

## Objectives
- Create static analyzer that inspects storage enum variants and key generation symbols.
- Prevent accidental storage overwrites between independent modules (e.g. escrow vs admin state).
- Integrate tool into local build scripts and CI checks.

## Acceptance Criteria
- [ ] Linter script created under `tooling/check_storage_keys.rs` or Python script.
- [ ] Run automated scan against all `DataKey` enum definitions.
- [ ] Fail build if duplicate storage key symbols are detected.
