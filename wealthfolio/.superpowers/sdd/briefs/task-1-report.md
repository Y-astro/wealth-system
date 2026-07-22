# P1-T1 Task Report: `InstrumentPreset` Enum and Metadata Schema Constants

**Status**: DONE  
**Commit**: `48a4230`  
**Branch**: `main`  
**Completed**: 2026-07-09

---

## Summary

Successfully implemented Phase 1, Task 1 of the wealth system transformation: added the `InstrumentPreset` enum and metadata schema constants to the Rust backend of Wealthfolio.

---

## Changes Made

### 1. `crates/core/src/assets/assets_model.rs`

Added after the `AssetKind` impl block:

- **`InstrumentPreset` enum** (15 variants):
  - `FixedDeposit`, `RecurringDeposit`, `Ppf`, `Epf`, `Nps`
  - `InsuranceTerm`, `InsuranceEndowment`, `InsuranceUlip`, `InsuranceHealth`
  - `HomeLoan`, `CarLoan`, `PersonalLoan`, `CreditCard`
  - `CashInHand`, `SovereignGoldBond`
  - Derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS`
  - Serde: `rename_all = "snake_case"` (stores as `"fixed_deposit"` etc.)
  - TypeScript export enabled via `#[ts(export)]`

- **`impl InstrumentPreset`** with:
  - `METADATA_KEY: &'static str = "instrument_preset"` — JSON key constant
  - `expected_asset_kind(&self) -> AssetKind` — maps presets to their correct `AssetKind`:
    - `Other`: FD, RD, PPF, EPF, NPS, Insurance variants, CashInHand
    - `Liability`: HomeLoan, CarLoan, PersonalLoan, CreditCard
    - `Investment`: SovereignGoldBond
  - `is_liquid(&self) -> bool` — `CashInHand`, `RecurringDeposit`, `Nps` are liquid
  - `display_name(&self) -> &'static str` — human-readable names for all 15 variants

- **Unit tests** in `mod instrument_preset_tests`:
  - `test_fixed_deposit_expected_kind` — FD maps to `AssetKind::Other`
  - `test_home_loan_expected_kind` — HomeLoan maps to `AssetKind::Liability`
  - `test_sgb_expected_kind` — SovereignGoldBond maps to `AssetKind::Investment`
  - `test_serde_round_trip` — FixedDeposit serializes to `"fixed_deposit"` and back
  - `test_display_names_non_empty` — 6 representative presets have non-empty display names

### 2. `crates/core/src/assets/assets_constants.rs`

Replaced the minimal stub with full metadata schema constants for all 8 instrument categories:

| Constant Group | Required Fields | Optional Fields |
|---|---|---|
| `FD_*` | 6 (preset, bank_name, principal, rate, start, maturity) | 6 |
| `RD_*` | 6 (preset, bank_name, installment, rate, start, maturity) | 3 |
| `PPF_*` | 3 (preset, account_number, bank_or_post_office) | 5 |
| `EPF_*` | 2 (preset, uan_number) | 6 |
| `NPS_*` | 3 (preset, pran_number, tier) | 5 |
| `LOAN_*` | 7 (preset, lender, principal, rate, tenure, emi, disbursement) | 4 |
| `CREDIT_CARD_*` | 5 (preset, issuer, limit, outstanding, due_date) | 4 |
| `CASH_*` | 1 (preset) | 2 |
| `SGB_*` | 4 (preset, series, issue_price, maturity) | 3 |

Total: 18 public constants exported from this module.

### 3. `crates/core/src/assets/mod.rs`

Added `InstrumentPreset` to the existing `pub use assets_model::{ ... }` re-export block, following the existing pattern.

---

## Compile Check

```
cargo check -p wealthfolio-core 2>&1 | tail -20
```

**Result**: `Finished dev profile [unoptimized + debuginfo] target(s) in 14.02s`

Only warnings: 18 unused-constant warnings for the new `assets_constants.rs` entries (expected — these will be consumed in P1-T2 validation logic).

No errors.

---

## Test Results

```
cargo test -p wealthfolio-core assets_model 2>&1 | tail -30
```

**Result**: `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 751 filtered out`

Notable passing tests from this task:
- `assets::assets_model::export_bindings_instrumentpreset` — TypeScript binding export works
- `assets::assets_model::instrument_preset_tests::test_fixed_deposit_expected_kind` — ok
- `assets::assets_model::instrument_preset_tests::test_home_loan_expected_kind` — ok
- `assets::assets_model::instrument_preset_tests::test_sgb_expected_kind` — ok
- `assets::assets_model::instrument_preset_tests::test_serde_round_trip` — ok
- `assets::assets_model::instrument_preset_tests::test_display_names_non_empty` — ok

All pre-existing tests continue to pass (no regressions).

---

## Design Decisions

1. **Tests placed inline in `assets_model.rs`** rather than in `assets_model_tests.rs` — the brief specified `assets_model.rs` as the primary location, and the new tests are tightly coupled to the `InstrumentPreset` impl. The existing `assets_model_tests.rs` covers `Asset`, `AssetKind`, etc. and was left unmodified.

2. **Unused constant warnings are expected** — The `assets_constants.rs` constants are schema definitions for use by P1-T2 (validation). They are `pub` and will be consumed in the next task. Suppressing them with `#[allow(dead_code)]` would be premature.

3. **`InstrumentPreset` follows existing enum patterns** — Uses the same derive set as `AssetKind` and `InstrumentType` (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS`) and the same TypeScript export annotation.

---

## Concerns

None. The implementation is straightforward and self-contained.
