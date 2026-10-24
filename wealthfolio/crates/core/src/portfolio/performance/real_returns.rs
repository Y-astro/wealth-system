//! Inflation-adjusted real returns calculator.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Default annual inflation rate for India (6.0%).

pub const DEFAULT_INDIA_INFLATION_RATE: f64 = 0.06;

/// Calculates real return given nominal return and inflation rate.
///
/// Formula: `Real Return = ((1 + nominal_return) / (1 + inflation_rate)) - 1`
pub fn calculate_real_return(nominal_return: Decimal, inflation_rate: Decimal) -> Decimal {
    let one = dec!(1.0);
    let num = one + nominal_return;
    let den = one + inflation_rate;
    if den == Decimal::ZERO {
        return nominal_return;
    }
    (num / den) - one
}

/// Adjusts a future cash flow or portfolio value for inflation to calculate purchasing power in present value terms.
///
/// Formula: `Present Real Value = Nominal Amount / (1 + inflation_rate)^years`
pub fn adjust_amount_for_inflation(amount: Decimal, inflation_rate: Decimal, years: u32) -> Decimal {
    if years == 0 || inflation_rate == Decimal::ZERO {
        return amount;
    }

    let rate_f64 = inflation_rate.to_string().parse::<f64>().unwrap_or(0.06);
    let discount_factor = (1.0 + rate_f64).powi(years as i32);
    if let Some(dec_discount) = Decimal::from_f64_retain(discount_factor) {
        if dec_discount != Decimal::ZERO {
            return amount / dec_discount;
        }
    }
    amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_real_return() {
        let nominal = dec!(0.12); // 12% return
        let inflation = dec!(0.06); // 6% inflation
        let real = calculate_real_return(nominal, inflation);

        // (1.12 / 1.06) - 1 = 0.05660377... (~5.66%)
        assert!(real > dec!(0.056) && real < dec!(0.057));
    }

    #[test]
    fn test_adjust_amount_for_inflation() {
        let nominal_amount = dec!(100000); // 1L
        let inflation = dec!(0.06); // 6%
        let real_5y = adjust_amount_for_inflation(nominal_amount, inflation, 5);

        // 100000 / (1.06^5) = 100000 / 1.338225578 = ~74725.8
        assert!(real_5y > dec!(74000) && real_5y < dec!(75500));
    }
}
