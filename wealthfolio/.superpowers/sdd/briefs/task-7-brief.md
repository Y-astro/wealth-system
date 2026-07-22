# P1-T7: Create `crates/core/src/tax/` Module Skeleton

## Context

You are implementing **Phase 1, Task 7** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

This task creates the skeleton for the new tax module in `crates/core`. The full implementation comes in Phase 5 — here we establish the module structure, public API surface, and placeholder types that downstream code can reference.

## Your Task

Create the `crates/core/src/tax/` module with skeleton files.

### Files to create

1. `crates/core/src/tax/mod.rs`
2. `crates/core/src/tax/models.rs`
3. `crates/core/src/tax/capital_gains.rs`
4. `crates/core/src/tax/deductions.rs`
5. `crates/core/src/tax/income_tax.rs`

### Also modify

6. `crates/core/src/lib.rs` — Add `pub mod tax;`

### Content

#### `mod.rs`

```rust
//! Tax calculation module for Indian income tax law.
//!
//! This module provides:
//! - Capital gains calculations (STCG/LTCG) per Indian tax rules
//! - Section 80C/80D deduction tracking
//! - Interest and dividend income aggregation
//! - ITR-compatible report generation
//!
//! **Phase 1 note**: This is a skeleton. Full implementation in Phase 5.

pub mod capital_gains;
pub mod deductions;
pub mod income_tax;
pub mod models;

pub use models::{
    CapitalGainsSummary, CapitalGainType, DeductionSummary, TaxRegime, TaxReport,
};
```

#### `models.rs`

```rust
//! Tax domain models for Indian income tax.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Indian tax regime selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum TaxRegime {
    /// Old regime with deductions (80C, 80D, HRA, etc.)
    Old,
    /// New regime with simplified slabs, no deductions
    New,
}

impl Default for TaxRegime {
    fn default() -> Self {
        TaxRegime::New
    }
}

/// Capital gain type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum CapitalGainType {
    /// Short-term capital gain (holding < 1 year for equity, < 2 years for property)
    Stcg,
    /// Long-term capital gain
    Ltcg,
}

/// Summary of capital gains for a financial year.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CapitalGainsSummary {
    /// Financial year, e.g. "FY2025-26"
    pub financial_year: String,

    // ── Equity (stocks, equity MF, ETF) ──────────────────────────────
    /// STCG on equity (taxed at 20%)
    pub equity_stcg: Decimal,
    /// LTCG on equity above ₹1.25L exempt threshold (taxed at 12.5%)
    pub equity_ltcg: Decimal,
    /// LTCG on equity that falls within ₹1.25L exemption
    pub equity_ltcg_exempt: Decimal,

    // ── Debt MF (post-2023: taxed at slab) ───────────────────────────
    /// Gains on debt MF (no STCG/LTCG distinction post-2023)
    pub debt_mf_gains: Decimal,

    // ── Property ─────────────────────────────────────────────────────
    pub property_stcg: Decimal,
    pub property_ltcg: Decimal,

    // ── Gold ─────────────────────────────────────────────────────────
    pub gold_stcg: Decimal,
    pub gold_ltcg: Decimal,

    // ── Totals ────────────────────────────────────────────────────────
    pub total_stcg: Decimal,
    pub total_ltcg: Decimal,
    pub estimated_tax_stcg: Decimal,
    pub estimated_tax_ltcg: Decimal,
}

/// Section 80C/80D deduction summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeductionSummary {
    pub financial_year: String,

    // ── Section 80C (limit ₹1,50,000) ────────────────────────────────
    pub elss_mf: Decimal,
    pub ppf_contribution: Decimal,
    pub epf_contribution: Decimal,
    pub tax_saver_fd: Decimal,
    pub life_insurance_premium: Decimal,
    pub home_loan_principal: Decimal,
    /// Total 80C deduction (capped at ₹1,50,000)
    pub total_80c: Decimal,
    pub limit_80c: Decimal,

    // ── Section 80D ──────────────────────────────────────────────────
    /// Health insurance premium (self + family)
    pub health_insurance_self: Decimal,
    /// Health insurance premium (parents)
    pub health_insurance_parents: Decimal,
    /// Total 80D deduction
    pub total_80d: Decimal,

    // ── Totals ────────────────────────────────────────────────────────
    pub total_deductions: Decimal,
}

/// Full tax report for a financial year.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaxReport {
    pub financial_year: String,
    pub regime: TaxRegime,
    pub capital_gains: CapitalGainsSummary,
    pub deductions: DeductionSummary,

    /// Total dividend income (taxable at slab rate post-2020)
    pub dividend_income: Decimal,
    /// Total interest income (FD, savings — taxable)
    pub taxable_interest_income: Decimal,
    /// PPF interest (exempt)
    pub exempt_interest_income: Decimal,

    /// The date this report was generated
    pub generated_at: NaiveDate,

    /// Disclaimer shown to user
    pub disclaimer: String,
}

impl TaxReport {
    pub const DISCLAIMER: &'static str =
        "This tax report is for reference only and is based on the data you have entered. \
         Please consult a qualified Chartered Accountant (CA) before filing your ITR.";
}
```

