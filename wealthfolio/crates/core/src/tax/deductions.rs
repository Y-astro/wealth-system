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
