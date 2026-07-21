# #036: Automated Re-Entrancy & Double-Claim Fuzz Testing Pipeline

## Overview
Set up property-based fuzz testing using `cargo-fuzz` / `proptest` to test arbitrary invocation sequences.

## Objectives
- Generate randomized claim sequences, deposit amounts, and withdrawal timings.
- Verify system invariants contract token balance >= unreleased escrow balance hold under all inputs.
- Detect hidden edge-case crashes or arithmetic overflows automatically.

## Acceptance Criteria
- [ ] Fuzz target script created in `fuzz/fuzz_targets/escrow_fuzz.rs`.
- [ ] Run fuzzing pipeline for 10,000+ iterations without invariant violations.
- [ ] Documentation for running fuzz tests locally.
