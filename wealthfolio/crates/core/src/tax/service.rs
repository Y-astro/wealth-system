//! Tax report generator service.

use chrono::Local;
use rust_decimal::Decimal;

use super::capital_gains::{compute_capital_gains, CapitalGainLotSale};
use super::deductions::{compute_deductions, DeductionInputs};
use super::income_tax::{calculate_income_tax, TaxSlabResult};
use super::models::{TaxRegime, TaxReport};

/// Service for generating comprehensive tax reports.
#[derive(Debug, Default)]
pub struct TaxReportService;

impl TaxReportService {
    pub fn new() -> Self {
        Self
    }

    /// Generate an ITR-compatible tax report for a financial year.
    pub fn generate_tax_report(
        &self,
        financial_year: &str,
        regime: TaxRegime,
        lot_sales: &[CapitalGainLotSale],
        deduction_inputs: &DeductionInputs,
        dividend_income: Decimal,
        taxable_interest_income: Decimal,
        exempt_interest_income: Decimal,
    ) -> TaxReport {
        let capital_gains = compute_capital_gains(lot_sales, financial_year);
        let deductions = compute_deductions(deduction_inputs, financial_year);

        TaxReport {
            financial_year: financial_year.to_string(),
            regime,
            capital_gains,
            deductions,
            dividend_income,
            taxable_interest_income,
            exempt_interest_income,
            generated_at: Local::now().date_naive(),
            disclaimer: TaxReport::DISCLAIMER.to_string(),
        }
    }

    /// Calculate estimated total tax payable including capital gains, interest, and dividends.
    pub fn estimate_total_tax_payable(&self, report: &TaxReport, other_income: Decimal) -> Decimal {
        let total_ordinary_income = report.dividend_income + report.taxable_interest_income + other_income;
        let taxable_ordinary_income = if report.regime == TaxRegime::Old {
            (total_ordinary_income - report.deductions.total_deductions).max(Decimal::ZERO)
        } else {
            total_ordinary_income
        };

        let ordinary_tax_result: TaxSlabResult = calculate_income_tax(taxable_ordinary_income, report.regime);

        ordinary_tax_result.total_tax
            + report.capital_gains.estimated_tax_stcg
            + report.capital_gains.estimated_tax_ltcg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_generate_tax_report() {
        let service = TaxReportService::new();
        let deduction_inputs = DeductionInputs {
            elss_mf: dec!(50000),
            ppf_contribution: dec!(100000),
            health_insurance_self: dec!(20000),
            ..Default::default()
        };

        let report = service.generate_tax_report(
            "FY2025-26",
            TaxRegime::New,
            &[],
            &deduction_inputs,
            dec!(15000),
            dec!(25000),
            dec!(12000),
        );

        assert_eq!(report.financial_year, "FY2025-26");
        assert_eq!(report.regime, TaxRegime::New);
        assert_eq!(report.dividend_income, dec!(15000));
        assert_eq!(report.taxable_interest_income, dec!(25000));
        assert_eq!(report.exempt_interest_income, dec!(12000));
        assert_eq!(report.deductions.total_80c, dec!(150000));
    }
}
