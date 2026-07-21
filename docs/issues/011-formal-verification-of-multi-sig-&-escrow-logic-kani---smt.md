# #011: Formal Verification of Multi-Sig & Escrow Logic (Kani / SMT)

## Overview
Add Kani model checking harnesses and formal verification specs for token initialization and balance invariants.

## Objectives
- Build formal proof harnesses verifying no unlocked funds can be withdrawn by non-owners.
- Verify mathematically that contract balance equals sum of vault deposits minus payouts.
- Detect potential integer overflow/underflow or state machine flaws.

## Acceptance Criteria
- [ ] Kani verification harness added under `src/kani_proofs.rs`.
- [ ] Proofs passing without counterexamples.
- [ ] CI integration for formal verification checks.
