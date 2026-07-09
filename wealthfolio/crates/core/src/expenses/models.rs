//! Expense categorization domain models.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An expense category (e.g., "Groceries", "Dining", "EMI").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategory {
    pub id: String,
    pub name: String,
    /// Emoji icon, e.g. "🛒"
    pub icon: Option<String>,
    /// Hex color for UI display, e.g. "#FF6B6B"
    pub color: Option<String>,
    /// Parent category ID for hierarchy (e.g., "Dining" under "Food")
    pub parent_id: Option<String>,
    /// True if this is a built-in system category
    pub is_system: bool,
}

/// Pattern matching type for expense rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchType {
    /// Rule matches if description contains the pattern (case-insensitive)
    Contains,
    /// Rule matches if description equals the pattern exactly
    Exact,
    /// Rule matches using regex pattern
    Regex,
    /// Rule matches if description starts with the pattern
    StartsWith,
}

/// An auto-categorization rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategoryRule {
    pub id: String,
    pub category_id: String,
    /// The pattern to match against transaction description
    pub match_pattern: String,
    pub match_type: MatchType,
    /// Higher priority rules are checked first
    pub priority: i32,
}

/// Monthly spending breakdown for analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySpend {
    pub category_id: String,
    pub category_name: String,
    pub amount: Decimal,
    pub transaction_count: i32,
}

/// Spending analysis for a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendingAnalysis {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub currency: String,
    pub total_spend: Decimal,
    pub breakdown: Vec<MonthlySpend>,
}

/// Default system expense categories.
pub fn default_categories() -> Vec<ExpenseCategory> {
    vec![
        cat("groceries", "Groceries", "🛒", "#4CAF50"),
        cat("dining", "Dining & Food", "🍽️", "#FF9800"),
        cat("shopping", "Shopping", "🛍️", "#E91E63"),
        cat("utilities", "Utilities & Bills", "💡", "#2196F3"),
        cat("transport", "Transport", "🚗", "#9C27B0"),
        cat("fuel", "Fuel", "⛽", "#795548"),
        cat("emi", "EMI & Loans", "🏦", "#F44336"),
        cat("insurance", "Insurance", "🛡️", "#607D8B"),
        cat("investment", "Investments", "📈", "#00BCD4"),
        cat("medical", "Medical & Health", "🏥", "#8BC34A"),
        cat("education", "Education", "📚", "#673AB7"),
        cat("entertainment", "Entertainment", "🎬", "#FF5722"),
        cat("travel", "Travel", "✈️", "#03A9F4"),
        cat("atm", "ATM Withdrawal", "🏧", "#9E9E9E"),
        cat("transfer", "Transfers", "💸", "#78909C"),
        cat("other", "Other", "📎", "#BDBDBD"),
    ]
}

fn cat(id: &str, name: &str, icon: &str, color: &str) -> ExpenseCategory {
    ExpenseCategory {
        id: id.to_string(),
        name: name.to_string(),
        icon: Some(icon.to_string()),
        color: Some(color.to_string()),
        parent_id: None,
        is_system: true,
    }
}
