//! Tax calculation module for Indian income tax law.
//!
//! This module provides:
//! - Capital gains calculations (STCG/LTCG) per Indian tax rules
//! - Section 80C/80D deduction tracking
//! - Interest and dividend income aggregation
//! - ITR-compatible report generation

pub mod capital_gains;
pub mod deductions;
pub mod income_tax;
pub mod models;
pub mod service;

pub use capital_gains::{
    calculate_grandfathered_cost, classify_holding_period, compute_capital_gains, CapitalGainLotSale,
};
pub use deductions::{compute_deductions, DeductionInputs};
pub use income_tax::{calculate_income_tax, TaxSlabResult};
pub use models::{
    CapitalGainsSummary, CapitalGainType, DeductionSummary, TaxRegime, TaxReport,
};
pub use service::TaxReportService;
