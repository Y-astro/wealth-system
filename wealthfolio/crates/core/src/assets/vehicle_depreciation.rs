//! Vehicle Depreciation module.

use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

/// Calculate the depreciated value of a vehicle using the reducing balance method.
/// Formula: `value = purchase_price * (1 - depreciation_rate)^years_held`
/// Default depreciation rate: 15% (0.15).
pub fn calculate_vehicle_depreciated_value(
    purchase_price: Decimal,
    purchase_date: NaiveDate,
    as_of_date: NaiveDate,
    annual_depreciation_pct: Option<f64>,
) -> Decimal {
    if as_of_date <= purchase_date {
        return purchase_price;
    }

    let depreciation_rate = match annual_depreciation_pct {
        Some(rate) => {
            if rate > 1.0 {
                rate / 100.0
            } else {
                rate
            }
        }
        None => 0.15,
    };

    let days_held = (as_of_date - purchase_date).num_days();
    let years_held = days_held as f64 / 365.0;

    let price_f64 = purchase_price.to_f64().unwrap_or(0.0);
    let depreciated = price_f64 * (1.0 - depreciation_rate).powf(years_held);

    Decimal::from_f64(depreciated)
        .unwrap_or(purchase_price)
        .round_dp(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_zero_days_held() {
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let price = dec!(100000);
        let val = calculate_vehicle_depreciated_value(price, date, date, None);
        assert_eq!(val, price);
    }

    #[test]
    fn test_as_of_date_before_purchase() {
        let purchase_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let as_of_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let price = dec!(100000);
        let val = calculate_vehicle_depreciated_value(price, purchase_date, as_of_date, None);
        assert_eq!(val, price);
    }

    #[test]
    fn test_default_depreciation_one_year() {
        let purchase_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let as_of_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let price = dec!(100000);
        let val = calculate_vehicle_depreciated_value(price, purchase_date, as_of_date, None);
        // 365 days / 365.0 = 1.0 year -> 100000 * 0.85 = 85000.00
        assert_eq!(val, dec!(85000.00));
    }

    #[test]
    fn test_default_depreciation_two_years() {
        let purchase_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let as_of_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let price = dec!(100000);
        let val = calculate_vehicle_depreciated_value(price, purchase_date, as_of_date, None);
        // 2023-01-01 to 2025-01-01 is 731 days (includes 2024 leap year). 731 / 365.0 = 2.00274 years -> 72217.84
        assert_eq!(val, dec!(72217.84));
    }

    #[test]
    fn test_custom_depreciation_pct() {
        let purchase_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let as_of_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let price = dec!(100000);
        // 20% custom rate
        let val = calculate_vehicle_depreciated_value(price, purchase_date, as_of_date, Some(20.0));
        assert_eq!(val, dec!(80000.00));

        let val_decimal_rate = calculate_vehicle_depreciated_value(price, purchase_date, as_of_date, Some(0.20));
        assert_eq!(val_decimal_rate, dec!(80000.00));
    }
}
