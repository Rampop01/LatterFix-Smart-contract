# #052: Property-Based Testing Harness with proptest-rs

## Overview
Implement property-based tests verifying key state invariants across arbitrary random inputs.

## Objectives
- Test mathematical properties such as idempotency, commutativity of deposit order, and conservation of balance.
- Use `proptest` crate to generate hundreds of test cases automatically per test run.
- Catch edge cases that manual unit tests miss.

## Acceptance Criteria
- [ ] Property test module added under `src/property_tests.rs`.
- [ ] Invariants verified across 1,000 random inputs.
- [ ] All tests passing cleanly in `cargo test`.
