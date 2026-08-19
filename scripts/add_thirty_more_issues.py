import os
import json
import time
import urllib.parse
import requests

creds_file = os.path.expanduser('~/.git-credentials')
if not os.path.exists(creds_file):
    raise RuntimeError("~/.git-credentials file not found!")

with open(creds_file) as f:
    lines = f.readlines()

token = None
for line in lines:
    line = line.strip()
    if not line:
        continue
    parsed = urllib.parse.urlparse(line)
    if parsed.username == "bakarezainab" or "bakarezainab" in line:
        token = parsed.password
        break

if not token:
    parsed = urllib.parse.urlparse(lines[0].strip())
    token = parsed.password

headers = {
    'Authorization': f'token {token}',
    'Accept': 'application/vnd.github.v3+json'
}

REPO_OWNER = "LatterFixxx"
REPO_NAME = "LatterFix-Smart-contract"
ISSUES_DIR = os.path.join(os.getcwd(), "docs", "issues")

os.makedirs(ISSUES_DIR, exist_ok=True)

THIRTY_ISSUES = [
    {
        "id": "056",
        "title": "Reputation-Weighted Slashing Mechanism on Governance Proposal Veto",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #056: Reputation-Weighted Slashing Mechanism on Governance Proposal Veto

## Overview
Implement a reputation-weighted slashing mechanism where users who vote in favor of malicious or spam proposals face automatic reputation point deduction if the proposal is vetoed by the admin/emergency council.

## Objectives
- Track the voting history of user profiles for each active governance proposal.
- Define a reputation slashing formula based on the user's vote weight and proposal severity.
- Apply reputation deductions automatically upon execution of an admin veto.
- Protect low-reputation users by preventing slashing below zero reputation points.

## Acceptance Criteria
- [ ] Slashing execution function integrated into `governance.rs` and `reputation.rs`.
- [ ] Safety limits to prevent slashing below zero reputation points.
- [ ] Unit tests covering proposal creation, voting, veto, and subsequent reputation reduction.
"""
    },
    {
        "id": "057",
        "title": "ZK-Snark Based Proof of Delivery Verification",
        "labels": ["smart-contract", "cryptography", "hard"],
        "body": """# #057: ZK-Snark Based Proof of Delivery Verification

## Overview
Implement a zero-knowledge proof verification pipeline to validate task completion/deliverables without exposing the sensitive delivery URLs on-chain.

## Objectives
- Integrate a Groth16 ZK proof verification helper in the smart contract.
- Support cryptographic verification of proof payloads matching the delivery hash.
- Ensure only verified deliveries trigger payment release.

## Acceptance Criteria
- [ ] ZK proof verification helper implemented.
- [ ] Submission endpoint updated to accept proof payloads.
- [ ] Unit tests verifying proof authentication and rejection of forged proofs.
"""
    },
    {
        "id": "058",
        "title": "Multi-Token Quadratic Voting System",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #058: Multi-Token Quadratic Voting System

## Overview
Implement a quadratic voting system for governance proposals where the voting power is calculated quadratically based on locked reputation and multiple stablecoin balances.

## Objectives
- Build mathematical quadratic voting calculation modules.
- Ensure votes count as the square root of locked voting weight.
- Allow users to split votes across multiple options.

## Acceptance Criteria
- [ ] Quadratic calculation helper verified.
- [ ] Proposal voting methods updated to support quadratic weights.
- [ ] Unit tests checking accurate power calculation and rounding error handling.
"""
    },
    {
        "id": "059",
        "title": "Decentralized Dispute Resolution Arbitration Council Selection",
        "labels": ["smart-contract", "reputation", "hard"],
        "body": """# #059: Decentralized Dispute Resolution Arbitration Council Selection

## Overview
Create a dynamic arbitrator selection protocol that selects a randomized panel of moderators from Expert/Legend tier users who stake reputation points.

## Objectives
- Maintain a pool of reputation-staked users eligible to act as arbitrators.
- Randomly select 3 or 5 arbitrators for disputed tasks.
- Reward correct decisions and penalize minority dissenters.

## Acceptance Criteria
- [ ] Arbitrator selection and registration methods.
- [ ] Consensus verification and reward/penalty execution.
- [ ] Test cases covering the full dispute resolution flow with a panel.
"""
    },
    {
        "id": "060",
        "title": "Automated Escrow Staking Yield Distribution",
        "labels": ["smart-contract", "defi", "hard"],
        "body": """# #060: Automated Escrow Staking Yield Distribution

## Overview
Route locked escrow funds to Soroban liquidity pools/AMMs automatically to earn yield, distributing interest between the treasury and the assignee upon task completion.

## Objectives
- Interconnect task escrow vaults with external Stellar liquidity pools.
- Track accrued interest rates per task escrow.
- Automate payout of principal to assignee and split yield upon completion.

## Acceptance Criteria
- [ ] External pool deposit/withdraw wrapper.
- [ ] Yield splitting and distribution calculations.
- [ ] Unit tests simulating yield generation and distribution.
"""
    },
    {
        "id": "061",
        "title": "Dual-State Ledger Storage Optimization",
        "labels": ["smart-contract", "performance", "hard"],
        "body": """# #061: Dual-State Ledger Storage Optimization

## Overview
Refactor the storage architecture to implement a dual-state system that uses temporary storage for high-churn task parameters and persistent storage for user profiles to minimize state rent fees.

## Objectives
- Identify high-churn data (e.g., active bids, submissions) vs persistent data (profiles, settings).
- Refactor `storage.rs` to separate temporary vs persistent storage keys.
- Write automatic TTL upgrade scripts for persistent keys.

## Acceptance Criteria
- [ ] Separate storage layers implemented.
- [ ] Benchmark comparing old storage gas costs vs new layout.
- [ ] All unit tests passing without storage state loss.
"""
    },
    {
        "id": "062",
        "title": "Formal Verification of State Transitions under Concurrency (Kani)",
        "labels": ["testing", "fuzzing", "hard"],
        "body": """# #062: Formal Verification of State Transitions under Concurrency (Kani)

## Overview
Add formal verification proofs verifying that concurrent invocations of claim, submission, and cancellation handlers cannot cause double-spending or unauthorized escrow releases.

## Objectives
- Write formal proof models using the Kani model checker.
- Verify absolute balance invariants under concurrent task actions.
- Identify and eliminate potential race conditions or state corruption.

## Acceptance Criteria
- [ ] Kani verification harness implemented.
- [ ] Proofs validating correct state transition constraints.
- [ ] 0 issues or violations found during Kani validation runs.
"""
    },
    {
        "id": "063",
        "title": "Non-interactive Zero-Knowledge Proofs for Verifiable Credentials",
        "labels": ["smart-contract", "cryptography", "hard"],
        "body": """# #063: Non-interactive Zero-Knowledge Proofs for Verifiable Credentials

## Overview
Let users verify corporate membership or eligibility credentials when creating profiles using non-interactive zero-knowledge proofs.

## Objectives
- Integrate NIZK proof parsing and verification inside profile registration.
- Verify credential issuers without exposing credential contents.
- Keep profiles anonymous while proving compliance.

## Acceptance Criteria
- [ ] Verifiable credential proof validator implemented.
- [ ] Integration with `user_profile.rs`.
- [ ] Unit tests with valid and invalid proof payloads.
"""
    },
    {
        "id": "064",
        "title": "Cross-Contract Call Reentrancy Lock using Host Invocation Key",
        "labels": ["smart-contract", "security", "hard"],
        "body": """# #064: Cross-Contract Call Reentrancy Lock using Host Invocation Key

## Overview
Secure token transfer entry points by implementing an advanced reentrancy protection check using host invocation key inspections.

## Objectives
- Intercept reentrancy attempts during cross-contract token transfers.
- Implement strict call stack depth limits.
- Prevent attacker contracts from recalling task payout functions recursively.

## Acceptance Criteria
- [ ] Advanced host invocation key validation guard.
- [ ] Security test suite simulating malicious callback contracts.
- [ ] Clean compilation and execution under all testing profiles.
"""
    },
    {
        "id": "065",
        "title": "Time-Locked Multi-Sig Master Admin Safe",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #065: Time-Locked Multi-Sig Master Admin Safe

## Overview
Create a time-locked multi-signature vault configuration acting as the master owner of the contract, enforcing a 7-day timelock delay on critical updates.

## Objectives
- Restrict direct admin actions behind a time-delayed queue.
- Support threshold signature validations for pending actions.
- Allow emergency cancel of pending actions by a guardian role.

## Acceptance Criteria
- [ ] Queue and timelock checks on admin endpoints.
- [ ] Multisig authorization checks.
- [ ] Unit tests covering the queue-approve-execute lifecycle.
"""
    },
    {
        "id": "066",
        "title": "Merkle Tree Batch Task Creation and Escrow Funding",
        "labels": ["smart-contract", "performance", "hard"],
        "body": """# #066: Merkle Tree Batch Task Creation and Escrow Funding

## Overview
Optimize gas costs for creating bulk tasks by storing a single Merkle root representing task descriptions and funding them collectively in one transaction.

## Objectives
- Write Merkle root storage mechanisms for batch tasks.
- Allow assignees to verify task details against the Merkle root on-chain.
- Support bulk deposit splits.

## Acceptance Criteria
- [ ] Merkle verification methods for task data validation.
- [ ] Bulk deposit handlers.
- [ ] Test checking correct verification of individual tasks inside the batch.
"""
    },
    {
        "id": "067",
        "title": "Dynamic Fee Schedule Optimization Model",
        "labels": ["smart-contract", "defi", "hard"],
        "body": """# #067: Dynamic Fee Schedule Optimization Model

## Overview
Optimize contract fee collection by implementing a dynamic schedule that programmatically adjusts platform basis points based on TVL and transaction frequency.

## Objectives
- Calculate active platform fee dynamically.
- Implement TVL-based discount brackets for high-volume creators.
- Guarantee fee calculations never underflow or exceed the 10% hard cap.

## Acceptance Criteria
- [ ] Dynamic fee evaluation engine.
- [ ] Safe math bounds for fee limits.
- [ ] Unit tests verifying fee adjustment behavior at different TVL levels.
"""
    },
    {
        "id": "068",
        "title": "Cryptographic Signature-Based Gasless Task Assignment",
        "labels": ["smart-contract", "auth", "hard"],
        "body": """# #068: Cryptographic Signature-Based Gasless Task Assignment

## Overview
Allow contributors to claim tasks gaslessly using cryptographic ECDSA signatures signed off-chain and verified inside the smart contract by a gas relayer.

## Objectives
- Support structured JSON message validation.
- Verify signer credentials using SECP256k1/Ed25519 methods.
- Track nonces per contributor to prevent message replay.

## Acceptance Criteria
- [ ] Off-chain signature parsing and validation.
- [ ] Relayer execution endpoints.
- [ ] Unit tests verifying gasless assignments.
"""
    },
    {
        "id": "069",
        "title": "Decentralized Oracle Integration for Milestone Verification",
        "labels": ["smart-contract", "oracle", "hard"],
        "body": """# #069: Decentralized Oracle Integration for Milestone Verification

## Overview
Integrate decentralized oracles to automatically verify GitHub Pull Request mergers, automating milestone payouts without creator intervention.

## Objectives
- Support incoming oracle callback data.
- Validate oracle cryptographic signatures.
- Execute auto-completion of milestones upon merge verification.

## Acceptance Criteria
- [ ] Oracle receiver interface.
- [ ] Signature check for whitelisted oracle addresses.
- [ ] Unit test simulating GitHub pull request merge callback.
"""
    },
    {
        "id": "070",
        "title": "Cross-Chain Escrow Settlement Bridge",
        "labels": ["smart-contract", "bridge", "hard"],
        "body": """# #070: Cross-Chain Escrow Settlement Bridge

## Overview
Build a bridge interface to allow cross-chain task rewards deposited on EVM chains to trigger task listings and payouts natively on Soroban.

## Objectives
- Parse bridge event validation proofs on-chain.
- Keep track of bridged deposits to prevent double spending.
- Support locking assets on EVM and distributing on Stellar.

## Acceptance Criteria
- [ ] Bridge proof validation helper.
- [ ] Locked asset registry.
- [ ] Test simulating complete EVM-to-Soroban settlement flow.
"""
    },
    {
        "id": "071",
        "title": "Task Collateral Staking for Contributor Accountability",
        "labels": ["smart-contract", "reputation", "hard"],
        "body": """# #071: Task Collateral Staking for Contributor Accountability

## Overview
Require assignees to stake collateral when claiming tasks. The collateral is automatically returned on completion, or slashed and refunded to the creator if abandoned.

## Objectives
- Implement a collateral mapping for active tasks.
- Track active deadlines and identify abandoned tasks.
- Release or slash staked collateral based on task finality.

## Acceptance Criteria
- [ ] Staking and slashing methods.
- [ ] Deadline checking rules.
- [ ] Unit tests checking correct refund and slashing execution.
"""
    },
    {
        "id": "072",
        "title": "Privacy-Preserving Reputation Leaderboard using Oblivious Transfer",
        "labels": ["smart-contract", "cryptography", "hard"],
        "body": """# #072: Privacy-Preserving Reputation Leaderboard using Oblivious Transfer

## Overview
Implement an obfuscated ranking protocol that ranks users on the leaderboard without disclosing their exact reputation score.

## Objectives
- Support zero-knowledge range proofs for reputation comparisons.
- Construct the leaderboard using relative rank proofs.
- Verify rankings on-chain without revealing private scores.

## Acceptance Criteria
- [ ] Comparative range verification module.
- [ ] Obfuscated rank tracking.
- [ ] Test cases demonstrating valid leaderboard sorting.
"""
    },
    {
        "id": "073",
        "title": "Fuzzing Escrow and Milestone States with Echidna/Proptest",
        "labels": ["testing", "fuzzing", "hard"],
        "body": """# #073: Fuzzing Escrow and Milestone States with Echidna/Proptest

## Overview
Build a stateful fuzzing environment that executes arbitrary series of milestone modifications to verify that escrow funds can never be locked or over-allocated.

## Objectives
- Write stateful invariants for escrow accounts.
- Generate hundreds of thousands of random milestone paths.
- Capture and resolve all invariant violations.

## Acceptance Criteria
- [ ] State transition fuzz targets.
- [ ] Multi-hour fuzz execution showing zero failures.
- [ ] Setup guide in testing documentation.
"""
    },
    {
        "id": "074",
        "title": "Multi-Stablecoin Swap Path Optimization for Escrows",
        "labels": ["smart-contract", "defi", "hard"],
        "body": """# #074: Multi-Stablecoin Swap Path Optimization for Escrows

## Overview
Integrate swap routes inside task creation to allow creators to fund escrows with arbitrary tokens, swapping them atomically to the target stablecoin.

## Objectives
- Integrate with Soroban AMM routers.
- Handle multi-hop swaps in single-transaction executions.
- Enforce slippage limits to reject inefficient routes.

## Acceptance Criteria
- [ ] AMM routing integration module.
- [ ] Slippage validation checks.
- [ ] Unit tests for multi-hop swap funding.
"""
    },
    {
        "id": "075",
        "title": "Dynamic Reputation Decay Mechanism",
        "labels": ["smart-contract", "reputation", "hard"],
        "body": """# #075: Dynamic Reputation Decay Mechanism

## Overview
Implement an on-chain decay function that periodically reduces reputation points for inactive accounts, prompting continuous ecosystem contribution.

## Objectives
- Track last activity timestamp per profile.
- Define a time-based decay function (e.g., 5% drop after 30 days of inactivity).
- Trigger decay check on profile reads/writes.

## Acceptance Criteria
- [ ] Decay math function in `reputation.rs`.
- [ ] Inactivity validation checks.
- [ ] Unit tests showing decay progression.
"""
    },
    {
        "id": "076",
        "title": "Decentralized Dispute Escrow Splitting with Multi-Sig Arbitration",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #076: Decentralized Dispute Escrow Splitting with Multi-Sig Arbitration

## Overview
Enhance dispute resolution to allow arbitrators to split escrowed funds among multiple assignees and creators according to specific negotiated percentages.

## Objectives
- Support fractional splitting parameters in dispute resolution endpoints.
- Verify threshold arbitrator signatures.
- Distribute rewards atomically while charging correct fees.

## Acceptance Criteria
- [ ] Fractional split math checked against total balance.
- [ ] Multi-sig arbitrator execution handler.
- [ ] Tests covering split payouts.
"""
    },
    {
        "id": "077",
        "title": "Soroban WASM Size Reduction & AssemblyScript Porting Review",
        "labels": ["performance", "tooling", "hard"],
        "body": """# #077: Soroban WASM Size Reduction & AssemblyScript Porting Review

## Overview
Audit and optimize WASM compiler structures, refactoring heavy library calls into micro-sized helpers to fit within Soroban limits.

## Objectives
- Run detailed WASM size profiling on current binaries.
- Replace heavy standard libraries with lightweight alternatives.
- Reduce compiled WASM footprint below 45KB.

## Acceptance Criteria
- [ ] WASM size profiling reports.
- [ ] Refactored code showing measurable binary size reduction.
- [ ] 100% test suite compatibility.
"""
    },
    {
        "id": "078",
        "title": "Delegated Voting Power with Decay & Revocation",
        "labels": ["smart-contract", "governance", "hard"],
        "body": """# #078: Delegated Voting Power with Decay & Revocation

## Overview
Implement liquid democracy delegation features in `governance.rs` allowing users to delegate voting weight with automatic decay.

## Objectives
- Build delegation mapping and delegation keys.
- Calculate delegate power dynamically considering decay curves.
- Support instant revocation of delegated votes.

## Acceptance Criteria
- [ ] Delegation methods and mapping.
- [ ] Weight decay curves verified.
- [ ] Unit tests for liquid democracy.
"""
    },
    {
        "id": "079",
        "title": "Time-Locked Milestone Vesting Vaults",
        "labels": ["smart-contract", "defi", "hard"],
        "body": """# #079: Time-Locked Milestone Vesting Vaults

## Overview
Create time-locked vesting vaults for approved milestone payouts, allowing a holding period during which payouts can be disputed.

## Objectives
- Hold payout amounts in a vesting state for a safety window.
- Let admins/creators open disputes during the vesting period.
- Complete release if no dispute is opened within the window.

## Acceptance Criteria
- [ ] Vesting vault mappings and time checks.
- [ ] Claim blocks and dispute handlers during vesting.
- [ ] Unit tests validating delayed payouts.
"""
    },
    {
        "id": "080",
        "title": "Sybil-Resistant Identity Staking and Verification",
        "labels": ["smart-contract", "security", "hard"],
        "body": """# #080: Sybil-Resistant Identity Staking and Verification

## Overview
Protect governance voting against Sybil attacks by integrating Gitcoin Passport cryptographic proofs or local KYC anchor verification.

## Objectives
- Integrate cryptographic verification of identity passports.
- Store verified identity hashes to prevent duplicate profile creation.
- Gate governance proposal creation behind verification checks.

## Acceptance Criteria
- [ ] Identity proof validator helper.
- [ ] Duplicate check on passport hash registry.
- [ ] Unit tests checking Sybil prevention rules.
"""
    },
    {
        "id": "081",
        "title": "Comprehensive Stress-Testing & Gas Profiling Pipeline",
        "labels": ["testing", "performance", "hard"],
        "body": """# #081: Comprehensive Stress-Testing & Gas Profiling Pipeline

## Overview
Build automated stress-testing scripts to profile gas consumption limits under heavy concurrent operations (e.g. 500 active tasks, 1000 claims).

## Objectives
- Simulate extreme concurrent user interactions.
- Log gas limits and execution times.
- Prevent transaction failures due to gas overflows.

## Acceptance Criteria
- [ ] Stress-testing benchmark script.
- [ ] Detailed logs showing gas profiles.
- [ ] Passing builds under simulated heavy loads.
"""
    },
    {
        "id": "082",
        "title": "On-Chain Audit Log with Ephemeral State Proofs",
        "labels": ["smart-contract", "audit", "hard"],
        "body": """# #082: On-Chain Audit Log with Ephemeral State Proofs

## Overview
Implement an audit log system that writes cryptographic state root hashes on-chain, enabling off-chain indexers to verify historical state.

## Objectives
- Generate state root hashes after critical mutative calls.
- Store root hash logs in compact on-chain storage.
- Standardize verification queries.

## Acceptance Criteria
- [ ] State hashing module.
- [ ] Audit log queries.
- [ ] Unit tests verifying root matching.
"""
    },
    {
        "id": "083",
        "title": "Upgradeable Storage Layout and Struct Migrator",
        "labels": ["smart-contract", "architecture", "hard"],
        "body": """# #083: Upgradeable Storage Layout and Struct Migrator

## Overview
Design a dynamic on-chain migration framework that converts legacy data structure versions (e.g. Task v1) to updated formats on-the-fly.

## Objectives
- Implement version tags on all storage structs.
- Build dynamic deserialization helpers.
- Run migration when legacy structs are read.

## Acceptance Criteria
- [ ] Versioned structs and deserializers.
- [ ] Auto-migration tests.
- [ ] Benchmark showing migration overhead.
"""
    },
    {
        "id": "084",
        "title": "Decentralized Reward Treasury Vesting & Distribution Manager",
        "labels": ["smart-contract", "defi", "hard"],
        "body": """# #084: Decentralized Reward Treasury Vesting & Distribution Manager

## Overview
Implement a vesting scheduler for the protocol reward treasury, releasing community incentives along a decay curve.

## Objectives
- Manage treasury balances.
- Schedule decay-based releases.
- Guard claims against over-allocation.

## Acceptance Criteria
- [ ] Vesting calculations and schedulers.
- [ ] Safe withdrawal handlers.
- [ ] Unit tests verifying decay curve values.
"""
    },
    {
        "id": "085",
        "title": "Multi-Signature Role Management Recovery Protocol",
        "labels": ["smart-contract", "security", "hard"],
        "body": """# #085: Multi-Signature Role Management Recovery Protocol

## Overview
Build an emergency social recovery mechanism for administrative roles using threshold signatures from Master/Legend tier users.

## Objectives
- Implement recovery proposals.
- Collect cryptographic threshold signatures from top contributors.
- Update admin addresses upon validation.

## Acceptance Criteria
- [ ] Recovery state machines.
- [ ] Role transition validations.
- [ ] Test cases verifying successful recovery.
"""
    }
]

def create_local_files():
    print("Writing 30 new hard issue markdown files to docs/issues/...")
    for issue in THIRTY_ISSUES:
        filename = f"{issue['id']}-{issue['title'].lower().replace(' ', '-').replace('/', '-').replace('(', '').replace(')', '')}.md"
        filepath = os.path.join(ISSUES_DIR, filename)
        with open(filepath, "w") as f:
            f.write(issue['body'])
        print(f"Created {filename}")

def push_to_github():
    print("\nPushing 30 new hard issues to GitHub repository via REST API...")
    url = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/issues"
    
    for issue in THIRTY_ISSUES:
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
