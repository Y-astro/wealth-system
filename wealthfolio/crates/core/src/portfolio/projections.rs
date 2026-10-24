//! Wealth growth projections engine.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::performance::real_returns::adjust_amount_for_inflation;

/// Default asset class nominal return assumptions for India.
pub const DEFAULT_EQUITY_RETURN: f64 = 0.12;       // 12%
pub const DEFAULT_FIXED_INCOME_RETURN: f64 = 0.07; // 7%
pub const DEFAULT_REAL_ESTATE_RETURN: f64 = 0.08;  // 8%
pub const DEFAULT_GOLD_RETURN: f64 = 0.10;         // 10%
pub const DEFAULT_CASH_OTHER_RETURN: f64 = 0.04;   // 4%

/// Input parameters for wealth growth projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthProjectionInputs {
    pub current_equities: Decimal,
    pub current_fixed_income: Decimal,
    pub current_real_estate: Decimal,
    pub current_gold: Decimal,
    pub current_cash_other: Decimal,
    pub annual_contribution: Decimal,
    /// Annual percentage increase in contribution (e.g., 0.05 for 5%)
    pub annual_contribution_step_up: Decimal,
    /// Inflation rate for real wealth adjustment (e.g., 0.06 for 6%)
    pub inflation_rate: Decimal,
    /// Horizon in years (e.g., 10 or 20)
    pub horizon_years: u32,
}

/// Projected wealth snapshot for a single year.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearlyWealthProjection {
    pub year: u32,
    pub nominal_wealth_base: Decimal,
    pub nominal_wealth_optimistic: Decimal,
    pub nominal_wealth_conservative: Decimal,
    pub real_wealth_base: Decimal,
    pub cumulative_contributions: Decimal,
}

/// Result of multi-year wealth growth projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WealthProjectionResult {
    pub initial_net_worth: Decimal,
    pub horizon_years: u32,
    pub yearly_projections: Vec<YearlyWealthProjection>,
}

/// Computes multi-year wealth growth projections under Base, Optimistic, and Conservative scenarios.
pub fn compute_wealth_projection(inputs: &WealthProjectionInputs) -> WealthProjectionResult {
    let initial_total = inputs.current_equities
        + inputs.current_fixed_income
        + inputs.current_real_estate
        + inputs.current_gold
        + inputs.current_cash_other;

    let eq_weight = if initial_total > Decimal::ZERO { inputs.current_equities / initial_total } else { dec!(0.5) };
    let fi_weight = if initial_total > Decimal::ZERO { inputs.current_fixed_income / initial_total } else { dec!(0.3) };
    let re_weight = if initial_total > Decimal::ZERO { inputs.current_real_estate / initial_total } else { dec!(0.1) };
    let gold_weight = if initial_total > Decimal::ZERO { inputs.current_gold / initial_total } else { dec!(0.05) };
    let cash_weight = if initial_total > Decimal::ZERO { inputs.current_cash_other / initial_total } else { dec!(0.05) };

    let blend_base_rate = (eq_weight * dec!(0.12))
        + (fi_weight * dec!(0.07))
        + (re_weight * dec!(0.08))
        + (gold_weight * dec!(0.10))
        + (cash_weight * dec!(0.04));

    let blend_optimistic_rate = blend_base_rate + dec!(0.02);
    let blend_conservative_rate = (blend_base_rate - dec!(0.02)).max(Decimal::ZERO);

    let mut yearly_projections = Vec::with_capacity(inputs.horizon_years as usize);

    let mut wealth_base = initial_total;
    let mut wealth_opt = initial_total;
    let mut wealth_cons = initial_total;

    let mut cumulative_contrib = Decimal::ZERO;
    let mut current_annual_contrib = inputs.annual_contribution;

    for y in 1..=inputs.horizon_years {
        // Compound existing wealth
        wealth_base = wealth_base * (dec!(1.0) + blend_base_rate) + current_annual_contrib;
        wealth_opt = wealth_opt * (dec!(1.0) + blend_optimistic_rate) + current_annual_contrib;
        wealth_cons = wealth_cons * (dec!(1.0) + blend_conservative_rate) + current_annual_contrib;

        cumulative_contrib += current_annual_contrib;
        current_annual_contrib *= dec!(1.0) + inputs.annual_contribution_step_up;

        let real_wealth_base = adjust_amount_for_inflation(wealth_base, inputs.inflation_rate, y);

        yearly_projections.push(YearlyWealthProjection {
            year: y,
            nominal_wealth_base: wealth_base.round_dp(2),
            nominal_wealth_optimistic: wealth_opt.round_dp(2),
            nominal_wealth_conservative: wealth_cons.round_dp(2),
            real_wealth_base: real_wealth_base.round_dp(2),
            cumulative_contributions: cumulative_contrib.round_dp(2),
        });
    }

    WealthProjectionResult {
        initial_net_worth: initial_total,
        horizon_years: inputs.horizon_years,
        yearly_projections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_wealth_projection() {
        let inputs = WealthProjectionInputs {
            current_equities: dec!(500000),      // 5L
            current_fixed_income: dec!(300000),  // 3L
            current_real_estate: dec!(0),
            current_gold: dec!(100000),         // 1L
            current_cash_other: dec!(100000),    // 1L
            annual_contribution: dec!(120000),   // 1.2L per year (10k/mo)
            annual_contribution_step_up: dec!(0.05), // 5% step up
            inflation_rate: dec!(0.06),          // 6% inflation
            horizon_years: 10,
        };

        let result = compute_wealth_projection(&inputs);

        assert_eq!(result.initial_net_worth, dec!(1000000)); // 10L initial
        assert_eq!(result.horizon_years, 10);
        assert_eq!(result.yearly_projections.len(), 10);

        let year_10 = &result.yearly_projections[9];
        assert_eq!(year_10.year, 10);
        assert!(year_10.nominal_wealth_optimistic > year_10.nominal_wealth_base);
        assert!(year_10.nominal_wealth_base > year_10.nominal_wealth_conservative);
        assert!(year_10.nominal_wealth_base > year_10.real_wealth_base);
        assert!(year_10.cumulative_contributions > dec!(1200000));
    }
}
