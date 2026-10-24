pub mod allocation;
pub mod fire;
pub mod holdings;
pub mod income;
pub mod net_worth;
pub mod performance;
pub mod projections;
pub mod snapshot;
pub mod valuation;

pub use projections::{
    compute_wealth_projection, WealthProjectionInputs, WealthProjectionResult, YearlyWealthProjection,
};
