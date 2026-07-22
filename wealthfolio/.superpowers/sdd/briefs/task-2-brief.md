# P1-T2: Implement Metadata Validation Per Instrument Preset

## Context

You are implementing **Phase 1, Task 2** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

Task 1 added the `InstrumentPreset` enum (in `assets_model.rs`) and field-name constants (`FD_REQUIRED_FIELDS`, etc. in `assets_constants.rs`). This task adds **validation logic** that uses those constants.

## Your Task

Create a metadata validation service function that validates an asset's metadata JSON against the expected schema for its instrument preset.

### Files to create/modify

1. **`crates/core/src/assets/alternative_assets_service.rs`** — Add `validate_instrument_preset_metadata` function
2. **`crates/core/src/assets/alternative_assets_service.rs`** — Add `extract_instrument_preset` helper
3. Write unit tests in the same file or in a test submodule

### Exact requirements

#### 1. Helper: extract preset from metadata JSON

```rust
use crate::assets::assets_model::InstrumentPreset;
use serde_json::Value;

/// Extracts the `InstrumentPreset` from an asset's metadata JSON value.
/// Returns `None` if the field is missing or unrecognized.
pub fn extract_instrument_preset(metadata: &Value) -> Option<InstrumentPreset> {
    let preset_str = metadata.get(InstrumentPreset::METADATA_KEY)?.as_str()?;
    serde_json::from_value(Value::String(preset_str.to_string())).ok()
}
```

#### 2. Validation function

```rust
use crate::assets::assets_constants::{
    FD_REQUIRED_FIELDS, RD_REQUIRED_FIELDS, PPF_REQUIRED_FIELDS, EPF_REQUIRED_FIELDS,
    NPS_REQUIRED_FIELDS, LOAN_REQUIRED_FIELDS, CREDIT_CARD_REQUIRED_FIELDS,
    CASH_REQUIRED_FIELDS, SGB_REQUIRED_FIELDS,
};
use crate::errors::{Result, ValidationError};

/// Validates that the metadata JSON for an alternative asset contains all
/// required fields for its declared `InstrumentPreset`.
///
/// Returns `Ok(())` if:
/// - metadata is `None` (no preset declared — always valid)
/// - metadata is `Some` but has no `instrument_preset` field (no preset — always valid)
/// - all required fields for the declared preset are present and non-null
///
/// Returns `Err(ValidationError)` if required fields are missing.
pub fn validate_instrument_preset_metadata(metadata: &Option<Value>) -> Result<()> {
    let Some(meta) = metadata else {
        return Ok(());
    };

    let Some(preset) = extract_instrument_preset(meta) else {
        return Ok(()); // no preset declared — not our problem
    };

    let required = match preset {
        InstrumentPreset::FixedDeposit => FD_REQUIRED_FIELDS,
        InstrumentPreset::RecurringDeposit => RD_REQUIRED_FIELDS,
        InstrumentPreset::Ppf => PPF_REQUIRED_FIELDS,
        InstrumentPreset::Epf => EPF_REQUIRED_FIELDS,
        InstrumentPreset::Nps => NPS_REQUIRED_FIELDS,
        InstrumentPreset::InsuranceTerm
        | InstrumentPreset::InsuranceEndowment
        | InstrumentPreset::InsuranceUlip
        | InstrumentPreset::InsuranceHealth => &["instrument_preset", "policy_number", "insurer"],
        InstrumentPreset::HomeLoan
        | InstrumentPreset::CarLoan
        | InstrumentPreset::PersonalLoan => LOAN_REQUIRED_FIELDS,
        InstrumentPreset::CreditCard => CREDIT_CARD_REQUIRED_FIELDS,
        InstrumentPreset::CashInHand => CASH_REQUIRED_FIELDS,
        InstrumentPreset::SovereignGoldBond => SGB_REQUIRED_FIELDS,
    };

    let missing: Vec<&str> = required
        .iter()
        .filter(|&&field| meta.get(field).map_or(true, |v| v.is_null()))
        .copied()
        .collect();

    if !missing.is_empty() {
        return Err(crate::Error::Validation(ValidationError::InvalidInput(
            format!(
                "Missing required metadata fields for preset '{:?}': {}",
                preset,
                missing.join(", ")
            ),
        )));
    }

    Ok(())
}
```

#### 3. Call site integration

Find where `CreateAlternativeAssetRequest` is validated in `alternative_assets_service.rs` (look for any existing validation call or the `create_alternative_asset` function). Call `validate_instrument_preset_metadata(&request.metadata)?` before the asset is persisted.

If there is no central validation point, add the call at the start of the create function.

#### 4. Tests to write

```rust
#[cfg(test)]
mod preset_validation_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_metadata_is_valid() {
        assert!(validate_instrument_preset_metadata(&None).is_ok());
    }

    #[test]
    fn test_metadata_without_preset_is_valid() {
        let meta = Some(json!({ "some_field": "value" }));
        assert!(validate_instrument_preset_metadata(&meta).is_ok());
    }

    #[test]
    fn test_valid_fd_metadata() {
        let meta = Some(json!({
            "instrument_preset": "fixed_deposit",
            "bank_name": "SBI",
            "principal": 500000,
            "interest_rate_pct": 7.1,
            "start_date": "2025-01-15",
            "maturity_date": "2027-01-15"
        }));
        assert!(validate_instrument_preset_metadata(&meta).is_ok());
    }

    #[test]
    fn test_fd_missing_required_field() {
        let meta = Some(json!({
            "instrument_preset": "fixed_deposit",
            "bank_name": "SBI"
            // missing: principal, interest_rate_pct, start_date, maturity_date
        }));
        let result = validate_instrument_preset_metadata(&meta);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("principal") || err_msg.contains("Missing"));
    }

    #[test]
    fn test_extract_preset_from_json() {
        let meta = json!({ "instrument_preset": "ppf" });
        let preset = extract_instrument_preset(&meta);
        assert_eq!(preset, Some(InstrumentPreset::Ppf));
    }

    #[test]
    fn test_extract_preset_missing_key() {
        let meta = json!({ "some_other_key": "value" });
        assert_eq!(extract_instrument_preset(&meta), None);
    }

    #[test]
    fn test_valid_cash_metadata() {
        let meta = Some(json!({ "instrument_preset": "cash_in_hand" }));
        assert!(validate_instrument_preset_metadata(&meta).is_ok());
    }

    #[test]
    fn test_valid_nps_metadata() {
        let meta = Some(json!({
            "instrument_preset": "nps",
            "pran_number": "XXXX1234",
            "tier": "I"
        }));
        assert!(validate_instrument_preset_metadata(&meta).is_ok());
    }
}
```

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -20
```

Fix all errors before reporting done.

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/assets/alternative_assets_service.rs
git commit -m "feat(assets): add instrument preset metadata validation"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-2-report.md`

Return only:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
