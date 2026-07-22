# P1-T5: Asset Liquidity and Risk Classification Logic

## Context

You are implementing **Phase 1, Task 5** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

Tasks 1-4 added the `InstrumentPreset` enum, validation, DB migration, and the liquidity/risk fields on `NetWorthResponse`. This task implements the **classification logic** that maps each asset to liquid/illiquid and safe/moderate/aggressive.

## Your Task

Create a classification module at `crates/core/src/assets/classification_service.rs` (it may already exist — check). This module provides pure functions for classifying any asset by its `AssetKind` and `InstrumentPreset`.

### Read first

Check the file:
```bash
wc -l /home/astro/wealth-system/wealthfolio/crates/core/src/assets/classification_service.rs
head -50 /home/astro/wealth-system/wealthfolio/crates/core/src/assets/classification_service.rs
```

If it already exists and has some classification logic, extend it without breaking existing behavior.

### Files to modify/create

- `crates/core/src/assets/classification_service.rs` — Add `classify_liquidity` and `classify_risk` functions

### Exact requirements

#### Classification rules

```
LIQUIDITY:
  Liquid:
    - CashInHand
    - RecurringDeposit (no lock-in)
    - NPS Tier-II (check metadata `tier` == "II")
    - AssetKind::Investment (stocks, ETFs, MFs — generally liquid)
    - AssetKind::PreciousMetal (can be sold)
    - Any AssetKind::Other with no InstrumentPreset
  Illiquid:
    - FixedDeposit (locked until maturity, unless `is_tax_saver` is false AND maturity is > 7 days away — simplify: treat all FDs as illiquid)
    - PPF (15-year lock-in)
    - EPF (locked until retirement)
    - NPS Tier-I (locked until 60)
    - InsuranceTerm, InsuranceHealth (no investment value)
    - InsuranceEndowment, InsuranceUlip (surrender-value instruments — illiquid)
    - AssetKind::Property
    - AssetKind::Vehicle
    - AssetKind::Collectible
    - AssetKind::PrivateEquity

RISK:
  Safe:
    - FixedDeposit, RecurringDeposit
    - PPF, EPF
    - InsuranceTerm, InsuranceEndowment, InsuranceHealth (protection, not growth)
    - CashInHand
    - SovereignGoldBond (govt-backed)
    - AssetKind::Liability → not classified (return None)
  Moderate:
    - NPS (any tier)
    - InsuranceUlip
    - AssetKind::Property (real estate — moderate long-term)
    - AssetKind::PreciousMetal (gold/silver — moderate)
    - AssetKind::Collectible
    - AssetKind::Vehicle (depreciating asset)
  Aggressive:
    - AssetKind::Investment (equities, crypto, ETFs — use Aggressive as default; allocation module handles sub-classification)
    - AssetKind::PrivateEquity
    - Any unrecognized preset → Aggressive (fail-safe)
```

#### Functions to add

```rust
use serde_json::Value;
use crate::assets::assets_model::{AssetKind, InstrumentPreset};
use crate::portfolio::net_worth::net_worth_model::{LiquidityClass, RiskClass};

/// Classify an asset as liquid or illiquid.
///
/// `metadata` is the asset's JSON attributes field.
/// Returns `LiquidityClass::Liquid` or `LiquidityClass::Illiquid`.
pub fn classify_liquidity(kind: &AssetKind, metadata: &Option<Value>) -> LiquidityClass {
    // ... implement per the rules above
}

/// Classify an asset by risk level.
///
/// Returns `None` for liabilities (not classified).
pub fn classify_risk(kind: &AssetKind, metadata: &Option<Value>) -> Option<RiskClass> {
    // ... implement per the rules above
}
```

Both functions must call `extract_instrument_preset(metadata)` from the alternative assets service (or inline the logic if you prefer to avoid the dependency direction — use `metadata.as_ref().and_then(|m| m.get("instrument_preset")).and_then(|v| v.as_str()).and_then(|s| serde_json::from_str(&format!("\"{}\"", s)).ok())` as an inline approach).

#### Integration with NetWorthService

In `crates/core/src/portfolio/net_worth/net_worth_service.rs`, use the new functions to populate the new fields in `NetWorthResponse`:

Find the section where `ValuationInfo` items are aggregated and for each asset, call `classify_liquidity` and `classify_risk`, then accumulate totals into `response.liquid_assets`, `response.illiquid_assets`, `response.safe_assets`, `response.moderate_assets`, `response.aggressive_assets`.

The metadata/attributes field should be available on the asset record — check what fields `ValuationInfo` and the asset repository return. You may need to check:
```bash
grep -n "attributes\|metadata" crates/core/src/assets/assets_model.rs | head -20
grep -n "ValuationInfo" crates/core/src/portfolio/net_worth/net_worth_service.rs | head -10
```

If attributes aren't in `ValuationInfo`, look at how asset data flows through the net worth calculation and pass the attributes along.

### Tests

```rust
#[cfg(test)]
mod classification_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fd_is_illiquid() {
        let meta = Some(json!({ "instrument_preset": "fixed_deposit" }));
        assert_eq!(classify_liquidity(&AssetKind::Other, &meta), LiquidityClass::Illiquid);
    }

    #[test]
    fn test_cash_is_liquid() {
        let meta = Some(json!({ "instrument_preset": "cash_in_hand" }));
        assert_eq!(classify_liquidity(&AssetKind::Other, &meta), LiquidityClass::Liquid);
    }

    #[test]
    fn test_investment_is_aggressive() {
        assert_eq!(classify_risk(&AssetKind::Investment, &None), Some(RiskClass::Aggressive));
    }

    #[test]
    fn test_ppf_is_safe() {
        let meta = Some(json!({ "instrument_preset": "ppf" }));
        assert_eq!(classify_risk(&AssetKind::Other, &meta), Some(RiskClass::Safe));
    }

    #[test]
    fn test_liability_has_no_risk_class() {
        assert_eq!(classify_risk(&AssetKind::Liability, &None), None);
    }

    #[test]
    fn test_property_is_moderate_risk() {
        assert_eq!(classify_risk(&AssetKind::Property, &None), Some(RiskClass::Moderate));
    }

    #[test]
    fn test_property_is_illiquid() {
        assert_eq!(classify_liquidity(&AssetKind::Property, &None), LiquidityClass::Illiquid);
    }
}
```

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -30
```

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/assets/classification_service.rs \
        crates/core/src/portfolio/net_worth/
git commit -m "feat(assets,net-worth): implement liquidity and risk classification logic"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-5-report.md`

Return:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
