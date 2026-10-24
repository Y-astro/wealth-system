//! SIP Schedule Tracker module.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Represents a Systematic Investment Plan (SIP) schedule.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SipSchedule {
    pub asset_id: String,
    pub mandate_amount: Decimal,
    pub frequency_months: u32,
    pub start_date: NaiveDate,
    pub last_installment_date: Option<NaiveDate>,
}

fn add_months_to_date(date: NaiveDate, months: u32) -> NaiveDate {
    if months == 0 {
        return date;
    }
    let mut year = date.year() + (months / 12) as i32;
    let mut month = date.month() + (months % 12);
    if month > 12 {
        year += 1;
        month -= 12;
    }

    NaiveDate::from_ymd_opt(year, month, date.day()).unwrap_or_else(|| {
        let mut d = date.day();
        while d > 28 {
            if let Some(nd) = NaiveDate::from_ymd_opt(year, month, d) {
                return nd;
            }
            d -= 1;
        }
        NaiveDate::from_ymd_opt(year, month, 1).unwrap()
    })
}

/// Calculate the next SIP installment date.
/// If no last installment exists, returns `start_date`.
/// Otherwise, returns `last_installment + frequency_months`.
pub fn calculate_next_sip_date(
    start_date: NaiveDate,
    frequency_months: u32,
    last_installment: Option<NaiveDate>,
) -> NaiveDate {
    match last_installment {
        None => start_date,
        Some(last_date) => {
            let next_date = add_months_to_date(last_date, frequency_months);
            if next_date < start_date {
                start_date
            } else {
                next_date
            }
        }
    }
}

/// Detects if an SIP installment has been missed relative to `as_of_date`.
/// Returns true if the next SIP date is strictly in the past relative to `as_of_date`.
pub fn detect_missed_sips(schedule: &SipSchedule, as_of_date: NaiveDate) -> bool {
    let next_sip_date = calculate_next_sip_date(
        schedule.start_date,
        schedule.frequency_months,
        schedule.last_installment_date,
    );
    next_sip_date < as_of_date
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_calculate_next_sip_date_no_last_installment() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let next_date = calculate_next_sip_date(start_date, 1, None);
        assert_eq!(next_date, start_date);
    }

    #[test]
    fn test_calculate_next_sip_date_with_last_installment() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let last_installment = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let next_date = calculate_next_sip_date(start_date, 1, Some(last_installment));
        assert_eq!(next_date, NaiveDate::from_ymd_opt(2026, 2, 15).unwrap());
    }

    #[test]
    fn test_calculate_next_sip_date_quarterly() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let last_installment = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let next_date = calculate_next_sip_date(start_date, 3, Some(last_installment));
        assert_eq!(next_date, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_detect_missed_sips() {
        let start_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let schedule = SipSchedule {
            asset_id: "ASSET123".to_string(),
            mandate_amount: dec!(5000),
            frequency_months: 1,
            start_date,
            last_installment_date: Some(start_date),
        };

        // Next SIP date is 2026-02-15
        // as_of_date is 2026-02-10 -> not missed yet
        assert!(!detect_missed_sips(&schedule, NaiveDate::from_ymd_opt(2026, 2, 10).unwrap()));
        // as_of_date is 2026-02-15 -> not missed (it's today)
        assert!(!detect_missed_sips(&schedule, NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()));
        // as_of_date is 2026-02-16 -> missed!
        assert!(detect_missed_sips(&schedule, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap()));
    }
}
