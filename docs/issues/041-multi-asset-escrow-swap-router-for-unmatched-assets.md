# #041: Multi-Asset Escrow Swap Router for Unmatched Assets

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
