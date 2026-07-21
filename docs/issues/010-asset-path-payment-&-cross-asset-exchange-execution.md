# #010: Asset Path Payment & Cross-Asset Exchange Execution

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