#### `capital_gains.rs`

```rust
//! Capital gains calculation for Indian tax law.
//!
//! TODO (Phase 5): Implement full STCG/LTCG calculations:
//! - Equity: STCG at 20% (< 1 year), LTCG at 12.5% above ₹1.25L (>= 1 year)
//! - Debt MF: Slab rate regardless of holding period (post-April 2023)
//! - Gold/Property: 20% with indexation (pre-July 2024), 12.5% without (post-July 2024)
//! - Grandfathering: equity cost = max(actual, FMV on 31-Jan-2018)

/// Financial year boundary: April 1 to March 31 (India).
pub const INDIA_FY_START_MONTH: u32 = 4;

/// Annual LTCG exemption for equity (₹1,25,000 from FY2024-25).
pub const EQUITY_LTCG_EXEMPTION: i64 = 125_000;

/// STCG tax rate on equity (20% from FY2024-25).
pub const EQUITY_STCG_RATE: f64 = 0.20;

/// LTCG tax rate on equity (12.5% from FY2024-25).
pub const EQUITY_LTCG_RATE: f64 = 0.125;

// Phase 5: implement CapitalGainsCalculator
```

#### `deductions.rs`

```rust
//! Section 80C, 80D, and other deduction tracking.
//!
//! TODO (Phase 5): Implement deduction aggregation from:
//! - ELSS MF activities tagged as 80C investments
//! - PPF contributions from InstrumentPreset::Ppf assets
//! - EPF contributions from InstrumentPreset::Epf assets
//! - Tax-saver FDs (is_tax_saver = true in FD metadata)
//! - Life insurance premiums (InsuranceTerm, InsuranceEndowment)
//! - Home loan principal repayment (from amortization schedule)

/// Section 80C annual limit (₹1,50,000).
pub const LIMIT_80C: i64 = 150_000;

/// Section 80D limit: self + family (below 60 years).
pub const LIMIT_80D_SELF: i64 = 25_000;

/// Section 80D limit: parents (below 60 years).
pub const LIMIT_80D_PARENTS: i64 = 25_000;

/// Section 80D limit: senior citizen self + family.
pub const LIMIT_80D_SELF_SENIOR: i64 = 50_000;

/// Section 80D limit: senior citizen parents.
pub const LIMIT_80D_PARENTS_SENIOR: i64 = 50_000;
```

#### `income_tax.rs`

```rust
//! Income tax slab calculation for India.
//!
//! TODO (Phase 5): Implement tax slab computation for both Old and New regimes.

/// New regime tax slabs (FY2024-25 onwards).
/// (income_limit_exclusive, tax_rate)
pub const NEW_REGIME_SLABS: &[(i64, f64)] = &[
    (300_000, 0.0),
    (700_000, 0.05),
    (1_000_000, 0.10),
    (1_200_000, 0.15),
    (1_500_000, 0.20),
    (i64::MAX, 0.30),
];

/// Surcharge threshold amounts.
pub const SURCHARGE_THRESHOLD_10PCT: i64 = 5_000_000;
pub const SURCHARGE_THRESHOLD_15PCT: i64 = 10_000_000;
```

### Register in `lib.rs`

Find `crates/core/src/lib.rs` and add:
```rust
pub mod tax;
```

Follow the pattern of existing module declarations.

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -20
```

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/tax/ crates/core/src/lib.rs
git commit -m "feat(tax): add tax module skeleton with models and Indian tax constants"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-7-report.md`

Return:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: N/A (skeleton only, no logic to test)
- Concerns (if any)
