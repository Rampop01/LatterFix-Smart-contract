# #035: Off-Chain EIP-712 Soroban Permit Auth Signature Structs

## Overview
Implement off-chain typed signature authorization Permit style for gasless employee payout claims.

## Objectives
- Allow employers to sign structured authorization payloads off-chain.
- Enable third-party relayers to submit claims on behalf of employees with employer signature.
- Verify typed domain separator, deadline timestamp, and nonce to prevent replay.

## Acceptance Criteria
- [ ] Typed authorization structure and signature verification logic.
- [ ] Deadline check rejecting expired permits.
- [ ] Test verifying gasless claim execution via relayer.
