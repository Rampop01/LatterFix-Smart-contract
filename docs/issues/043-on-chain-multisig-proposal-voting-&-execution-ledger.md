# #043: On-Chain Multisig Proposal Voting & Execution Ledger

## Overview
Build an on-chain proposal creation, approval voting, and execution state machine for admin transactions.

## Objectives
- Allow admins to propose parameter changes or treasury fund movements.
- Collect cryptographic signatures / approval votes from authorized multisig members on-chain.
- Execute proposal automatically once threshold approval is reached.

## Acceptance Criteria
- [ ] Proposal data structures and status workflow Pending, Approved, Executed, Cancelled.
- [ ] `vote_proposal` and `execute_proposal` endpoints.
- [ ] Unit tests covering proposal lifecycle.
