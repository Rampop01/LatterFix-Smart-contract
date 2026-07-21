# #006: Support Multi-Stablecoin Vaults (USDC, ORGUSD, EURT)

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
