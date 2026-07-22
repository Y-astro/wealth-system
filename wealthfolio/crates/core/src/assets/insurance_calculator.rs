use chrono::NaiveDate;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::assets::InstrumentPreset;

pub struct PremiumStatus {
    pub premium_annual: Option<Decimal>,
    pub premium_due_date: Option<NaiveDate>,
    pub is_premium_overdue: bool,
    pub days_until_due: Option<i64>,
}

/// Helper to parse decimal from JSON value that might be string or number
fn parse_decimal(metadata: &serde_json::Value, field: &str) -> Option<Decimal> {
    metadata.get(field).and_then(|v| {
        if v.is_string() {
            v.as_str().and_then(|s| s.parse().ok())
        } else if v.is_f64() || v.is_i64() || v.is_u64() {
            v.as_f64().and_then(|f| Decimal::from_f64(f))
        } else {
            None
        }
    })
}

/// Helper to parse NaiveDate from JSON value
fn parse_date(metadata: &serde_json::Value, field: &str) -> Option<NaiveDate> {
    metadata.get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}

/// Returns the net-worth-contributing value for an insurance policy.
/// Term and Health insurance return Decimal::ZERO (pure protection).
/// Endowment returns surrender_value from metadata.
/// ULIP returns fund_value from metadata, or falls back to nav * units_held.
pub fn calculate_insurance_value(
    preset: &InstrumentPreset,
    metadata: &serde_json::Value,
) -> Decimal {
    match preset {
        InstrumentPreset::InsuranceTerm | InstrumentPreset::InsuranceHealth => Decimal::ZERO,
        InstrumentPreset::InsuranceEndowment => {
            parse_decimal(metadata, "surrender_value").unwrap_or(Decimal::ZERO)
        }
        InstrumentPreset::InsuranceUlip => {
            if let Some(fund_value) = parse_decimal(metadata, "fund_value") {
                fund_value
            } else {
                let nav = parse_decimal(metadata, "nav").unwrap_or(Decimal::ZERO);
                let units = parse_decimal(metadata, "units_held").unwrap_or(Decimal::ZERO);
                nav * units
            }
        }
        _ => Decimal::ZERO,
    }
}

/// Extract premium status from insurance metadata.
pub fn get_premium_status(metadata: &serde_json::Value, today: NaiveDate) -> PremiumStatus {
    let premium_annual = parse_decimal(metadata, "premium_annual");
    let premium_due_date = parse_date(metadata, "premium_due_date");

    let is_premium_overdue = match premium_due_date {
        Some(date) => date < today,
        None => false,
    };

    let days_until_due = match premium_due_date {
        Some(date) => Some((date - today).num_days()),
        None => None,
    };

    PremiumStatus {
        premium_annual,
        premium_due_date,
        is_premium_overdue,
        days_until_due,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_calculate_insurance_value_term() {
        let metadata = json!({});
        let val = calculate_insurance_value(&InstrumentPreset::InsuranceTerm, &metadata);
        assert_eq!(val, dec!(0));
    }

    #[test]
    fn test_calculate_insurance_value_health() {
        let metadata = json!({});
        let val = calculate_insurance_value(&InstrumentPreset::InsuranceHealth, &metadata);
        assert_eq!(val, dec!(0));
    }

    #[test]
    fn test_calculate_insurance_value_endowment() {
        let metadata = json!({
            "surrender_value": "150000.50"
        });
        let val = calculate_insurance_value(&InstrumentPreset::InsuranceEndowment, &metadata);
        assert_eq!(val, dec!(150000.50));
    }

    #[test]
    fn test_calculate_insurance_value_ulip_fund_value() {
        let metadata = json!({
            "fund_value": 250000.0,
            "nav": 50.0,
            "units_held": 4000.0
        });
        let val = calculate_insurance_value(&InstrumentPreset::InsuranceUlip, &metadata);
        assert_eq!(val, dec!(250000.0));
    }

    #[test]
    fn test_calculate_insurance_value_ulip_fallback() {
        let metadata = json!({
            "nav": "50.0",
            "units_held": 4000.0
        });
        let val = calculate_insurance_value(&InstrumentPreset::InsuranceUlip, &metadata);
        assert_eq!(val, dec!(200000.0));
    }

    #[test]
    fn test_get_premium_status_overdue() {
        let metadata = json!({
            "premium_annual": 12000.0,
            "premium_due_date": "2023-05-01"
        });
        let today = NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
        let status = get_premium_status(&metadata, today);

        assert_eq!(status.premium_annual, Some(dec!(12000.0)));
        assert_eq!(status.premium_due_date, Some(NaiveDate::from_ymd_opt(2023, 5, 1).unwrap()));
        assert!(status.is_premium_overdue);
        assert_eq!(status.days_until_due, Some(-31));
    }

    #[test]
    fn test_get_premium_status_upcoming() {
        let metadata = json!({
            "premium_annual": "24000",
            "premium_due_date": "2023-07-01"
        });
        let today = NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
        let status = get_premium_status(&metadata, today);

        assert_eq!(status.premium_annual, Some(dec!(24000.0)));
        assert_eq!(status.premium_due_date, Some(NaiveDate::from_ymd_opt(2023, 7, 1).unwrap()));
        assert!(!status.is_premium_overdue);
        assert_eq!(status.days_until_due, Some(30));
    }
}
