# #032: Time-Weighted Average Price TWAP Oracle Module

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
