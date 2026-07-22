# P1-T1: Define `InstrumentPreset` Enum and Metadata Schema Constants

## Context

You are implementing **Phase 1, Task 1** of a wealth system transformation for Wealthfolio — a privacy-first, self-hosted Rust/React portfolio tracker.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

The goal is to transform Wealthfolio into a comprehensive **Indian wealth management system**. The design principle is "Evolve in place" — Indian financial instruments (FDs, PPF, EPF, NPS, etc.) are modeled using the existing `AssetKind::Other` with structured JSON in the `attributes` / `metadata` field. No new top-level schema changes for instruments.

## Your Task

Add the `InstrumentPreset` enum and metadata schema constants to the Rust backend.

### Files to modify

1. **`crates/core/src/assets/assets_model.rs`** — Add `InstrumentPreset` enum  
2. **`crates/core/src/assets/assets_constants.rs`** — Add metadata schema constants

### Exact requirements

#### 1. `InstrumentPreset` enum in `assets_model.rs`

Add after the existing `AssetKind` enum:

```rust
/// Classification hint stored in asset metadata (`instrument_preset` field).
///
/// This is NOT a schema-level change — it's a string tag stored in the existing
/// `attributes` JSON column. It tells the UI and validation logic which
/// Indian/alternative financial instrument this `Other` asset represents.
///
/// Stored in metadata as: `"instrument_preset": "fixed_deposit"` (snake_case string)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum InstrumentPreset {
    FixedDeposit,
    RecurringDeposit,
    Ppf,
    Epf,
    Nps,
    InsuranceTerm,
    InsuranceEndowment,
    InsuranceUlip,
    InsuranceHealth,
    HomeLoan,
    CarLoan,
    PersonalLoan,
    CreditCard,
    CashInHand,
    SovereignGoldBond,
}
```

Add a helper on `InstrumentPreset`:

```rust
impl InstrumentPreset {
    /// The metadata key used to identify instrument preset in JSON.
    pub const METADATA_KEY: &'static str = "instrument_preset";

    /// Returns the AssetKind that should be used for this preset.
    pub fn expected_asset_kind(&self) -> AssetKind {
        match self {
            InstrumentPreset::FixedDeposit
            | InstrumentPreset::RecurringDeposit
            | InstrumentPreset::Ppf
            | InstrumentPreset::Epf
            | InstrumentPreset::Nps
            | InstrumentPreset::InsuranceTerm
            | InstrumentPreset::InsuranceEndowment
            | InstrumentPreset::InsuranceUlip
            | InstrumentPreset::InsuranceHealth
            | InstrumentPreset::CashInHand => AssetKind::Other,
            InstrumentPreset::HomeLoan
            | InstrumentPreset::CarLoan
            | InstrumentPreset::PersonalLoan
            | InstrumentPreset::CreditCard => AssetKind::Liability,
            InstrumentPreset::SovereignGoldBond => AssetKind::Investment,
        }
    }

    /// Returns true if this preset represents a liquid asset.
    pub fn is_liquid(&self) -> bool {
        matches!(
            self,
            InstrumentPreset::CashInHand
                | InstrumentPreset::RecurringDeposit
                | InstrumentPreset::Nps // Tier II is liquid; caller must check tier
        )
    }

    /// Returns a human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            InstrumentPreset::FixedDeposit => "Fixed Deposit",
            InstrumentPreset::RecurringDeposit => "Recurring Deposit",
            InstrumentPreset::Ppf => "Public Provident Fund (PPF)",
            InstrumentPreset::Epf => "Employee Provident Fund (EPF)",
            InstrumentPreset::Nps => "National Pension System (NPS)",
            InstrumentPreset::InsuranceTerm => "Term Life Insurance",
            InstrumentPreset::InsuranceEndowment => "Endowment / Whole Life Insurance",
            InstrumentPreset::InsuranceUlip => "ULIP",
            InstrumentPreset::InsuranceHealth => "Health Insurance",
            InstrumentPreset::HomeLoan => "Home Loan",
            InstrumentPreset::CarLoan => "Car Loan",
            InstrumentPreset::PersonalLoan => "Personal Loan",
            InstrumentPreset::CreditCard => "Credit Card",
            InstrumentPreset::CashInHand => "Cash in Hand",
            InstrumentPreset::SovereignGoldBond => "Sovereign Gold Bond (SGB)",
        }
    }
}
```

#### 2. Metadata schema constants in `assets_constants.rs`

Add constants that define the **expected metadata fields** for each preset. These are used for validation in the next task (P1-T2). Use `&'static [&'static str]` slices:

