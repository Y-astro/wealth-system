//! Capital gains calculation for Indian tax law.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::assets::AssetKind;
use crate::tax::models::{CapitalGainType, CapitalGainsSummary};

/// Financial year boundary: April 1 to March 31 (India).
pub const INDIA_FY_START_MONTH: u32 = 4;

/// Annual LTCG exemption for equity (₹1,25,000 from FY2024-25).
pub const EQUITY_LTCG_EXEMPTION: i64 = 125_000;

/// STCG tax rate on equity (20% from FY2024-25).
pub const EQUITY_STCG_RATE: f64 = 0.20;

/// LTCG tax rate on equity (12.5% from FY2024-25).
pub const EQUITY_LTCG_RATE: f64 = 0.125;

/// Represents a matched sale lot for capital gains calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CapitalGainLotSale {
    pub buy_date: NaiveDate,
    pub sell_date: NaiveDate,
    #[ts(type = "number")]
    pub buy_price: Decimal,
    #[ts(type = "number")]
    pub sell_price: Decimal,
    #[ts(type = "number")]
    pub quantity: Decimal,
    pub asset_kind: AssetKind,
    #[ts(type = "number | null")]
    pub fmv_31_jan_2018: Option<Decimal>,
}

/// Calculates the grandfathered cost of acquisition for pre-2018 equity holdings under Section 55(2)(ac).
///
/// Formula:
/// If `fmv_31_jan_2018` is provided: `actual_cost.max(fmv_31_jan_2018.min(sell_price))`
/// Else: `actual_cost`
pub fn calculate_grandfathered_cost(
    actual_cost: Decimal,
    fmv_31_jan_2018: Option<Decimal>,
    sell_price: Decimal,
) -> Decimal {
    if let Some(fmv) = fmv_31_jan_2018 {
        actual_cost.max(fmv.min(sell_price))
    } else {
        actual_cost
    }
}

/// Classifies the holding period of an asset into STCG or LTCG per Indian tax rules.
///
/// Rules:
/// - Equity (`AssetKind::Investment`): < 365 days -> STCG, >= 365 days -> LTCG.
/// - Property (`AssetKind::Property`): < 730 days (2 years) -> STCG, >= 730 days -> LTCG.
/// - Gold/PreciousMetal (`AssetKind::PreciousMetal`): < 730 days (2 years) -> STCG, >= 730 days -> LTCG.
/// - Debt MF (`AssetKind::DebtMf`): Always classified as STCG (post-2023 Finance Act rule).
/// - Other assets: < 730 days -> STCG, >= 730 days -> LTCG.
pub fn classify_holding_period(
    buy_date: NaiveDate,
    sell_date: NaiveDate,
    asset_kind: AssetKind,
) -> CapitalGainType {
    if asset_kind == AssetKind::DebtMf {
        return CapitalGainType::Stcg;
    }

    let days = (sell_date - buy_date).num_days();

    match asset_kind {
        AssetKind::Investment => {
            if days < 365 {
                CapitalGainType::Stcg
            } else {
                CapitalGainType::Ltcg
            }
        }
        AssetKind::Property | AssetKind::PreciousMetal => {
            if days < 730 {
                CapitalGainType::Stcg
            } else {
                CapitalGainType::Ltcg
            }
        }
        _ => {
            if days < 730 {
                CapitalGainType::Stcg
            } else {
                CapitalGainType::Ltcg
            }
        }
    }
}

