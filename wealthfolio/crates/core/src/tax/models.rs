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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
