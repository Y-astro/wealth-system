# P2-T8: Integration Tests — Full Preset Lifecycle

## Task Overview

Create an end-to-end integration test suite `crates/core/tests/preset_lifecycle_tests.rs` (or in `crates/core/src/assets/preset_lifecycle_tests.rs`) that validates the complete lifecycle of all 15 Indian instrument presets: creation metadata validation, liquidity classification, risk classification, dynamic valuation in NetWorthService, and cleanup.

## Requirements

1. Test all 15 `InstrumentPreset` variants:
   - `FixedDeposit`
   - `RecurringDeposit`
   - `Ppf`
   - `Epf`
   - `Nps`
   - `InsuranceTerm`
   - `InsuranceEndowment`
   - `InsuranceUlip`
   - `InsuranceHealth`
   - `HomeLoan`
   - `CarLoan`
   - `PersonalLoan`
   - `CreditCard`
   - `CashInHand`
   - `SovereignGoldBond`

2. For each preset, assert:
   - `validate_instrument_preset_metadata` succeeds with valid metadata
   - `validate_instrument_preset_metadata` fails when required fields are missing
   - `classify_liquidity` produces expected classification (Liquid vs Illiquid)
   - `classify_risk` produces expected risk tier (Safe vs Moderate vs Aggressive vs None for liabilities)
   - `compute_preset_value` computes expected Decimal value (FD, RD, PPF, NPS, Insurance surrender/fund value, Loan outstanding, Credit Card outstanding)

3. Run `cargo test --workspace` and verify all tests pass.

## Files to Create/Modify
- **Create**: `crates/core/tests/preset_lifecycle_tests.rs` (or `crates/core/src/assets/preset_lifecycle_tests.rs`)