/// Computes the capital gains summary for a list of sale lots in a financial year.
///
/// Calculates:
/// - Equity STCG & LTCG (split into exempt up to ₹1,25,000 and taxable beyond ₹1.25L)
/// - Debt MF gains
/// - Property STCG & LTCG
/// - Gold STCG & LTCG
/// - Total STCG, Total LTCG
/// - Estimated STCG tax (20% on equity STCG)
/// - Estimated LTCG tax (12.5% on taxable equity LTCG, gold LTCG, and property LTCG)
pub fn compute_capital_gains(
    sales: &[CapitalGainLotSale],
    financial_year: &str,
) -> CapitalGainsSummary {
    let mut equity_stcg = Decimal::ZERO;
    let mut raw_equity_ltcg = Decimal::ZERO;
    let mut debt_mf_gains = Decimal::ZERO;
    let mut property_stcg = Decimal::ZERO;
    let mut property_ltcg = Decimal::ZERO;
    let mut gold_stcg = Decimal::ZERO;
    let mut gold_ltcg = Decimal::ZERO;

    for sale in sales {
        let cost_per_unit = calculate_grandfathered_cost(
            sale.buy_price,
            sale.fmv_31_jan_2018,
            sale.sell_price,
        );
        let gain = (sale.sell_price - cost_per_unit) * sale.quantity;
        let gain_type = classify_holding_period(sale.buy_date, sale.sell_date, sale.asset_kind);

        match sale.asset_kind {
            AssetKind::Investment => match gain_type {
                CapitalGainType::Stcg => equity_stcg += gain,
                CapitalGainType::Ltcg => raw_equity_ltcg += gain,
            },
            AssetKind::DebtMf => {
                debt_mf_gains += gain;
            }
            AssetKind::Property => match gain_type {
                CapitalGainType::Stcg => property_stcg += gain,
                CapitalGainType::Ltcg => property_ltcg += gain,
            },
            AssetKind::PreciousMetal => match gain_type {
                CapitalGainType::Stcg => gold_stcg += gain,
                CapitalGainType::Ltcg => gold_ltcg += gain,
            },
            _ => match gain_type {
                CapitalGainType::Stcg => property_stcg += gain,
                CapitalGainType::Ltcg => property_ltcg += gain,
            },
        }
    }

    let exemption_limit = Decimal::from(EQUITY_LTCG_EXEMPTION);
    let (equity_ltcg_exempt, equity_ltcg) = if raw_equity_ltcg > Decimal::ZERO {
        let exempt = raw_equity_ltcg.min(exemption_limit);
        let taxable = raw_equity_ltcg - exempt;
        (exempt, taxable)
    } else {
        (Decimal::ZERO, raw_equity_ltcg)
    };

    let total_stcg = equity_stcg + debt_mf_gains + property_stcg + gold_stcg;
    let total_ltcg = (equity_ltcg + equity_ltcg_exempt) + property_ltcg + gold_ltcg;

    // Tax rates
    let stcg_rate = Decimal::new(20, 2);  // 0.20 (20%)
    let ltcg_rate = Decimal::new(125, 3); // 0.125 (12.5%)

    let estimated_tax_stcg = equity_stcg.max(Decimal::ZERO) * stcg_rate;

    let taxable_ltcg_base = equity_ltcg.max(Decimal::ZERO)
        + property_ltcg.max(Decimal::ZERO)
        + gold_ltcg.max(Decimal::ZERO);
    let estimated_tax_ltcg = taxable_ltcg_base * ltcg_rate;

    CapitalGainsSummary {
        financial_year: financial_year.to_string(),
        equity_stcg,
        equity_ltcg,
        equity_ltcg_exempt,
        debt_mf_gains,
        property_stcg,
        property_ltcg,
        gold_stcg,
        gold_ltcg,
        total_stcg,
        total_ltcg,
        estimated_tax_stcg,
        estimated_tax_ltcg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rust_decimal_macros::dec;

    #[test]
    fn test_equity_holding_period_boundary() {
        let buy_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        // 364 days -> STCG
        let sell_date_364 = buy_date + Duration::days(364);
        assert_eq!(
            classify_holding_period(buy_date, sell_date_364, AssetKind::Investment),
            CapitalGainType::Stcg
        );

        // 365 days -> LTCG
        let sell_date_365 = buy_date + Duration::days(365);
        assert_eq!(
            classify_holding_period(buy_date, sell_date_365, AssetKind::Investment),
            CapitalGainType::Ltcg
        );
    }

    #[test]
    fn test_property_and_gold_holding_period_boundary() {
        let buy_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();

        // 729 days -> STCG for property & gold
        let sell_date_729 = buy_date + Duration::days(729);
        assert_eq!(
            classify_holding_period(buy_date, sell_date_729, AssetKind::Property),
            CapitalGainType::Stcg
        );
        assert_eq!(
            classify_holding_period(buy_date, sell_date_729, AssetKind::PreciousMetal),
            CapitalGainType::Stcg
        );

        // 730 days -> LTCG for property & gold
        let sell_date_730 = buy_date + Duration::days(730);
        assert_eq!(
            classify_holding_period(buy_date, sell_date_730, AssetKind::Property),
            CapitalGainType::Ltcg
        );
        assert_eq!(
            classify_holding_period(buy_date, sell_date_730, AssetKind::PreciousMetal),
            CapitalGainType::Ltcg
        );
    }

    #[test]
    fn test_debt_mf_holding_period_always_stcg() {
        let buy_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        // Short holding period
        let sell_short = buy_date + Duration::days(100);
        assert_eq!(
            classify_holding_period(buy_date, sell_short, AssetKind::DebtMf),
            CapitalGainType::Stcg
        );

        // Long holding period (> 3 years)
        let sell_long = buy_date + Duration::days(1200);
        assert_eq!(
            classify_holding_period(buy_date, sell_long, AssetKind::DebtMf),
            CapitalGainType::Stcg
        );
    }

    #[test]
    fn test_pre_2018_grandfathering_scenarios() {
        // Scenario 1: Stock bought at 100, FMV on 31 Jan 2018 = 150, sold at 200
        // Cost = max(100, min(150, 200)) = 150
        assert_eq!(
            calculate_grandfathered_cost(dec!(100), Some(dec!(150)), dec!(200)),
            dec!(150)
        );

        // Scenario 2: Stock bought at 100, FMV = 200, sold at 150 (sold below FMV but above cost)
        // Cost = max(100, min(200, 150)) = 150
        assert_eq!(
            calculate_grandfathered_cost(dec!(100), Some(dec!(200)), dec!(150)),
            dec!(150)
        );

        // Scenario 3: Stock bought at 100, FMV = 150, sold at 80 (sold below cost)
        // Cost = max(100, min(150, 80)) = 100
        assert_eq!(
            calculate_grandfathered_cost(dec!(100), Some(dec!(150)), dec!(80)),
            dec!(100)
        );

        // Scenario 4: Stock bought at 200, FMV = 150, sold at 250 (bought above FMV)
        // Cost = max(200, min(150, 250)) = 200
        assert_eq!(
            calculate_grandfathered_cost(dec!(200), Some(dec!(150)), dec!(250)),
            dec!(200)
        );

        // Scenario 5: Bought post-2018 (FMV is None)
        assert_eq!(
            calculate_grandfathered_cost(dec!(100), None, dec!(200)),
            dec!(100)
        );
    }

    #[test]
    fn test_equity_ltcg_exemption_cap_below_125k() {
        let buy_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let sell_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(); // > 365 days

        // Equity LTCG = 80,000 (below ₹1,25,000 limit)
        let sales = vec![CapitalGainLotSale {
            buy_date,
            sell_date,
            buy_price: dec!(100),
            sell_price: dec!(180),
            quantity: dec!(1000), // Gain = 80 * 1000 = 80,000
            asset_kind: AssetKind::Investment,
            fmv_31_jan_2018: None,
        }];

        let summary = compute_capital_gains(&sales, "FY2024-25");

        assert_eq!(summary.equity_stcg, dec!(0));
        assert_eq!(summary.equity_ltcg, dec!(0)); // Taxable LTCG
        assert_eq!(summary.equity_ltcg_exempt, dec!(80000)); // Exempt LTCG
        assert_eq!(summary.total_ltcg, dec!(80000));
        assert_eq!(summary.estimated_tax_ltcg, dec!(0));
    }

    #[test]
    fn test_equity_ltcg_exemption_cap_above_125k() {
        let buy_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let sell_date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(); // > 365 days

        // Equity LTCG = 2,00,000 (above ₹1,25,000 limit)
        let sales = vec![CapitalGainLotSale {
            buy_date,
            sell_date,
            buy_price: dec!(100),
            sell_price: dec!(300),
            quantity: dec!(1000), // Gain = 200 * 1000 = 2,00,000
            asset_kind: AssetKind::Investment,
            fmv_31_jan_2018: None,
        }];

        let summary = compute_capital_gains(&sales, "FY2024-25");

        assert_eq!(summary.equity_stcg, dec!(0));
        assert_eq!(summary.equity_ltcg_exempt, dec!(125000));
        assert_eq!(summary.equity_ltcg, dec!(75000)); // Taxable = 2,00,000 - 1,25,000
        assert_eq!(summary.total_ltcg, dec!(200000));
        // Estimated LTCG tax: 12.5% of 75,000 = 9,375
        assert_eq!(summary.estimated_tax_ltcg, dec!(9375));
    }

    #[test]
    fn test_debt_mf_gains_calculation() {
        let buy_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let sell_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // 4 years

        let sales = vec![CapitalGainLotSale {
            buy_date,
            sell_date,
            buy_price: dec!(50),
            sell_price: dec!(70),
            quantity: dec!(500), // Gain = 20 * 500 = 10,000
            asset_kind: AssetKind::DebtMf,
            fmv_31_jan_2018: None,
        }];

        let summary = compute_capital_gains(&sales, "FY2024-25");

        assert_eq!(summary.debt_mf_gains, dec!(10000));
        assert_eq!(summary.total_stcg, dec!(10000));
        assert_eq!(summary.total_ltcg, dec!(0));
    }

    #[test]
    fn test_property_and_gold_gains_and_tax_estimates() {
        let buy_date = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();

        // Property sold after > 730 days -> LTCG gain = 5,00,000
        let sell_prop = buy_date + Duration::days(800);
        // Gold sold after < 730 days -> STCG gain = 50,000
        let sell_gold = buy_date + Duration::days(500);

        let sales = vec![
            CapitalGainLotSale {
                buy_date,
                sell_date: sell_prop,
                buy_price: dec!(5000000),
                sell_price: dec!(5500000),
                quantity: dec!(1),
                asset_kind: AssetKind::Property,
                fmv_31_jan_2018: None,
            },
            CapitalGainLotSale {
                buy_date,
                sell_date: sell_gold,
                buy_price: dec!(50000),
                sell_price: dec!(100000),
                quantity: dec!(1),
                asset_kind: AssetKind::PreciousMetal,
                fmv_31_jan_2018: None,
            },
        ];

        let summary = compute_capital_gains(&sales, "FY2024-25");

        assert_eq!(summary.property_stcg, dec!(0));
        assert_eq!(summary.property_ltcg, dec!(500000));
        assert_eq!(summary.gold_stcg, dec!(50000));
        assert_eq!(summary.gold_ltcg, dec!(0));
        assert_eq!(summary.total_stcg, dec!(50000));
        assert_eq!(summary.total_ltcg, dec!(500000));

        // Estimated LTCG tax: 12.5% of property LTCG (5,00,000) = 62,500
        assert_eq!(summary.estimated_tax_ltcg, dec!(62500));
    }
}
