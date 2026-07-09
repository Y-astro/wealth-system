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
