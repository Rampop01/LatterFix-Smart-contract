# #031: Soroban Diagnostic Event Filter & Subgraph Indexing Specs

## Overview
Expand event payloads to support granular diagnostic indexing by external Graph / Goldsky indexers.

## Objectives
- Include transaction correlation IDs and block sequence metadata in published event vectors.
- Publish contract state transition snapshots during major lifecycle operations.
- Ensure event fields align with GraphQL schema definitions.

## Acceptance Criteria
- [ ] Event schema dictionary document created for indexer authors.
- [ ] Correlation fields added to contract events.
- [ ] Integration test verifying event JSON serialization format.
