# LatterFix Smart Contract

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.84%2B-orange.svg)](https://www.rust-lang.org)
[![Stellar](https://img.shields.io/badge/Stellar-Soroban-blue.svg)](https://stellar.org)
[![Tests](https://img.shields.io/badge/Tests-5%2F5%20Passing-brightgreen.svg)](https://github.com/LatterFixxx/LatterFix-Smart-contract)

<div align="center">
  <h3>TaskManager Pro — Soroban Smart Contract</h3>
  <p><i>Escrow-based decentralized task management protocol built on the Stellar Network.</i></p>
</div>

---

## Overview

This repository contains the Soroban smart contract powering the LatterFix TaskManager Pro platform. The contract implements a complete decentralized escrow workflow — locking task rewards on-chain, routing platform fees, and enabling dispute resolution between task creators and contributors.

---

## Contract Modules

| File | Description |
|------|-------------|
| `src/lib.rs` | Core escrow contract — task lifecycle, payout logic, fee routing, dispute resolution |
| `src/user_profile.rs` | Contributor profile manager — on-chain reputation and completion tracking |
| `src/test.rs` | Comprehensive unit test suite (5 tests, 100% passing) |

---

## Key Contract Methods

### Task Escrow (`lib.rs`)

*   `initialize(admin, platform_fee_bps, token_contract, fee_recipient)`
    Bootstrap the contract with admin keys, Stellar token address, and platform fee config.
*   `create_task(creator, title, description, reward, tags) -> u32`
    Deposits reward tokens into escrow and registers a new task on-chain.
*   `assign_task(assignee, task_id)`
    Claims an open task, transitioning it to `InProgress`.
*   `submit_work(assignee, task_id, delivery_url)`
    Submits a delivery URL, advancing the task to `Completed`.
*   `complete_task(caller, task_id)`
    Releases escrowed reward to assignee (minus platform fee) and closes the task.
*   `cancel_task(creator, task_id)`
    Cancels an open task and refunds the escrowed reward to the creator.
*   `dispute_task(caller, task_id)`
    Freezes escrow and moves task into `Disputed` state.
*   `resolve_dispute(admin, task_id, creator_refund, assignee_payout)`
    Admin resolves dispute with a custom token split between parties.

### User Profiles (`user_profile.rs`)

*   `create_profile(user, username, bio)`
    Registers a developer profile on-chain with a starting reputation of 100 points.
*   `update_bio(user, new_bio)`
    Updates the contributor's biography on-chain.
*   `reward_contribution(admin, user, points)`
    Increments reputation and completed task count after a verified payout.
*   `get_profile(user) -> Option<UserProfile>`
    Fetches on-chain contributor profile data.

---

## Task Status Lifecycle

```
Open → InProgress → Completed → Verified
  │         │            │
  └─────────┴────────────┴──────── Cancelled / Disputed → Resolved
```

---

## Data Structures

```rust
pub struct Task {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub reward: i128,
    pub assignee: Option<Address>,
    pub status: TaskStatus,
    pub created_by: Address,
    pub tags: Vec<String>,
}

pub struct UserProfile {
    pub address: Address,
    pub username: String,
    pub reputation: u32,
    pub completed_tasks: u32,
    pub joined_at: u64,
    pub bio: String,
}
```

---

## Development

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli
```

### Run Tests

```bash
cargo test
```

All 5 unit tests cover the complete contract lifecycle:
- `test_initialization` — Admin setup and config validation
- `test_create_and_complete_task_flow` — Escrow deposit and verified payout
- `test_cancel_task_refund` — Creator cancellation and full refund
- `test_dispute_and_resolution` — Admin-mediated 50/50 dispute split
- `test_user_profile_lifecycle` — Profile creation, bio update, reputation rewards

### Build WASM Binary

```bash
cargo build --target wasm32-unknown-unknown --release
```

### Deploy to Testnet

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/task_manager_pro.wasm \
  --network testnet \
  --source <your-account>
```

---
