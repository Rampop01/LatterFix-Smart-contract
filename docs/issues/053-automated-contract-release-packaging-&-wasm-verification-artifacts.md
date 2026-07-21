# #053: Automated Contract Release Packaging & WASM Verification Artifacts

## Overview
Set up automated release pipeline that builds reproducible WASM binaries and attaches SHA-256 checksums to GitHub releases.

## Objectives
- Build WASM targets inside deterministic Docker container to guarantee binary reproducibility.
- Generate cryptographic hash manifest `checksums.txt` for WASM builds.
- Automate GitHub Release asset attachment on tag creation.

## Acceptance Criteria
- [ ] Release workflow configured in `.github/workflows/release.yml`.
- [ ] Reproducible WASM verification script.
- [ ] Release workflow tested.
