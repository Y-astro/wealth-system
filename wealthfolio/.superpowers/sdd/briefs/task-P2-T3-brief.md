# P2-T3: Insurance Valuation Logic + Premium Status

## Task Overview

Create insurance valuation functions in the `fixed_income_calculator.rs` module (or a new `insurance_calculator.rs` in the same directory), and integrate them into the net worth service's `compute_preset_value` function.

## Requirements

### 1. Insurance Valuation Functions

The system has 4 insurance preset types (already defined in `InstrumentPreset` enum):

| Preset | Net Worth Contribution | Valuation Method |
|--------|----------------------|------------------|
| `InsuranceTerm` | ₹0 (pure protection) | No value - returns `Decimal::ZERO` |
| `InsuranceHealth` | ₹0 (pure protection) | No value - returns `Decimal::ZERO` |
| `InsuranceEndowment` | Yes — surrender value | Read `surrender_value` from metadata, return it |
| `InsuranceUlip` | Yes — fund value | Either read `fund_value` from metadata, or compute `nav × units_held` |

Create the following functions:

```rust
/// Returns the net-worth-contributing value for an insurance policy.
/// Term and Health insurance return Decimal::ZERO (pure protection).
/// Endowment returns surrender_value from metadata.
/// ULIP returns fund_value from metadata, or falls back to nav * units_held.
pub fn calculate_insurance_value(
    preset: &InstrumentPreset,
    metadata: &serde_json::Value,
) -> Decimal
```

### 2. Premium Status Tracking

Create a helper that determines premium status:

```rust
pub struct PremiumStatus {
    pub premium_annual: Option<Decimal>,
    pub premium_due_date: Option<NaiveDate>,
    pub is_premium_overdue: bool,
    pub days_until_due: Option<i64>,
}

/// Extract premium status from insurance metadata.
pub fn get_premium_status(
    metadata: &serde_json::Value,
    today: NaiveDate,
) -> PremiumStatus
```

Fields from metadata: `premium_annual`, `premium_due_date` (ISO date string).

### 3. Net Worth Integration

In `crates/core/src/portfolio/net_worth/net_worth_service.rs`, add match arms to `compute_preset_value` for insurance presets:

```rust
InstrumentPreset::InsuranceTerm | InstrumentPreset::InsuranceHealth => {
    Some(Decimal::ZERO)  // Pure protection policies
}
InstrumentPreset::InsuranceEndowment | InstrumentPreset::InsuranceUlip => {
    Some(calculate_insurance_value(&preset, metadata))
}
```

### 4. Tests

Write unit tests covering:
- Term insurance returns zero
- Health insurance returns zero
- Endowment returns surrender value from metadata
- ULIP returns fund_value when present
- ULIP falls back to nav × units_held when fund_value is absent
- Premium status: overdue detection
- Premium status: upcoming due date calculation

## Existing Code Patterns

The `fixed_income_calculator.rs` module follows these patterns:
- Functions take typed parameters (Decimal, NaiveDate, &str) 
- Use `crate::errors::{Error, Result, ValidationError}` for errors
- Use `rust_decimal::Decimal` with `.round_dp(2)` for money values
- Tests use `NaiveDate::from_ymd_opt(Y, M, D).unwrap()` and `Decimal::from_f64(x).unwrap()`

The metadata fields are stored as JSON strings in the `assets.attributes` column. When reading from metadata:
- Numeric fields: `metadata.get("field")?.as_str()?.parse().ok()` — they may be stored as string or number, handle both
- Also try: `metadata.get("field")?.as_f64().map(Decimal::from_f64).flatten()`
- Date fields: `NaiveDate::parse_from_str(metadata.get("field")?.as_str()?, "%Y-%m-%d").ok()`

## Files to Create/Modify

1. **Create**: `crates/core/src/assets/insurance_calculator.rs` — valuation + premium status functions + tests
2. **Modify**: `crates/core/src/assets/mod.rs` — add `pub mod insurance_calculator;` re-export
3. **Modify**: `crates/core/src/portfolio/net_worth/net_worth_service.rs` — add insurance arms to `compute_preset_value`

## Constraints

- Do NOT modify the `InstrumentPreset` enum or `assets_constants.rs` — they already have the insurance presets and field schemas
- Match the coding style of `fixed_income_calculator.rs`
- All tests must pass: `cargo test --workspace`
- Compile cleanly: `cargo check --workspace`
