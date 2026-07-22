use chrono::{Datelike, NaiveDate};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;

use crate::errors::{Error, Result, ValidationError};

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
    
    NaiveDate::from_ymd_opt(year, month, date.day())
        .unwrap_or_else(|| {
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

pub fn calculate_emi(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
) -> Result<Decimal> {
    if tenure_months == 0 {
        return Err(Error::Validation(ValidationError::InvalidInput(
            "Tenure must be greater than 0".to_string(),
        )));
    }
    if annual_rate_pct <= Decimal::ZERO {
        return Ok((principal / Decimal::from(tenure_months)).round_dp(2));
    }

    let r = annual_rate_pct / Decimal::from(1200);
    let r_f64 = r.to_f64().unwrap_or(0.0);
    let factor = (1.0 + r_f64).powi(tenure_months as i32);
    let p_f64 = principal.to_f64().unwrap_or(0.0);
    
    let emi_f64 = p_f64 * r_f64 * factor / (factor - 1.0);
    Ok(Decimal::from_f64(emi_f64).unwrap_or(Decimal::ZERO).round_dp(2))
}

pub struct AmortizationEntry {
    pub month: u32,
    pub emi: Decimal,
    pub principal_component: Decimal,
    pub interest_component: Decimal,
    pub outstanding_after: Decimal,
    pub date: NaiveDate,
}

pub fn generate_amortization_schedule(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    emi_start_date: NaiveDate,
) -> Result<Vec<AmortizationEntry>> {
    let mut schedule = Vec::with_capacity(tenure_months as usize);
    let emi = calculate_emi(principal, annual_rate_pct, tenure_months)?;
    
    let r = if annual_rate_pct > Decimal::ZERO {
        annual_rate_pct / Decimal::from(1200)
    } else {
        Decimal::ZERO
    };

    let mut outstanding = principal;

    for m in 1..=tenure_months {
        let date = add_months_to_date(emi_start_date, m - 1);
        
        let interest = (outstanding * r).round_dp(2);
        let mut actual_emi = emi;
        let mut principal_comp = actual_emi - interest;

        if m == tenure_months {
            principal_comp = outstanding;
            actual_emi = principal_comp + interest;
        }

        outstanding -= principal_comp;
        if outstanding < Decimal::ZERO {
            outstanding = Decimal::ZERO;
        }

        schedule.push(AmortizationEntry {
            month: m,
            emi: actual_emi,
            principal_component: principal_comp,
            interest_component: interest,
            outstanding_after: outstanding,
            date,
        });
    }

    Ok(schedule)
}

pub fn calculate_outstanding_balance(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    disbursement_date: NaiveDate,
    emi_start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<Decimal> {
    if target_date < disbursement_date {
        return Ok(Decimal::ZERO);
    }
    if target_date < emi_start_date {
        return Ok(principal);
    }

    let schedule = generate_amortization_schedule(principal, annual_rate_pct, tenure_months, emi_start_date)?;
    
    let mut last_outstanding = principal;
    for entry in schedule {
        if target_date >= entry.date {
            last_outstanding = entry.outstanding_after;
        } else {
            break;
        }
    }

    Ok(last_outstanding)
}

pub struct LoanSummary {
    pub emi: Decimal,
    pub total_interest_payable: Decimal,
    pub total_amount_payable: Decimal,
    pub principal_paid_so_far: Decimal,
    pub interest_paid_so_far: Decimal,
    pub outstanding_balance: Decimal,
    pub completion_pct: Decimal,
    pub emis_remaining: u32,
}

pub fn calculate_loan_summary(
    principal: Decimal,
    annual_rate_pct: Decimal,
    tenure_months: u32,
    emi_start_date: NaiveDate,
    target_date: NaiveDate,
) -> Result<LoanSummary> {
    let schedule = generate_amortization_schedule(principal, annual_rate_pct, tenure_months, emi_start_date)?;
    
    let emi = schedule.first().map(|e| e.emi).unwrap_or(Decimal::ZERO);
    let total_interest_payable: Decimal = schedule.iter().map(|e| e.interest_component).sum();
    let total_amount_payable: Decimal = schedule.iter().map(|e| e.emi).sum();
    
    let mut principal_paid_so_far = Decimal::ZERO;
    let mut interest_paid_so_far = Decimal::ZERO;
    let mut outstanding_balance = principal;
    let mut emis_remaining = tenure_months;

    if target_date >= emi_start_date {
        for entry in schedule {
            if target_date >= entry.date {
                principal_paid_so_far += entry.principal_component;
                interest_paid_so_far += entry.interest_component;
                outstanding_balance = entry.outstanding_after;
                emis_remaining -= 1;
            } else {
                break;
            }
        }
    }

    let completion_pct = if principal > Decimal::ZERO {
        (principal_paid_so_far / principal * Decimal::from(100)).round_dp(2)
    } else {
        Decimal::from(100)
    };

    Ok(LoanSummary {
        emi,
        total_interest_payable,
        total_amount_payable,
        principal_paid_so_far,
        interest_paid_so_far,
        outstanding_balance,
        completion_pct,
        emis_remaining,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_emi() {
        let p = Decimal::from(5000000);
        let r = Decimal::from_f64(8.5).unwrap();
        let tenure = 240;

        let emi = calculate_emi(p, r, tenure).unwrap();
        // 50,00,000 at 8.5% for 240 months = 43391
        // Let's verify precision. The exact is 43391.16 or something
        assert_eq!(emi.round_dp(0), Decimal::from(43391));
    }

    #[test]
    fn test_amortization_schedule() {
        let p = Decimal::from(5000000);
        let r = Decimal::from_f64(8.5).unwrap();
        let tenure = 240;
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        let schedule = generate_amortization_schedule(p, r, tenure, start_date).unwrap();
        
        assert_eq!(schedule.len(), 240);
        
        // First entry interest should be 50L * 8.5% / 12 = 35416.67
        let first = &schedule[0];
        assert_eq!(first.interest_component, Decimal::from_f64(35416.67).unwrap());
        assert!(first.interest_component > first.principal_component);
        
        // Last entry outstanding should be 0
        let last = &schedule[239];
        assert_eq!(last.outstanding_after, Decimal::ZERO);
        
        let total_amount: Decimal = schedule.iter().map(|e| e.emi).sum();
        let expected_emi = calculate_emi(p, r, tenure).unwrap();
        // Since the last EMI is adjusted, total amount might not be exactly EMI * 240
        // but close.
        let diff = (total_amount - (expected_emi * Decimal::from(240))).abs();
        assert!(diff < Decimal::from(10));
    }

    #[test]
    fn test_calculate_outstanding_balance() {
        let p = Decimal::from(100000);
        let r = Decimal::from(12);
        let tenure = 12;
        let disb_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let start_date = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();

        // At start (before first EMI)
        let bal_start = calculate_outstanding_balance(p, r, tenure, disb_date, start_date, disb_date).unwrap();
        assert_eq!(bal_start, p);

        // After maturity
        let bal_end = calculate_outstanding_balance(
            p, r, tenure, disb_date, start_date, 
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()
        ).unwrap();
        assert_eq!(bal_end, Decimal::ZERO);
        
        // Midway
        let bal_mid = calculate_outstanding_balance(
            p, r, tenure, disb_date, start_date, 
            NaiveDate::from_ymd_opt(2023, 7, 1).unwrap() // 6 EMIs paid
        ).unwrap();
        assert!(bal_mid > Decimal::ZERO && bal_mid < p);
    }
}
