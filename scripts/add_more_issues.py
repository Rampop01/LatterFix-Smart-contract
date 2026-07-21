import os
import json
import time
import urllib.parse
import requests

creds_file = os.path.expanduser('~/.git-credentials')
if not os.path.exists(creds_file):
    raise RuntimeError("~/.git-credentials file not found!")

with open(creds_file) as f:
    line = f.readline().strip()

parsed = urllib.parse.urlparse(line)
token = parsed.password
headers = {
    'Authorization': f'token {token}',
    'Accept': 'application/vnd.github.v3+json'
}

REPO_OWNER = "LatterFixxx"
REPO_NAME = "LatterFix-Smart-contract"
ISSUES_DIR = os.path.join(os.getcwd(), "docs", "issues")

os.makedirs(ISSUES_DIR, exist_ok=True)

NEW_ISSUES = [
    {
        "id": "026",
        "title": "Multi-Token Escrow Yield Integration Strategy",
        "labels": ["smart-contract", "defi", "medium"],
        "body": """# #026: Multi-Token Escrow Yield Integration Strategy

## Overview
Implement yield-generating vault integration for tokens held in payroll lockup prior to distribution.

## Objectives
- Allow escrowed funds to earn yield via integrated Soroban yield pools while waiting for payout release dates.
- Ensure principal salary balance is strictly protected and isolated from yield fluctuations.
- Distribute earned interest back to employer account or protocol fee collector on payout completion.

## Acceptance Criteria
- [ ] Vault wrapper supporting yield pool deposit and redeem operations.
- [ ] Principal preservation guard asserting withdrawal amount >= original deposit.
- [ ] Unit tests for yield allocation calculation.
"""
    },
    {
        "id": "027",
        "title": "Zero-Knowledge Proof ZKP Identity Attestation Module",
        "labels": ["smart-contract", "privacy", "hard"],
        "body": """# #027: Zero-Knowledge Proof ZKP Identity Attestation Module

## Overview
Integrate zero-knowledge proof verification logic for private identity attestation during payroll processing.

## Objectives
- Allow employees to prove eligibility / KYC status without revealing underlying identity data on-chain.
- Verify zk-SNARK / Groth16 proof payloads inside Soroban host environment.
- Reject invalid or replayed ZK proofs.

## Acceptance Criteria
- [ ] ZK proof verifier host binding wrapper created.
- [ ] Nullifier tracking to prevent ZK proof replay attacks.
- [ ] Unit tests checking valid and invalid proof attestations.
"""
    },
    {
        "id": "028",
        "title": "Custom Error Code Registry & Macro Standardization",
        "labels": ["smart-contract", "refactoring", "easy"],
        "body": """# #028: Custom Error Code Registry & Macro Standardization

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
"""
    },
    {
        "id": "029",
        "title": "Cross-Chain Message Relayer & Bridge Receiver Guard",
        "labels": ["smart-contract", "bridge", "hard"],
        "body": """# #029: Cross-Chain Message Relayer & Bridge Receiver Guard

## Overview
Build contract interface to accept authorized cross-chain payment instructions from external EVM / Stacks bridges.

## Objectives
- Verify bridge relayer proof payload and source domain identifiers.
- Prevent unauthorized cross-chain message execution or malicious parameter injection.
- Process cross-chain payroll deposits seamlessly into Soroban vaults.

## Acceptance Criteria
- [ ] Cross-chain message decoder and relayer auth check.
- [ ] Source domain whitelist enforcement.
- [ ] Test harness simulating cross-chain deposit message execution.
"""
    },
    {
        "id": "030",
        "title": "Flash Loan Prevention & Reentrancy Shield Hardening",
        "labels": ["smart-contract", "security", "medium"],
        "body": """# #030: Flash Loan Prevention & Reentrancy Shield Hardening

## Overview
Audit and harden all vault claim and deposit handlers against flash loan manipulation and reentrancy vectors.

## Objectives
- Implement single-transaction balance lock guards preventing flash-loan-assisted fee manipulation.
- Apply non-reentrant state transition order checks-effects-interactions pattern.
- Verify state updates occur prior to external SAC token transfer invocations.

## Acceptance Criteria
- [ ] Reentrancy guard modifier applied across all external state mutation handlers.
- [ ] Security test verifying nested invocation rejection.
- [ ] Flash loan simulation test verifying pool state integrity.
"""
    },
    {
        "id": "031",
        "title": "Soroban Diagnostic Event Filter & Subgraph Indexing Specs",
        "labels": ["smart-contract", "events", "medium"],
        "body": """# #031: Soroban Diagnostic Event Filter & Subgraph Indexing Specs

## Overview
Expand event payloads to support granular diagnostic indexing by external Graph / Goldsky indexers.

## Objectives
- Include transaction correlation IDs and block sequence metadata in published event vectors.
- Publish contract state transition snapshots during major lifecycle operations.
- Ensure event fields align with GraphQL schema definitions.

## Acceptance Criteria
- [ ] Event schema dictionary document created for indexer authors.
- [ ] Correlation fields added to contract events.
- [ ] Integration test verifying event JSON serialization format.
"""
    },
    {
        "id": "032",
        "title": "Time-Weighted Average Price TWAP Oracle Module",
        "labels": ["smart-contract", "oracle", "hard"],
        "body": """# #032: Time-Weighted Average Price TWAP Oracle Module

## Overview
Implement an on-chain TWAP oracle reader for cross-asset salary conversion valuation.

## Objectives
- Read price cumulative values from Soroban DEX pools across configurable observation windows.
- Protect payment calculations against short-term price manipulation and flash spikes.
- Fall back gracefully to secondary oracle feeds if main pool liquidity drops.

## Acceptance Criteria
- [ ] TWAP calculation module with observation storage buffer.
- [ ] Outlier price filter rejecting sudden deviances.
- [ ] Unit tests for multi-period TWAP accumulation.
"""
    },
    {
        "id": "033",
        "title": "Automated Dust Collection & Vault Sweeping Handler",
        "labels": ["smart-contract", "feature", "easy"],
        "body": """# #033: Automated Dust Collection & Vault Sweeping Handler

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
"""
    },
    {
        "id": "034",
        "title": "Multi-Tiered Admin Privilege Delegation & Role Registry",
        "labels": ["smart-contract", "governance", "medium"],
        "body": """# #034: Multi-Tiered Admin Privilege Delegation & Role Registry

## Overview
Implement Role-Based Access Control RBAC smart contract registry for granular administrative roles.

## Objectives
- Define distinct roles SuperAdmin, PayrollOperator, EmergencyGuardian, Auditor.
- Restrict sensitive actions fee change vs pause vs payout trigger to designated roles.
- Support dynamic role assignment and revocation with event logs.

## Acceptance Criteria
- [ ] Role storage mapping and `has_role` assertion helper.
- [ ] Role management methods `grant_role` and `revoke_role`.
- [ ] Permission tests verifying role isolation.
"""
    },
    {
        "id": "035",
        "title": "Off-Chain EIP-712 Soroban Permit Auth Signature Structs",
        "labels": ["smart-contract", "auth", "medium"],
        "body": """# #035: Off-Chain EIP-712 Soroban Permit Auth Signature Structs

## Overview
Implement off-chain typed signature authorization Permit style for gasless employee payout claims.

## Objectives
- Allow employers to sign structured authorization payloads off-chain.
- Enable third-party relayers to submit claims on behalf of employees with employer signature.
- Verify typed domain separator, deadline timestamp, and nonce to prevent replay.

## Acceptance Criteria
- [ ] Typed authorization structure and signature verification logic.
- [ ] Deadline check rejecting expired permits.
- [ ] Test verifying gasless claim execution via relayer.
"""
    },
    {
        "id": "036",
        "title": "Automated Re-Entrancy & Double-Claim Fuzz Testing Pipeline",
        "labels": ["testing", "fuzzing", "medium"],
        "body": """# #036: Automated Re-Entrancy & Double-Claim Fuzz Testing Pipeline

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
"""
    },
    {
        "id": "037",
        "title": "Smart Contract Upgrade Rollback Guard & Timelock Enforcement",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #037: Smart Contract Upgrade Rollback Guard & Timelock Enforcement

## Overview
Enforce mandatory timelock delay before executing contract WASM upgrades to allow user inspection.

## Objectives
- Require proposed contract upgrades to sit in a pending state for a configurable timelock period e.g. 48 hours.
- Allow emergency guardians to veto malicious or buggy upgrades during the timelock window.
- Store historical WASM hashes to support emergency rollback if needed.

## Acceptance Criteria
- [ ] Upgrade proposal state machine with timelock delay check.
- [ ] `veto_upgrade` method for emergency guardian key.
- [ ] Unit tests for premature upgrade rejection and successful timelock execution.
"""
    },
    {
        "id": "038",
        "title": "Automated Vault Insurance & Emergency Loss Cushion",
        "labels": ["smart-contract", "defi", "medium"],
        "body": """# #038: Automated Vault Insurance & Emergency Loss Cushion

## Overview
Design optional insurance reserve deduction mechanism to protect against protocol level losses.

## Objectives
- Deduct micro-percentage e.g. 0.05% from deposits into dedicated on-chain insurance pool.
- Provide claim interface for affected employers in case of verified security incidents.
- Track insurance pool solvency and reserve ratios.

## Acceptance Criteria
- [ ] Insurance fee calculation and dedicated pool storage logic.
- [ ] Reserve allocation tracking.
- [ ] Unit test verifying insurance fund accumulation.
"""
    },
    {
        "id": "039",
        "title": "Contract Storage Memory Footprint & Compression Optimization",
        "labels": ["smart-contract", "performance", "medium"],
        "body": """# #039: Contract Storage Memory Footprint & Compression Optimization

## Overview
Refactor storage structures to minimize byte footprint and lower Soroban storage rent costs.

## Objectives
- Pack boolean flags and small integers into bitfield bitmasks.
- Optimize key encoding for fast lookups and lower ledger byte size.
- Measure storage rent reduction across 1,000 active vault instances.

## Acceptance Criteria
- [ ] Compact storage representation implemented.
- [ ] Benchmark report showing storage byte size reduction.
- [ ] All functional tests passing without regression.
"""
    },
    {
        "id": "040",
        "title": "Dynamic Gas Price Advisory & Fee Buffer Calculator",
        "labels": ["smart-contract", "soroban", "easy"],
        "body": """# #040: Dynamic Gas Price Advisory & Fee Buffer Calculator

## Overview
Expose read-only contract query functions returning estimated gas instructions for complex multi-payment calls.

## Objectives
- Calculate resource requirements based on recipient list length dynamically.
- Return recommended resource fee buffers to prevent transaction fee rejection during network congestion.
- Support off-chain client pre-flight transaction assembly.

## Acceptance Criteria
- [ ] Query function `estimate_payroll_resources(recipient_count)` implemented.
- [ ] Accurate resource estimation tested across 1 to 50 recipients.
- [ ] Unit test verifying query accuracy.
"""
    },
    {
        "id": "041",
        "title": "Multi-Asset Escrow Swap Router for Unmatched Assets",
        "labels": ["smart-contract", "dex", "hard"],
        "body": """# #041: Multi-Asset Escrow Swap Router for Unmatched Assets

## Overview
Implement smart router that automatically converts incoming non-standard tokens into approved vault stablecoins.

## Objectives
- Integrate DEX pool routing logic to convert arbitrary SAC tokens to USDC/ORGUSD.
- Validate minimum conversion output against oracle price before accepting deposit.
- Refund sender if swap route cannot be resolved or fails slippage checks.

## Acceptance Criteria
- [ ] Multi-hop swap execution wrapper.
- [ ] Minimum return guard against price manipulation.
- [ ] Unit tests for direct and multi-hop asset conversions.
"""
    },
    {
        "id": "042",
        "title": "Vesting Cliff Schedule Calculator & Partial Release Handler",
        "labels": ["smart-contract", "feature", "medium"],
        "body": """# #042: Vesting Cliff Schedule Calculator & Partial Release Handler

## Overview
Implement cliff vesting calculation logic supporting linear unlock schedules for employee equity/bonus tokens.

## Objectives
- Support configurable cliff duration e.g. 6 or 12 months followed by monthly linear vesting.
- Calculate claimable unlocked amount precisely based on current ledger timestamp.
- Prevent premature claims before cliff expiry.

## Acceptance Criteria
- [ ] Vesting schedule struct cliff_timestamp, end_timestamp, total_amount.
- [ ] `calculate_unlocked_amount` helper function verified.
- [ ] Unit tests for pre-cliff, mid-vesting, and post-vesting claims.
"""
    },
    {
        "id": "043",
        "title": "On-Chain Multisig Proposal Voting & Execution Ledger",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #043: On-Chain Multisig Proposal Voting & Execution Ledger

## Overview
Build an on-chain proposal creation, approval voting, and execution state machine for admin transactions.

## Objectives
- Allow admins to propose parameter changes or treasury fund movements.
- Collect cryptographic signatures / approval votes from authorized multisig members on-chain.
- Execute proposal automatically once threshold approval is reached.

## Acceptance Criteria
- [ ] Proposal data structures and status workflow Pending, Approved, Executed, Cancelled.
- [ ] `vote_proposal` and `execute_proposal` endpoints.
- [ ] Unit tests covering proposal lifecycle.
"""
    },
    {
        "id": "044",
        "title": "Soroban WASM Size Optimization & Dead Code Elimination",
        "labels": ["tooling", "performance", "easy"],
        "body": """# #044: Soroban WASM Size Optimization & Dead Code Elimination

## Overview
Optimize Rust compilation flags and Cargo features to achieve minimal compiled `.wasm` file size.

## Objectives
- Configure release profile flags opt-level = 'z', lto = true, codegen-units = 1, panic = 'abort'.
- Remove unused dependency features and redundant error string literals.
- Reduce target WASM size below 50 KB for faster deployment and lower ledger footprint.

## Acceptance Criteria
- [ ] Cargo profile settings optimized in root `Cargo.toml`.
- [ ] WASM file size reduction measured and documented.
- [ ] All unit and integration tests passing.
"""
    },
    {
        "id": "045",
        "title": "Integration Test Harness against Soroban Sandbox RPC",
        "labels": ["testing", "integration", "medium"],
        "body": """# #045: Integration Test Harness against Soroban Sandbox RPC

## Overview
Build automated integration testing pipeline that spins up a local Soroban RPC node and executes contract flows.

## Objectives
- Automate contract deployment and SAC token setup on local standalone node.
- Execute end-to-end payment scenarios matching production API calls.
- Assert RPC response codes and ledger state changes.

## Acceptance Criteria
- [ ] Test harness script created under `tests/integration_test.rs` or Python script.
- [ ] Clean execution against local Soroban sandbox container.
- [ ] CI workflow step for end-to-end integration test execution.
"""
    },
    {
        "id": "046",
        "title": "Cross-Contract Call Error Boundary & Trap Handler",
        "labels": ["smart-contract", "architecture", "medium"],
        "body": """# #046: Cross-Contract Call Error Boundary & Trap Handler

## Overview
Implement error boundary guards around invocations to external SAC tokens or oracle contracts to catch traps safely.

## Objectives
- Prevent sub-contract panics from bricking main contract execution.
- Catch host call errors and translate them into structured contract return codes.
- Emit diagnostic events detailing caught external failures.

## Acceptance Criteria
- [ ] Safe external contract invocation wrapper using `try_invoke`.
- [ ] Fallback logic for unhandled external contract traps.
- [ ] Unit tests simulating failing external token transfers.
"""
    },
    {
        "id": "047",
        "title": "Sanctioned Account Check & Dynamic Blacklist Registry",
        "labels": ["smart-contract", "compliance", "medium"],
        "body": """# #047: Sanctioned Account Check & Dynamic Blacklist Registry

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
"""
    },
    {
        "id": "048",
        "title": "On-Chain Royalty & Developer Fee Auto-Distribution",
        "labels": ["smart-contract", "feature", "easy"],
        "body": """# #048: On-Chain Royalty & Developer Fee Auto-Distribution

## Overview
Implement protocol fee distribution mechanism allocating percentages to developer ecosystem reserve fund.

## Objectives
- Split protocol fees automatically into developer fund and operational treasury.
- Allow community governance to adjust developer allocation percentage within safety bounds 0% - 5%.
- Ensure transparent fee auditability.

## Acceptance Criteria
- [ ] Fee splitter logic supporting multiple recipient accounts.
- [ ] Governance adjustment handler with maximum cap assertion.
- [ ] Unit tests verifying fee calculation math.
"""
    },
    {
        "id": "049",
        "title": "Off-Chain Snapshot Verifier & Merkle Proof Vault Claim",
        "labels": ["smart-contract", "cryptography", "hard"],
        "body": """# #049: Off-Chain Snapshot Verifier & Merkle Proof Vault Claim

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
"""
    },
    {
        "id": "050",
        "title": "Storage TTL Monitoring & Auto-Renewal Bot Config",
        "labels": ["devops", "tooling", "medium"],
        "body": """# #050: Storage TTL Monitoring & Auto-Renewal Bot Config

## Overview
Build automated background monitoring bot configuration to track contract storage TTL and send bump transactions before expiration.

## Objectives
- Query Soroban RPC storage entry TTL for contract keys periodically.
- Trigger `extend_ttl` transactions automatically when TTL falls below safety threshold.
- Alert maintainers if bot wallet balance is low.

## Acceptance Criteria
- [ ] Monitoring script created under `tooling/ttl_bot.py`.
- [ ] Configurable TTL threshold e.g. 10,000 ledgers.
- [ ] Log output and execution dry-run test.
"""
    },
    {
        "id": "051",
        "title": "Soroban SDK v22 Pre-Release Migration & Feature Flags",
        "labels": ["maintenance", "soroban", "medium"],
        "body": """# #051: Soroban SDK v22 Pre-Release Migration & Feature Flags

## Overview
Prepare codebase for upcoming Soroban SDK v22 features and test compatibility flags.

## Objectives
- Audit breaking changes in upcoming Soroban SDK releases.
- Implement conditional compilation feature flags.
- Ensure project builds seamlessly across version boundaries.

## Acceptance Criteria
- [ ] Feature flag configured in `Cargo.toml`.
- [ ] Code base free of deprecated syntax.
- [ ] Successful compilation under target feature flags.
"""
    },
    {
        "id": "052",
        "title": "Property-Based Testing Harness with proptest-rs",
        "labels": ["testing", "fuzzing", "medium"],
        "body": """# #052: Property-Based Testing Harness with proptest-rs

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
"""
    },
    {
        "id": "053",
        "title": "Automated Contract Release Packaging & WASM Verification Artifacts",
        "labels": ["devops", "ci-cd", "easy"],
        "body": """# #053: Automated Contract Release Packaging & WASM Verification Artifacts

## Overview
Set up automated release pipeline that builds reproducible WASM binaries and attaches SHA-256 checksums to GitHub releases.

## Objectives
- Build WASM targets inside deterministic Docker container to guarantee binary reproducibility.
- Generate cryptographic hash manifest `checksums.txt` for WASM builds.
- Automate GitHub Release asset attachment on tag creation.

## Acceptance Criteria
- [ ] Release workflow configured in `.github/workflows/release.yml`.
- [ ] Reproducible WASM verification script.
- [ ] Release workflow tested.
"""
    },
    {
        "id": "054",
        "title": "Comprehensive Multi-Sig Governance Security Checklist",
        "labels": ["documentation", "security", "easy"],
        "body": """# #054: Comprehensive Multi-Sig Governance Security Checklist

## Overview
Create detailed security operational guide for managing contract multisig signers, key rotation, and emergency procedures.

## Objectives
- Detail best practices for signer threshold selection and key storage hardware wallets / KMS.
- Define step-by-step SOP for key rotation and emergency contract pause.
- Add security incident response escalation tree.

## Acceptance Criteria
- [ ] Operational guide created at `docs/MULTISIG_OPERATIONS.md`.
- [ ] Step-by-step verification procedures for multi-sig key rotation.
- [ ] Peer review completed.
"""
    },
    {
        "id": "055",
        "title": "Soroban Local RPC Dev Environment Docker Compose Setup",
        "labels": ["devops", "tooling", "easy"],
        "body": """# #055: Soroban Local RPC Dev Environment Docker Compose Setup

## Overview
Create a single-command local development environment running Stellar Standalone Node with Soroban RPC.

## Objectives
- Provide `docker-compose.yml` that starts Stellar standalone node, friendbot, and Soroban RPC service.
- Pre-fund developer test accounts on container startup.
- Document simple setup commands for new smart contract contributors.

## Acceptance Criteria
- [ ] `docker-compose.yml` created in root directory.
- [ ] Healthcheck script verifying Soroban RPC readiness.
- [ ] Developer onboarding section added to main `README.md`.
"""
    }
]

def create_local_files():
    print("Writing 30 new issue markdown files to docs/issues/...")
    for issue in NEW_ISSUES:
        filename = f"{issue['id']}-{issue['title'].lower().replace(' ', '-').replace('/', '-').replace('(', '').replace(')', '')}.md"
        filepath = os.path.join(ISSUES_DIR, filename)
        with open(filepath, "w") as f:
            f.write(issue['body'])
        print(f"Created {filename}")

def push_to_github():
    print("\nPushing 30 new issues to GitHub repository via REST API...")
    url = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/issues"
    
    for issue in NEW_ISSUES:
        payload = {
            "title": f"#{issue['id']}: {issue['title']}",
            "body": issue['body'],
            "labels": issue['labels']
        }
        res = requests.post(url, headers=headers, json=payload)
        if res.status_code == 201:
            issue_data = res.json()
            print(f"✅ Created GitHub Issue #{issue_data['number']}: {issue['title']}")
        else:
            print(f"❌ Failed to create issue {issue['title']}: {res.status_code} - {res.text}")
        time.sleep(1)

if __name__ == "__main__":
    create_local_files()
    push_to_github()
