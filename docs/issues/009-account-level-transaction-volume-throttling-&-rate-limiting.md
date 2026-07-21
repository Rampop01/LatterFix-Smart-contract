# #009: Account-Level Transaction Volume Throttling & Rate Limiting

## Overview
Implement sliding-window rate limit checks per user or employer address inside smart contract invocations.

## Objectives
- Prevent spam and malicious high-frequency calls on contract endpoints.
- Track call frequencies and cumulative withdrawal volumes per ledger window.
- Revert transactions exceeding defined risk thresholds.

## Acceptance Criteria
- [ ] Rate limit tracking state added to contract storage.
- [ ] Window evaluation logic tested against ledger timestamp / sequence.
- [ ] Unit test verifying rate limit error enforcement.
