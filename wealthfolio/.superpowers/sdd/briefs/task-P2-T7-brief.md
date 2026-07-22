# P2-T7: Cash-in-Hand Preset End-to-End Verification

## Task Overview

Verify and ensure complete end-to-end functionality for the Cash-in-Hand preset across backend classification, valuation, frontend quick-add, and detail cards. Add unit tests verifying Cash-in-Hand metadata handling and net worth integration.

## Requirements

1. Verify that `InstrumentPreset::CashInHand`:
   - Is classified as `AssetKind::Other` with `LiquidityClass::Liquid` and `RiskClass::Safe` (already implemented in `classification_service.rs`)
   - Computes net worth value directly from its current value quote
   - Handles metadata fields (`location`, `denomination_notes`) cleanly
2. Add explicit unit test in `crates/core/src/assets/classification_service.rs` and `crates/core/src/portfolio/net_worth/net_worth_service_tests.rs` for CashInHand asset behavior.
3. Ensure no regressions in `cargo check --workspace` or `cargo test --workspace`.

## Files to Create/Modify
- `crates/core/src/assets/classification_service.rs` (add/enhance test)
- `crates/core/src/portfolio/net_worth/net_worth_service_tests.rs` (add cash-in-hand net worth test)
