//! Expense categorization service.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use super::categorizer;
use super::models::{
    ExpenseCategory, ExpenseCategoryRule, MatchType, MonthlySpend, SpendingAnalysis,
};

/// Service providing expense categorization, rule generation, and spending analysis.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExpenseCategorizerService;

impl ExpenseCategorizerService {
    /// Creates a new `ExpenseCategorizerService`.
    pub fn new() -> Self {
        Self
    }

    /// Categorize a transaction activity description using the given rules.
    pub fn categorize_activity(description: &str, rules: &[ExpenseCategoryRule]) -> String {
        categorizer::categorize_with_fallback(description, rules, "other")
    }

    /// Aggregates transaction amounts by category ID, calculates transaction counts,
    /// sorts by amount descending, and returns `SpendingAnalysis`.
    pub fn aggregate_monthly_spending(
        activities: &[(NaiveDate, Decimal, String, String)],
        categories: &[ExpenseCategory],
        from_date: NaiveDate,
        to_date: NaiveDate,
        currency: &str,
    ) -> SpendingAnalysis {
        let category_map: HashMap<&str, &ExpenseCategory> =
            categories.iter().map(|c| (c.id.as_str(), c)).collect();

        let mut totals: HashMap<String, (Decimal, i32)> = HashMap::new();

        for (date, amount, category_id, _description) in activities {
            if *date >= from_date && *date <= to_date {
                let entry = totals.entry(category_id.clone()).or_insert((Decimal::ZERO, 0));
                entry.0 += *amount;
                entry.1 += 1;
            }
        }

        let mut breakdown: Vec<MonthlySpend> = totals
            .into_iter()
            .map(|(cat_id, (amount, count))| {
                let category_name = category_map
                    .get(cat_id.as_str())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| cat_id.clone());

                MonthlySpend {
                    category_id: cat_id,
                    category_name,
                    amount,
                    transaction_count: count,
                }
            })
            .collect();

        // Sort breakdown by amount descending
        breakdown.sort_by(|a, b| b.amount.cmp(&a.amount));

        let total_spend: Decimal = breakdown.iter().map(|item| item.amount).sum();

        SpendingAnalysis {
            from_date,
            to_date,
            currency: currency.to_string(),
            total_spend,
            breakdown,
        }
    }

    /// Generates a default `Contains` rule from a user description correction
    /// by cleaning numbers and punctuation to create a clean pattern.
    pub fn suggest_rule_from_correction(description: &str, category_id: &str) -> ExpenseCategoryRule {
        let clean_pattern = clean_description_pattern(description);
        ExpenseCategoryRule {
            id: Uuid::new_v4().to_string(),
            category_id: category_id.to_string(),
            match_pattern: clean_pattern,
            match_type: MatchType::Contains,
            priority: 100,
        }
    }
}

/// Helper function to clean common numbers, punctuation, and transaction noise
/// to create a clean pattern for categorization rules.
fn clean_description_pattern(description: &str) -> String {
    let raw_cleaned: String = description
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect();

    let tokens: Vec<&str> = raw_cleaned
        .split_whitespace()
        .filter(|token| !token.chars().all(|c| c.is_ascii_digit()))
        .collect();

    if tokens.is_empty() {
        description.trim().to_string()
    } else {
        tokens.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expenses::models::{default_categories, ExpenseCategoryRule, MatchType};
    use rust_decimal_macros::dec;

    fn make_rule(pattern: &str, category_id: &str, priority: i32) -> ExpenseCategoryRule {
        ExpenseCategoryRule {
            id: Uuid::new_v4().to_string(),
            category_id: category_id.to_string(),
            match_pattern: pattern.to_string(),
            match_type: MatchType::Contains,
            priority,
        }
    }

    #[test]
    fn test_categorize_activity() {
        let rules = vec![
            make_rule("ZOMATO", "dining", 100),
            make_rule("UBER", "transport", 100),
        ];

        assert_eq!(
            ExpenseCategorizerService::categorize_activity("ZOMATO ORDER #123", &rules),
            "dining"
        );
        assert_eq!(
            ExpenseCategorizerService::categorize_activity("UBER TRIP 456", &rules),
            "transport"
        );
        assert_eq!(
            ExpenseCategorizerService::categorize_activity("NETFLIX SUBSCRIPTION", &rules),
            "other"
        );
    }

    #[test]
    fn test_aggregate_monthly_spending() {
        let categories = default_categories();
        let from_date = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let to_date = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();

        let activities = vec![
            (
                NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
                dec!(150.50),
                "dining".to_string(),
                "Zomato Order 1".to_string(),
            ),
            (
                NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
                dec!(250.00),
                "groceries".to_string(),
                "Supermarket".to_string(),
            ),
            (
                NaiveDate::from_ymd_opt(2026, 7, 15).unwrap(),
                dec!(99.50),
                "dining".to_string(),
                "Zomato Order 2".to_string(),
            ),
            (
                // Out of date range (June)
                NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
                dec!(500.00),
                "groceries".to_string(),
                "June Supermarket".to_string(),
            ),
            (
                // Out of date range (August)
                NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                dec!(300.00),
                "shopping".to_string(),
                "August Mall".to_string(),
            ),
        ];

        let analysis = ExpenseCategorizerService::aggregate_monthly_spending(
            &activities,
            &categories,
            from_date,
            to_date,
            "INR",
        );

        assert_eq!(analysis.from_date, from_date);
        assert_eq!(analysis.to_date, to_date);
        assert_eq!(analysis.currency, "INR");
        assert_eq!(analysis.total_spend, dec!(500.00));
        assert_eq!(analysis.breakdown.len(), 2);

        // Sort order descending by amount: groceries (250.00) == dining (250.00 total: 150.50 + 99.50)
        assert_eq!(analysis.total_spend, dec!(500.00));
        let groceries_item = analysis
            .breakdown
            .iter()
            .find(|item| item.category_id == "groceries")
            .unwrap();
        assert_eq!(groceries_item.category_name, "Groceries");
        assert_eq!(groceries_item.amount, dec!(250.00));
        assert_eq!(groceries_item.transaction_count, 1);

        let dining_item = analysis
            .breakdown
            .iter()
            .find(|item| item.category_id == "dining")
            .unwrap();
        assert_eq!(dining_item.category_name, "Dining & Food");
        assert_eq!(dining_item.amount, dec!(250.00));
        assert_eq!(dining_item.transaction_count, 2);
    }

    #[test]
    fn test_suggest_rule_from_correction() {
        let rule1 = ExpenseCategorizerService::suggest_rule_from_correction(
            "UPI/1234567890/ZOMATO ORDER #999",
            "dining",
        );
        assert_eq!(rule1.category_id, "dining");
        assert_eq!(rule1.match_type, MatchType::Contains);
        assert_eq!(rule1.match_pattern, "UPI ZOMATO ORDER");
        assert_eq!(rule1.priority, 100);

        let rule2 = ExpenseCategorizerService::suggest_rule_from_correction(
            "POS 987654 SWIGGY BANGALORE IN",
            "dining",
        );
        assert_eq!(rule2.match_pattern, "POS SWIGGY BANGALORE IN");

        let rule3 = ExpenseCategorizerService::suggest_rule_from_correction("123456", "other");
        assert_eq!(rule3.match_pattern, "123456");
    }
}
