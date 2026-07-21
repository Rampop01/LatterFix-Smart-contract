# #055: Soroban Local RPC Dev Environment Docker Compose Setup

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
