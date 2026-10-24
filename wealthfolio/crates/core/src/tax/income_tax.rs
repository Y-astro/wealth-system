//! Income tax slab calculation for India.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::models::TaxRegime;

/// New regime tax slabs (FY2024-25 / FY2025-26 onwards).
/// (lower_bound, upper_bound, tax_rate)
pub const NEW_REGIME_SLABS: &[(i64, i64, f64)] = &[
    (0, 300_000, 0.0),
    (300_000, 700_000, 0.05),
    (700_000, 1_000_000, 0.10),
    (1_000_000, 1_200_000, 0.15),
    (1_200_000, 1_500_000, 0.20),
    (1_500_000, i64::MAX, 0.30),
];

/// Old regime tax slabs.
/// (lower_bound, upper_bound, tax_rate)
pub const OLD_REGIME_SLABS: &[(i64, i64, f64)] = &[
    (0, 250_000, 0.0),
    (250_000, 500_000, 0.05),
    (500_000, 1_000_000, 0.20),
    (1_000_000, i64::MAX, 0.30),
];

/// Surcharge threshold amounts.
pub const SURCHARGE_THRESHOLD_10PCT: i64 = 5_000_000;
pub const SURCHARGE_THRESHOLD_15PCT: i64 = 10_000_000;

/// Tax computation result for income.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaxSlabResult {
    pub taxable_income: Decimal,
    pub base_tax: Decimal,
    pub rebate_87a: Decimal,
    pub surcharge: Decimal,
    pub cess: Decimal,
    pub total_tax: Decimal,
}

/// Calculate Indian income tax based on regime and net taxable income.
pub fn calculate_income_tax(taxable_income: Decimal, regime: TaxRegime) -> TaxSlabResult {
    if taxable_income <= Decimal::ZERO {
        return TaxSlabResult::default();
    }

    let slabs = match regime {
        TaxRegime::New => NEW_REGIME_SLABS,
        TaxRegime::Old => OLD_REGIME_SLABS,
    };

    let mut base_tax = Decimal::ZERO;
    let income_i64 = taxable_income.to_string().parse::<f64>().unwrap_or(0.0);

    for &(lower, upper, rate) in slabs {
        if lower as f64 >= income_i64 {
            break;
        }
        let upper_limit = if upper == i64::MAX { income_i64 } else { (upper as f64).min(income_i64) };
        let slab_income = (upper_limit - lower as f64).max(0.0);
        let tax_in_slab = slab_income * rate;
        if let Some(dec_tax) = Decimal::from_f64_retain(tax_in_slab) {
            base_tax += dec_tax;
        }
    }

    // Section 87A Rebate
    let rebate_87a = match regime {
        TaxRegime::New => {
            if taxable_income <= dec!(700000) {
                base_tax.min(dec!(25000))
            } else {
                Decimal::ZERO
            }
        }
        TaxRegime::Old => {
            if taxable_income <= dec!(500000) {
                base_tax.min(dec!(12500))
            } else {
                Decimal::ZERO
            }
        }
    };

    let tax_after_rebate = (base_tax - rebate_87a).max(Decimal::ZERO);

    // Surcharge
    let surcharge = if taxable_income > Decimal::from(SURCHARGE_THRESHOLD_15PCT) {
        tax_after_rebate * dec!(0.15)
    } else if taxable_income > Decimal::from(SURCHARGE_THRESHOLD_10PCT) {
        tax_after_rebate * dec!(0.10)
    } else {
        Decimal::ZERO
    };

    // Health & Education Cess (4%)
    let cess = (tax_after_rebate + surcharge) * dec!(0.04);
    let total_tax = tax_after_rebate + surcharge + cess;

    TaxSlabResult {
        taxable_income,
        base_tax,
        rebate_87a,
        surcharge,
        cess,
        total_tax,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_regime_rebate_under_7lakhs() {
        let result = calculate_income_tax(dec!(650000), TaxRegime::New);
        assert_eq!(result.rebate_87a, result.base_tax);
        assert_eq!(result.total_tax, dec!(0));
    }

    #[test]
    fn test_new_regime_tax_12lakhs() {
        let result = calculate_income_tax(dec!(1200000), TaxRegime::New);
        // Slabs:
        // 0-3L: 0
        // 3-7L: 400k * 5% = 20,000
        // 7-10L: 300k * 10% = 30,000
        // 10-12L: 200k * 15% = 30,000
        // Total base tax = 80,000
        assert_eq!(result.base_tax, dec!(80000));
        assert_eq!(result.rebate_87a, dec!(0));
        // Cess: 4% of 80,000 = 3,200
        assert_eq!(result.total_tax, dec!(83200));
    }

    #[test]
    fn test_old_regime_rebate_under_5lakhs() {
        let result = calculate_income_tax(dec!(450000), TaxRegime::Old);
        assert_eq!(result.rebate_87a, result.base_tax);
        assert_eq!(result.total_tax, dec!(0));
    }
}
