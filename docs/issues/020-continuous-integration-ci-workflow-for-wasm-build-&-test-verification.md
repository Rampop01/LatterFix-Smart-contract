# #020: Continuous Integration (CI) Workflow for WASM Build & Test Verification

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
