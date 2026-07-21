import os
import json
import time
import urllib.parse
import requests

# 1. Read token from ~/.git-credentials
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

ISSUES = [
    {
        "id": "001",
        "title": "Soroban Escrow & Payroll Lockup Contract Implementation",
        "labels": ["smart-contract", "soroban", "core", "hard"],
        "body": """# #001: Soroban Escrow & Payroll Lockup Contract Implementation

## Overview
Implement core Soroban smart contract logic for time-locked escrow, automated payroll distribution, and locked payout execution on Stellar.

## Objectives
- Create Soroban smart contract module for employer vault deposits.
- Support time-based release schedule for employee salaries and milestone payouts.
- Enforce strict authorization checks ensuring only authorized admins or scheduled triggers release funds.

## Acceptance Criteria
- [ ] Escrow storage data structures defined and unit tested.
- [ ] `deposit`, `release`, and `refund` methods implemented with host invocation auth.
- [ ] 100% test coverage for lockup period expiration and early release scenarios.
"""
    },
    {
        "id": "002",
        "title": "Multi-Sig Issuer & Admin Security Controls",
        "labels": ["smart-contract", "security", "hard"],
        "body": """# #002: Multi-Sig Issuer & Admin Security Controls

## Overview
Add threshold-based multi-signature authorization for administrative functions, contract parameter changes, and high-value payroll releases.

## Objectives
- Support multiple admin signers with configurable weight thresholds.
- Prevent single point of failure or compromise for employer treasury accounts.
- Enforce multi-sig checks on critical methods like fee adjustments and vault withdrawals.

## Acceptance Criteria
- [ ] Admin threshold data structures defined in contract storage.
- [ ] Verification logic for multi-sig approvals implemented.
- [ ] Unit tests for sub-threshold rejection and valid multi-sig execution.
"""
    },
    {
        "id": "003",
        "title": "Emergency Freeze & Circuit Breaker Mechanism",
        "labels": ["smart-contract", "security", "medium"],
        "body": """# #003: Emergency Freeze & Circuit Breaker Mechanism

## Overview
Implement pause and resume state controls (`pausable.rs`) for emergency freezes during abnormal activities or security incidents.

## Objectives
- Allow designated emergency admin or guardian key to pause contract functions instantly.
- Prevent deposits and withdrawals when contract is in paused state.
- Provide secure unpause mechanism once security audit / resolution is completed.

## Acceptance Criteria
- [ ] Contract status flag `IsPaused` added to persistent storage.
- [ ] Modifiers or guard assertions added to sensitive functions (`deposit`, `claim`, `payout`).
- [ ] Test cases verifying paused state error handling.
"""
    },
    {
        "id": "004",
        "title": "Asset Clawback & Compliance Logic Support",
        "labels": ["smart-contract", "compliance", "medium"],
        "body": """# #004: Asset Clawback & Compliance Logic Support

## Overview
Integrate Stellar Soroban token clawback interface for regulatory compliance and dispute resolution in payroll lockups.

## Objectives
- Support `clawback` calls for SAC (Stellar Asset Contract) tokens held in escrow.
- Ensure clawback events emit standard compliance telemetry for audit trails.
- Enforce caller authority checks so only legal compliance controllers can initiate clawbacks.

## Acceptance Criteria
- [ ] Clawback wrapper method defined and integrated with token client.
- [ ] Compliance verification logic for clawback requests.
- [ ] Unit test simulating regulatory clawback execution.
"""
    },
    {
        "id": "005",
        "title": "Revenue Split & Fee Sharing Smart Contract Module",
        "labels": ["smart-contract", "feature", "medium"],
        "body": """# #005: Revenue Split & Fee Sharing Smart Contract Module

## Overview
Implement automated percentage split on transaction processing for platform operational fees and liquidity rewards.

## Objectives
- Define configurable fee percentage shares for platform treasury and protocol reserve.
- Calculate and execute fee deductions atomically during payroll execution.
- Prevent precision loss or rounding errors in token distribution.

## Acceptance Criteria
- [ ] On-chain configuration interface for split percentages.
- [ ] Atomic multi-recipient payout execution in a single invocation.
- [ ] Unit tests checking integer math accuracy across edge cases.
"""
    },
    {
        "id": "006",
        "title": "Support Multi-Stablecoin Vaults (USDC, ORGUSD, EURT)",
        "labels": ["smart-contract", "feature", "hard"],
        "body": """# #006: Support Multi-Stablecoin Vaults (USDC, ORGUSD, EURT)

## Overview
Expand smart contract storage and payment routes to accept multiple SAC (Stellar Asset Contract) stablecoins.

## Objectives
- Decouple contract logic from a single token address to support dynamic token mapping.
- Support employer vaults holding USDC, PYUSD, EURT, or custom ORGUSD assets.
- Maintain separate balance ledgers per token contract address.

## Acceptance Criteria
- [ ] Token-indexed vault balance mapping in contract storage.
- [ ] Multi-token deposit and claim handlers.
- [ ] Unit tests for multi-stablecoin operations in the same escrow instance.
"""
    },
    {
        "id": "007",
        "title": "Contract State Archival & TTL Extension Strategy",
        "labels": ["smart-contract", "soroban", "medium"],
        "body": """# #007: Contract State Archival & TTL Extension Strategy

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
"""
    },
    {
        "id": "008",
        "title": "Gas Optimization & Batch Payment Benchmarking",
        "labels": ["smart-contract", "performance", "medium"],
        "body": """# #008: Gas Optimization & Batch Payment Benchmarking

## Overview
Profile CPU instructions and memory usage for batch payouts; optimize loop structures and storage writes to minimize gas fees.

## Objectives
- Benchmark Soroban resource consumption for batch payouts of 10, 50, and 100 employees.
- Minimize redundant storage reads/writes by caching contract state in local variables.
- Optimize data structures (e.g. using compact vectors instead of heavy maps where appropriate).

## Acceptance Criteria
- [ ] Gas benchmark report generated for batch sizes up to 100 payments.
- [ ] CPU instruction reduction achieved across contract loops.
- [ ] Tests verifying gas limit compliance on Testnet ledger bounds.
"""
    },
    {
        "id": "009",
        "title": "Account-Level Transaction Volume Throttling & Rate Limiting",
        "labels": ["smart-contract", "security", "medium"],
        "body": """# #009: Account-Level Transaction Volume Throttling & Rate Limiting

## Overview
Implement sliding-window rate limit checks per user or employer address inside smart contract invocations.

## Objectives
- Prevent spam and malicious high-frequency calls on contract endpoints.
- Track call frequencies and cumulative withdrawal volumes per ledger window.
- Revert transactions exceeding defined risk thresholds.

## Acceptance Criteria
- [ ] Rate limit tracking state added to contract storage.
- [ ] Window evaluation logic tested against ledger timestamp / sequence.
- [ ] Unit test verifying rate limit error enforcement.
"""
    },
    {
        "id": "010",
        "title": "Asset Path Payment & Cross-Asset Exchange Execution",
        "labels": ["smart-contract", "soroban", "hard"],
        "body": """# #010: Asset Path Payment & Cross-Asset Exchange Execution

## Overview
Integrate Soroban DEX / Stellar liquidity pools interface for direct cross-asset payroll conversions during payout.

## Objectives
- Allow employers to deposit USDC while employees receive their local stablecoin via atomic swap.
- Support slippage protection and minimum recipient amount enforcement.
- Ensure failed swaps cleanly abort without loss of funds.

## Acceptance Criteria
- [ ] Liquidity pool / DEX swap interface bindings created.
- [ ] Path payment execution wrapper with slippage guards.
- [ ] Test cases covering successful swap payouts and slippage reverts.
"""
    },
    {
        "id": "011",
        "title": "Formal Verification of Multi-Sig & Escrow Logic (Kani / SMT)",
        "labels": ["smart-contract", "testing", "hard"],
        "body": """# #011: Formal Verification of Multi-Sig & Escrow Logic (Kani / SMT)

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
"""
    },
    {
        "id": "012",
        "title": "Graceful Revert & Failed Payout Auto-Refund Handler",
        "labels": ["smart-contract", "soroban", "medium"],
        "body": """# #012: Graceful Revert & Failed Payout Auto-Refund Handler

## Overview
Ensure failed atomic transfers release locked funds back to employer vaults without leaving state in an inconsistent lockup state.

## Objectives
- Wrap payout operations in safe transaction handlers that revert state cleanly on failure.
- Emit explicit failure logs with error reason symbols for off-chain indexing.
- Prevent locked balances from being permanently stuck in transient states.

## Acceptance Criteria
- [ ] Safe error handling and revert guards added to batch payout loops.
- [ ] Error event emission for invalid recipient addresses or frozen trustlines.
- [ ] Unit tests verifying atomic state rollback on single payment failure.
"""
    },
    {
        "id": "013",
        "title": "SECP256K1 & Custom Signature Verification Support",
        "labels": ["smart-contract", "cryptography", "hard"],
        "body": """# #013: SECP256K1 & Custom Signature Verification Support

## Overview
Add native SECP256K1 signature validation for cross-chain identity verification and authorized auth payload validation.

## Objectives
- Utilize Soroban host functions for SECP256K1 signature checking.
- Allow off-chain signature authorization from ECDSA keys (e.g. Ethereum/Bitcoin wallets).
- Enforce replay attack protection with strictly incrementing nonces.

## Acceptance Criteria
- [ ] SECP256K1 signature verification helper module implemented.
- [ ] Nonce-based replay protection integrated.
- [ ] Unit tests validating valid signature execution and invalid signature rejection.
"""
    },
    {
        "id": "014",
        "title": "Implement Contract Metadata Standard (SEP-0034)",
        "labels": ["smart-contract", "standards", "easy"],
        "body": """# #014: Implement Contract Metadata Standard (SEP-0034)

## Overview
Add SEP-0034 contract metadata specs, interface descriptions, version tag embedding, and deployment manifests.

## Objectives
- Embed standard metadata attributes (name, description, author, version, repo URL) in contract WASM.
- Provide queried getter function returning standardized JSON metadata.
- Align with Stellar ecosystem tool discoverability standards.

## Acceptance Criteria
- [ ] Metadata attributes embedded into contract WASM build.
- [ ] Query interface `get_contract_metadata` returns valid SEP-0034 format.
- [ ] Unit tests verifying metadata output consistency.
"""
    },
    {
        "id": "015",
        "title": "On-Chain Performance Bonus & Vesting Audit Trail",
        "labels": ["smart-contract", "audit", "medium"],
        "body": """# #015: On-Chain Performance Bonus & Vesting Audit Trail

## Overview
Add event emitting (`env.events().publish(...)`) for bonus assignments, vesting milestones, and claim tracking.

## Objectives
- Define structured event schemas for all contract lifecycle actions.
- Ensure event topics enable efficient filtering by employer, employee, and timestamp.
- Guarantee auditability of all salary, bonus, and vesting state changes.

## Acceptance Criteria
- [ ] Event schema symbols defined for `Deposit`, `BonusAssigned`, `Claimed`, and `Refunded`.
- [ ] Events published on all state mutation paths.
- [ ] Integration tests verifying event topic structure and data payloads.
"""
    },
    {
        "id": "016",
        "title": "Soroban v21+ Host Environment Compatibility Audit",
        "labels": ["smart-contract", "maintenance", "medium"],
        "body": """# #016: Soroban v21+ Host Environment Compatibility Audit

## Overview
Update smart contract dependencies to `soroban-sdk 21.x`, resolve deprecation warnings, and verify host function bindings.

## Objectives
- Upgrade Cargo workspace dependencies to latest Soroban SDK version.
- Update test environment and host environment test runners.
- Ensure zero build warnings or deprecated API usage.

## Acceptance Criteria
- [ ] `Cargo.toml` updated with latest compatible `soroban-sdk`.
- [ ] All contract modules compilation clean with `cargo build --target wasm32-unknown-unknown`.
- [ ] Unit tests passing green under updated Soroban host simulator.
"""
    },
    {
        "id": "017",
        "title": "Storage Key Collision Detection & Static Analysis Tooling",
        "labels": ["tooling", "security", "medium"],
        "body": """# #017: Storage Key Collision Detection & Static Analysis Tooling

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
"""
    },
    {
        "id": "018",
        "title": "Automated Unit Test Suite for Edge-Case Payroll Scenarios",
        "labels": ["testing", "unit-tests", "easy"],
        "body": """# #018: Automated Unit Test Suite for Edge-Case Payroll Scenarios

## Overview
Expand `src/test.rs` to cover zero-balance claims, unauthorized pause attempts, double claims, and boundary conditions.

## Objectives
- Increase test coverage across edge-case logic and negative security tests.
- Verify proper panic error messages for invalid input arguments.
- Test concurrency and sequence of multiple employee claims.

## Acceptance Criteria
- [ ] Comprehensive test suite covering at least 20 new edge-case scenarios.
- [ ] All tests passing with `cargo test`.
- [ ] Code coverage above 90% across core contract modules.
"""
    },
    {
        "id": "019",
        "title": "Build CLI Tool for Contract Deployment & Network Initialization",
        "labels": ["cli", "tooling", "medium"],
        "body": """# #019: Build CLI Tool for Contract Deployment & Network Initialization

## Overview
Enhance `latterfix-smart-contract-cli` to automate contract WASM deployment, initialization, and token binding on Testnet/Mainnet.

## Objectives
- Build interactive and automated CLI commands (`deploy`, `init`, `upgrade`).
- Automate key management and transaction signing for network deployments.
- Output deployment artifacts (Contract ID, WASM hash, network configs) as JSON.

## Acceptance Criteria
- [ ] CLI binary builds cleanly and supports `--help` commands.
- [ ] Successful deployment test script against Stellar Testnet.
- [ ] Documentation for CLI usage added to README.
"""
    },
    {
        "id": "020",
        "title": "Continuous Integration (CI) Workflow for WASM Build & Test Verification",
        "labels": ["devops", "ci-cd", "easy"],
        "body": """# #020: Continuous Integration (CI) Workflow for WASM Build & Test Verification

## Overview
Set up GitHub Actions workflow `.github/workflows/contract-ci.yml` for automated cargo test, clippy, and WASM compilation.

## Objectives
- Run linting (`cargo clippy --all-targets`) on every Pull Request.
- Execute unit test suite and verify WASM target compilation (`wasm32-unknown-unknown`).
- Block pull requests if tests or lints fail.

## Acceptance Criteria
- [ ] GitHub Actions workflow file configured and committed.
- [ ] Build matrix testing Rust stable and nightly toolchains.
- [ ] Badge added to README displaying CI build status.
"""
    },
    {
        "id": "021",
        "title": "Implement Contract Upgradeability & Storage Migration Patterns",
        "labels": ["smart-contract", "architecture", "hard"],
        "body": """# #021: Implement Contract Upgradeability & Storage Migration Patterns

## Overview
Design WASM code hash replacement flow (`env.deployer().update_current_contract_wasm(...)`) with state migration checks.

## Objectives
- Provide secure contract code upgrade endpoint guarded by multi-sig authorization.
- Ensure contract state data structures remain compatible across WASM upgrades.
- Add versioning checks to prevent incompatible code migrations.

## Acceptance Criteria
- [ ] `upgrade` endpoint implemented using Soroban deployer host function.
- [ ] Upgrade authorization and code hash validation tests.
- [ ] Migration test demonstrating state preservation across upgrade invocation.
"""
    },
    {
        "id": "022",
        "title": "Comprehensive API & Interface Documentation (Rustdoc)",
        "labels": ["documentation", "easy"],
        "body": """# #022: Comprehensive API & Interface Documentation (Rustdoc)

## Overview
Add complete Rustdoc comments, architecture diagrams, and sequence flows across all contract modules.

## Objectives
- Document every public struct, enum, function, and error code.
- Generate HTML doc pages with `cargo doc --no-deps`.
- Provide inline code examples for common contract function invocations.

## Acceptance Criteria
- [ ] 100% Rustdoc documentation coverage for public contract APIs.
- [ ] `cargo doc` builds without warnings.
- [ ] Architecture diagrams embedded in documentation.
"""
    },
    {
        "id": "023",
        "title": "Event Schema Indexing Specification & Topic Standardization",
        "labels": ["smart-contract", "events", "medium"],
        "body": """# #023: Event Schema Indexing Specification & Topic Standardization

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
"""
    },
    {
        "id": "024",
        "title": "Mock Environment Test Harness for Front-End Developers",
        "labels": ["testing", "tooling", "easy"],
        "body": """# #024: Mock Environment Test Harness for Front-End Developers

## Overview
Export mock contract client helpers and TypeScript contract binding generators for rapid local testing.

## Objectives
- Generate TypeScript type definitions and invocation helpers from contract WASM.
- Provide local Soroban RPC sandbox startup script for frontend integration.
- Document mock user keypairs and pre-funded test tokens.

## Acceptance Criteria
- [ ] Automated script to generate TS client bindings via `@stellar/stellar-sdk`.
- [ ] Mock environment setup script in `tooling/`.
- [ ] Frontend developer onboarding guide for local contract testing.
"""
    },
    {
        "id": "025",
        "title": "Security Audit Checklist & Static Code Scanner Integration",
        "labels": ["security", "audit", "medium"],
        "body": """# #025: Security Audit Checklist & Static Code Scanner Integration

## Overview
Integrate `cargo-audit`, `cargo-deny`, and static security analysis rules into the smart contract repository.

## Objectives
- Check all third-party dependencies for known vulnerabilities.
- Enforce strict license compliance and dependency security advisories.
- Publish a self-audit security checklist for contract reviewers.

## Acceptance Criteria
- [ ] `deny.toml` configuration committed to repository.
- [ ] Security audit checklist document created in `docs/SECURITY.md`.
- [ ] CI pipeline step executing `cargo audit` and `cargo deny check`.
"""
    }
]

def create_local_files():
    print("Writing local issue markdown files to docs/issues/...")
    for issue in ISSUES:
        filename = f"{issue['id']}-{issue['title'].lower().replace(' ', '-').replace('/', '-').replace('(', '').replace(')', '')}.md"
        filepath = os.path.join(ISSUES_DIR, filename)
        with open(filepath, "w") as f:
            f.write(issue['body'])
        print(f"Created {filename}")

def push_to_github():
    print("\nPushing issues to GitHub repository via REST API...")
    url = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/issues"
    
    for issue in ISSUES:
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