```rust
// ── Indian Instrument Metadata Schemas ────────────────────────────────────────

/// Required metadata fields for Fixed Deposit assets.
pub const FD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "bank_name",
    "principal",
    "interest_rate_pct",
    "start_date",
    "maturity_date",
];

pub const FD_OPTIONAL_FIELDS: &[&str] = &[
    "account_number",
    "compounding_frequency", // "monthly" | "quarterly" | "half_yearly" | "annually"
    "tenure_months",
    "is_tax_saver",
    "auto_renewal",
    "nomination",
];

/// Required metadata fields for Recurring Deposit assets.
pub const RD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "bank_name",
    "monthly_installment",
    "interest_rate_pct",
    "start_date",
    "maturity_date",
];

pub const RD_OPTIONAL_FIELDS: &[&str] = &[
    "account_number",
    "tenure_months",
    "total_installments",
];

/// Required metadata fields for PPF assets.
pub const PPF_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "account_number",
    "bank_or_post_office",
];

pub const PPF_OPTIONAL_FIELDS: &[&str] = &[
    "current_interest_rate_pct",
    "opening_date",
    "maturity_date",
    "financial_year_contributions",  // JSON array of {fy, amount}
    "rate_history",                  // JSON array of {from_date, rate_pct}
];

/// Required metadata fields for EPF assets.
pub const EPF_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "uan_number",
];

pub const EPF_OPTIONAL_FIELDS: &[&str] = &[
    "employer_name",
    "employee_contribution_pct",
    "employer_contribution_pct",
    "eps_component",
    "current_balance",
    "as_of_date",
];

/// Required metadata fields for NPS assets.
pub const NPS_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "pran_number",
    "tier", // "I" | "II"
];

pub const NPS_OPTIONAL_FIELDS: &[&str] = &[
    "fund_manager",
    "scheme_preference", // "E" | "C" | "G" | "A"
    "nav",
    "units_held",
    "fund_value",
];

/// Required metadata fields for Loan/Liability presets.
pub const LOAN_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "lender",
    "principal_amount",
    "interest_rate_pct",
    "tenure_months",
    "emi_amount",
    "disbursement_date",
];

pub const LOAN_OPTIONAL_FIELDS: &[&str] = &[
    "loan_account_number",
    "emi_start_date",
    "linked_asset_id",
    "prepayment_charges_pct",
];

/// Required metadata fields for Credit Card liability.
pub const CREDIT_CARD_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "card_issuer",
    "credit_limit",
    "current_outstanding",
    "due_date",
];

pub const CREDIT_CARD_OPTIONAL_FIELDS: &[&str] = &[
    "card_last_4",
    "minimum_due",
    "interest_rate_pct",
    "reward_type",
];

/// Required metadata fields for Cash in Hand.
pub const CASH_REQUIRED_FIELDS: &[&str] = &["instrument_preset"];

pub const CASH_OPTIONAL_FIELDS: &[&str] = &["location", "denomination_notes"];

/// Required metadata fields for Sovereign Gold Bond.
pub const SGB_REQUIRED_FIELDS: &[&str] = &[
    "instrument_preset",
    "series_name",
    "issue_price",
    "maturity_date",
];

pub const SGB_OPTIONAL_FIELDS: &[&str] = &[
    "interest_rate_pct",
    "quantity_grams",
    "purchase_date",
];
```

#### 3. Pub-use `InstrumentPreset` from the `assets` module

In `crates/core/src/assets/mod.rs`, re-export `InstrumentPreset`:

```rust
pub use assets_model::InstrumentPreset;
```

(Check the existing re-exports in `mod.rs` and follow the same pattern.)

### Tests to write

In `crates/core/src/assets/assets_model.rs` (or a `assets_model_tests.rs` if one exists), add unit tests:

```rust
#[cfg(test)]
mod instrument_preset_tests {
    use super::*;

    #[test]
    fn test_fixed_deposit_expected_kind() {
        assert_eq!(InstrumentPreset::FixedDeposit.expected_asset_kind(), AssetKind::Other);
    }

    #[test]
    fn test_home_loan_expected_kind() {
        assert_eq!(InstrumentPreset::HomeLoan.expected_asset_kind(), AssetKind::Liability);
    }

    #[test]
    fn test_sgb_expected_kind() {
        assert_eq!(InstrumentPreset::SovereignGoldBond.expected_asset_kind(), AssetKind::Investment);
    }

    #[test]
    fn test_serde_round_trip() {
        let preset = InstrumentPreset::FixedDeposit;
        let json = serde_json::to_string(&preset).unwrap();
        assert_eq!(json, "\"fixed_deposit\"");
        let back: InstrumentPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back, preset);
    }

    #[test]
    fn test_display_names_non_empty() {
        let presets = [
            InstrumentPreset::FixedDeposit,
            InstrumentPreset::Ppf,
            InstrumentPreset::Epf,
            InstrumentPreset::Nps,
            InstrumentPreset::HomeLoan,
            InstrumentPreset::CashInHand,
        ];
        for p in &presets {
            assert!(!p.display_name().is_empty());
        }
    }
}
```

### Compile check

After implementing, run:

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -20
```

Fix all errors before reporting done.

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/assets/assets_model.rs crates/core/src/assets/assets_constants.rs crates/core/src/assets/mod.rs
git commit -m "feat(assets): add InstrumentPreset enum and metadata schema constants"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-1-report.md`

Return only:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
