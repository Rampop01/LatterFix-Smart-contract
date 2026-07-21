# #028: Custom Error Code Registry & Macro Standardization

## Overview
Refactor contract error enums into a centralized, standardized `ContractError` registry across all contract modules.

## Objectives
- Eliminate duplicate error symbols and numeric conflicts across modules.
- Add descriptive Rustdoc comments explaining cause and recovery for each error code.
- Implement ergonomic error helper macros for guard assertions.

## Acceptance Criteria
- [ ] Central `errors.rs` module with standardized `#[contracterror]` enum.
- [ ] Replace ad-hoc panic calls with typed contract errors.
- [ ] All existing tests updated and passing cleanly.
