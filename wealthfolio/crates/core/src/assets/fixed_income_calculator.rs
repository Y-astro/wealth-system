use chrono::{Datelike, NaiveDate};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::errors::{Error, Result, ValidationError};

/// Calculate compound interest for a Fixed Deposit (FD)
pub fn calculate_fd_value(
    principal: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str, // "monthly"|"quarterly"|"half_yearly"|"yearly"
    start_date: NaiveDate,
    maturity_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal> {
    if annual_rate_pct <= Decimal::ZERO {
        return Err(Error::Validation(ValidationError::InvalidInput(
            "Interest rate must be positive".to_string(),
        )));
    }

    let n = match compounding_frequency {
        "monthly" => Decimal::from(12),
        "quarterly" => Decimal::from(4),
        "half_yearly" => Decimal::from(2),
        "yearly" => Decimal::from(1),
        _ => {
            return Err(Error::Validation(ValidationError::InvalidInput(
                "Invalid compounding frequency".to_string(),
            )))
        }
    };

    if target_date < start_date {
        return Ok(principal);
    }

    let effective_date = if target_date > maturity_date {
        maturity_date
    } else {
        target_date
    };

    let days_elapsed = (effective_date - start_date).num_days();
    let t_years = Decimal::from(days_elapsed) / Decimal::from(365);
    
    // r is decimal rate
    let r = annual_rate_pct / Decimal::from(100);

    // V(t) = P × (1 + r/n)^(n × t_years)
    let base = Decimal::ONE + (r / n);
    let exponent = (n * t_years).to_f64().unwrap_or(0.0);
    
    // using f64 for powf since rust_decimal doesn't have robust powf yet for fractional exponents
    let multiplier = base.to_f64().unwrap_or(1.0).powf(exponent);
    
    let mut value = principal * Decimal::from_f64(multiplier).unwrap_or(Decimal::ONE);
    
    value = value.round_dp(2);
    Ok(value)
}

/// Calculate accrued interest for FD
pub fn calculate_accrued_interest_fd(
    principal: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str,
    start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal> {
    let val = calculate_fd_value(
        principal,
        annual_rate_pct,
        compounding_frequency,
        start_date,
        NaiveDate::MAX, // uncapped
        target_date,
    )?;
    
    Ok(val - principal)
}

/// Calculate Recurring Deposit (RD) value
pub fn calculate_rd_value(
    monthly_installment: Decimal,
    annual_rate_pct: Decimal,
    compounding_frequency: &str,
    start_date: NaiveDate,
    maturity_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal> {
    if annual_rate_pct <= Decimal::ZERO {
        return Err(Error::Validation(ValidationError::InvalidInput(
            "Interest rate must be positive".to_string(),
        )));
    }

    let n = match compounding_frequency {
        "monthly" => Decimal::from(12),
        "quarterly" => Decimal::from(4),
        "half_yearly" => Decimal::from(2),
        "yearly" => Decimal::from(1),
        _ => {
            return Err(Error::Validation(ValidationError::InvalidInput(
                "Invalid compounding frequency".to_string(),
            )))
        }
    };

    if target_date < start_date {
        return Ok(Decimal::ZERO);
    }

    let effective_date = if target_date > maturity_date {
        maturity_date
    } else {
        target_date
    };

    let r = annual_rate_pct / Decimal::from(100);
    
    let mut total_value = Decimal::ZERO;
    let mut i = 0;

    loop {
        let add_months = i as u32;
        let mut year = start_date.year() + (add_months / 12) as i32;
        let mut month = start_date.month() + (add_months % 12);
        if month > 12 {
            year += 1;
            month -= 12;
        }
        
        let installment_date = NaiveDate::from_ymd_opt(year, month, start_date.day())
            .unwrap_or_else(|| {
                let mut d = start_date.day();
                while d > 28 {
                    if let Some(nd) = NaiveDate::from_ymd_opt(year, month, d) {
                        return nd;
                    }
                    d -= 1;
                }
                NaiveDate::from_ymd_opt(year, month, 1).unwrap()
            });

        if installment_date > effective_date || installment_date >= maturity_date {
            break;
        }
        
        let days_remaining = (effective_date - installment_date).num_days();
        let remaining_years = Decimal::from(days_remaining) / Decimal::from_f64(365.25).unwrap();

        let base = Decimal::ONE + (r / n);
        let exponent = (n * remaining_years).to_f64().unwrap_or(0.0);
        let multiplier = base.to_f64().unwrap_or(1.0).powf(exponent);
        
        total_value += monthly_installment * Decimal::from_f64(multiplier).unwrap_or(Decimal::ONE);
        
        i += 1;
        // safety break
        if i > 1200 { break; }
    }

    Ok(total_value.round_dp(2))
}

/// Calculate Public Provident Fund (PPF) value
pub fn calculate_ppf_value(
    contributions: &[(NaiveDate, Decimal)],
    annual_rate_pct: Decimal,
    target_date: NaiveDate,
) -> Result<Decimal> {
    if annual_rate_pct < Decimal::ZERO {
        return Err(Error::Validation(ValidationError::InvalidInput(
            "Interest rate must be non-negative".to_string(),
        )));
    }

    let mut total_value = Decimal::ZERO;
    let r = annual_rate_pct / Decimal::from(100);

    for (date, amount) in contributions {
        if *date > target_date {
            continue;
        }

        let days_elapsed = (target_date - *date).num_days();
        let t_years = Decimal::from(days_elapsed) / Decimal::from_f64(365.25).unwrap();

        let base = Decimal::ONE + r;
        let exponent = t_years.to_f64().unwrap_or(0.0);
        let multiplier = base.to_f64().unwrap_or(1.0).powf(exponent);
        
        total_value += amount * Decimal::from_f64(multiplier).unwrap_or(Decimal::ONE);
    }

    Ok(total_value.round_dp(2))
}

/// Calculate National Pension System (NPS) value
pub fn calculate_nps_value(
    nav: Decimal,
    units_held: Decimal,
) -> Decimal {
    (nav * units_held).round_dp(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_basic() {
        let p = Decimal::from(100000);
        let r = Decimal::from(7); // 7%
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let maturity = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        
        let val = calculate_fd_value(p, r, "quarterly", start, maturity, target).unwrap();
        let interest = val - p;
        // 100000 * (1 + 0.07/4)^(4*1) = 107185.90
        assert_eq!(val, Decimal::from_f64(107185.90).unwrap());
        assert_eq!(interest, Decimal::from_f64(7185.90).unwrap());
    }

    #[test]
    fn test_fd_at_maturity() {
        let p = Decimal::from(100000);
        let r = Decimal::from(7); // 7%
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let maturity = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(); // after maturity
        
        let val = calculate_fd_value(p, r, "quarterly", start, maturity, target).unwrap();
        assert_eq!(val, Decimal::from_f64(107185.90).unwrap());
    }

    #[test]
    fn test_fd_before_start() {
        let p = Decimal::from(100000);
        let r = Decimal::from(7);
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let maturity = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(); // before start
        
        let val = calculate_fd_value(p, r, "quarterly", start, maturity, target).unwrap();
        assert_eq!(val, p);
    }

    #[test]
    fn test_rd_basic() {
        let inst = Decimal::from(10000);
        let r = Decimal::from(7);
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let maturity = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        
        let val = calculate_rd_value(inst, r, "quarterly", start, maturity, target).unwrap();
        // 12 installments, some math
        assert!(val > Decimal::from(120000));
        assert!(val < Decimal::from(130000));
    }

    #[test]
    fn test_rd_partial() {
        let inst = Decimal::from(10000);
        let r = Decimal::from(7);
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let maturity = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2023, 7, 1).unwrap();
        
        let val = calculate_rd_value(inst, r, "quarterly", start, maturity, target).unwrap();
        assert!(val > Decimal::from(70000));
        assert!(val < Decimal::from(75000));
    }

    #[test]
    fn test_invalid_compounding_freq() {
        let err = calculate_fd_value(
            Decimal::from(100), Decimal::from(7), "invalid", 
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_zero_rate() {
        let err = calculate_fd_value(
            Decimal::from(100), Decimal::ZERO, "quarterly", 
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_ppf_basic() {
        let p = Decimal::from(100000);
        let r = Decimal::from_f64(7.1).unwrap();
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // 1 year
        let val = calculate_ppf_value(&[(start, p)], r, target).unwrap();
        assert_eq!(val, Decimal::from_f64(107094.97).unwrap());
    }

    #[test]
    fn test_ppf_multi_year() {
        let r = Decimal::from_f64(7.1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        
        let contributions = vec![
            (d1, Decimal::from(100000)), // 2 years
            (d2, Decimal::from(50000)),  // 1 year
        ];
        
        let val = calculate_ppf_value(&contributions, r, target).unwrap();
        assert_eq!(val, Decimal::from_f64(168240.82).unwrap());
    }

    #[test]
    fn test_ppf_limit_warning_not_enforced_in_math() {
        // Just checking math doesn't panic if limit exceeded
        let p = Decimal::from(200000); // Exceeds 1.5L
        let r = Decimal::from_f64(7.1).unwrap();
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let target = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        
        let val = calculate_ppf_value(&[(start, p)], r, target).unwrap();
        assert_eq!(val, Decimal::from_f64(214189.94).unwrap());
    }

    #[test]
    fn test_nps_calculation() {
        let nav = Decimal::from_f64(25.50).unwrap();
        let units = Decimal::from_f64(100.5).unwrap();
        let val = calculate_nps_value(nav, units);
        // 25.50 * 100.5 = 2562.75
        assert_eq!(val, Decimal::from_f64(2562.75).unwrap());
    }

    #[test]
    fn test_nps_zero_units() {
        let nav = Decimal::from_f64(25.50).unwrap();
        let units = Decimal::ZERO;
        let val = calculate_nps_value(nav, units);
        assert_eq!(val, Decimal::ZERO);
    }
}
