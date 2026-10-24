//! Section 80C, 80D, and other deduction tracking.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::models::DeductionSummary;

/// Section 80C annual limit (₹1,50,000).
pub const LIMIT_80C: i64 = 150_000;

/// Section 80D limit: self + family (below 60 years).
pub const LIMIT_80D_SELF: i64 = 25_000;

/// Section 80D limit: parents (below 60 years).
pub const LIMIT_80D_PARENTS: i64 = 25_000;

/// Section 80D limit: senior citizen self + family.
pub const LIMIT_80D_SELF_SENIOR: i64 = 50_000;

/// Section 80D limit: senior citizen parents.
pub const LIMIT_80D_PARENTS_SENIOR: i64 = 50_000;

/// Inputs for calculating deductions under Chapter VI-A.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeductionInputs {
    pub elss_mf: Decimal,
    pub ppf_contribution: Decimal,
    pub epf_contribution: Decimal,
    pub tax_saver_fd: Decimal,
    pub life_insurance_premium: Decimal,
    pub home_loan_principal: Decimal,
    pub health_insurance_self: Decimal,
    pub health_insurance_parents: Decimal,
    pub is_self_senior: bool,
    pub is_parents_senior: bool,
}

/// Computes Section 80C and Section 80D deductions for a given financial year.
pub fn compute_deductions(inputs: &DeductionInputs, financial_year: &str) -> DeductionSummary {
    let limit_80c = Decimal::from(LIMIT_80C);
    let uncapped_80c = inputs.elss_mf
        + inputs.ppf_contribution
        + inputs.epf_contribution
        + inputs.tax_saver_fd
        + inputs.life_insurance_premium
        + inputs.home_loan_principal;
    let total_80c = uncapped_80c.min(limit_80c);

    let limit_80d_self = if inputs.is_self_senior {
        Decimal::from(LIMIT_80D_SELF_SENIOR)
    } else {
        Decimal::from(LIMIT_80D_SELF)
    };

    let limit_80d_parents = if inputs.is_parents_senior {
        Decimal::from(LIMIT_80D_PARENTS_SENIOR)
    } else {
        Decimal::from(LIMIT_80D_PARENTS)
    };

    let total_80d_self = inputs.health_insurance_self.min(limit_80d_self);
    let total_80d_parents = inputs.health_insurance_parents.min(limit_80d_parents);
    let total_80d = total_80d_self + total_80d_parents;

    let total_deductions = total_80c + total_80d;

    DeductionSummary {
        financial_year: financial_year.to_string(),
        elss_mf: inputs.elss_mf,
        ppf_contribution: inputs.ppf_contribution,
        epf_contribution: inputs.epf_contribution,
        tax_saver_fd: inputs.tax_saver_fd,
        life_insurance_premium: inputs.life_insurance_premium,
        home_loan_principal: inputs.home_loan_principal,
        total_80c,
        limit_80c,
        health_insurance_self: inputs.health_insurance_self,
        health_insurance_parents: inputs.health_insurance_parents,
        total_80d,
        total_deductions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_compute_deductions_80c_cap() {
        let inputs = DeductionInputs {
            elss_mf: dec!(50000),
            ppf_contribution: dec!(70000),
            epf_contribution: dec!(50000), // Total 80C = 170,000 (> 150,000 cap)
            tax_saver_fd: dec!(0),
            life_insurance_premium: dec!(0),
            home_loan_principal: dec!(0),
            health_insurance_self: dec!(20000),
            health_insurance_parents: dec!(25000),
            is_self_senior: false,
            is_parents_senior: false,
        };

        let summary = compute_deductions(&inputs, "FY2025-26");

        assert_eq!(summary.financial_year, "FY2025-26");
        assert_eq!(summary.total_80c, dec!(150000));
        assert_eq!(summary.limit_80c, dec!(150000));
        assert_eq!(summary.total_80d, dec!(45000));
        assert_eq!(summary.total_deductions, dec!(195000));
    }

    #[test]
    fn test_compute_deductions_80d_senior_citizens() {
        let inputs = DeductionInputs {
            elss_mf: dec!(100000),
            ppf_contribution: dec!(0),
            epf_contribution: dec!(0),
            tax_saver_fd: dec!(0),
            life_insurance_premium: dec!(0),
            home_loan_principal: dec!(0),
            health_insurance_self: dec!(40000), // capped at 50,000 because senior
            health_insurance_parents: dec!(60000), // capped at 50,000 because senior
            is_self_senior: true,
            is_parents_senior: true,
        };

        let summary = compute_deductions(&inputs, "FY2025-26");

        assert_eq!(summary.total_80c, dec!(100000));
        assert_eq!(summary.total_80d, dec!(90000)); // 40,000 + 50,000
        assert_eq!(summary.total_deductions, dec!(190000));
    }
}
