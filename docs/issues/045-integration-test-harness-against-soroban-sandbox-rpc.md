# #045: Integration Test Harness against Soroban Sandbox RPC

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
