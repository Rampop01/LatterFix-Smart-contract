# #049: Off-Chain Snapshot Verifier & Merkle Proof Vault Claim

## Overview
Implement Merkle root storage and proof verification to allow mass employee claims from compressed off-chain payroll trees.

## Objectives
- Store Merkle root of mass payroll snapshot on-chain.
- Allow employees to submit Merkle inclusion proof to claim salary.
- Drastically reduce contract storage requirements for large enterprise payroll runs.

## Acceptance Criteria
- [ ] Merkle proof verifier helper function `verify_merkle_proof`.
- [ ] Claim handler validating proof against stored Merkle root.
- [ ] Unit tests verifying valid and forged Merkle proof claims.
