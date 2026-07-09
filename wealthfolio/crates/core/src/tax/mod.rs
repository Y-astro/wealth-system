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
