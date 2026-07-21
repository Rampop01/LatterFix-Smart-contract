# #014: Implement Contract Metadata Standard (SEP-0034)

## Overview
Add SEP-0034 contract metadata specs, interface descriptions, version tag embedding, and deployment manifests.

## Objectives
- Embed standard metadata attributes (name, description, author, version, repo URL) in contract WASM.
- Provide queried getter function returning standardized JSON metadata.
- Align with Stellar ecosystem tool discoverability standards.

## Acceptance Criteria
- [ ] Metadata attributes embedded into contract WASM build.
- [ ] Query interface `get_contract_metadata` returns valid SEP-0034 format.
- [ ] Unit tests verifying metadata output consistency.
