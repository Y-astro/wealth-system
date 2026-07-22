# P1-T8: Create `crates/core/src/expenses/` Module Skeleton

## Context

You are implementing **Phase 1, Task 8** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

This task creates the skeleton for the expense categorization module. Full implementation comes in Phase 5 (expense categorizer, AI-assisted rules engine). Here we establish the module structure and types.

## Your Task

Create the `crates/core/src/expenses/` module with skeleton files.

### Files to create

1. `crates/core/src/expenses/mod.rs`
2. `crates/core/src/expenses/models.rs`
3. `crates/core/src/expenses/categorizer.rs`
4. `crates/core/src/expenses/service.rs`

### Also modify

5. `crates/core/src/lib.rs` — Add `pub mod expenses;`

### Content

#### `mod.rs`

```rust
//! Expense categorization module.
//!
//! Categorizes bank statement transactions (WITHDRAWAL activities) into
//! spending categories like Groceries, Dining, Shopping, EMI, etc.
//!
//! Categories and rules are stored in the database (see DB migration for
//! `expense_categories` and `expense_category_rules` tables — Phase 5).
//!
//! **Phase 1 note**: This is a skeleton. Full implementation in Phase 5.

pub mod categorizer;
pub mod models;
pub mod service;

pub use models::{ExpenseCategory, ExpenseCategoryRule, MatchType, SpendingAnalysis};
```

#### `models.rs`

```rust
//! Expense categorization domain models.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An expense category (e.g., "Groceries", "Dining", "EMI").
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MonthlySpend {
    pub category_id: String,
    pub category_name: String,
    pub amount: Decimal,
    pub transaction_count: i32,
}

/// Spending analysis for a date range.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
```

#### `categorizer.rs`

```rust
//! Rule-based expense categorizer.
//!
//! TODO (Phase 5): Implement:
//! 1. Load rules from database
//! 2. For each transaction, apply rules in priority order (highest first)
//! 3. Return matched category or "other" as fallback
//! 4. Support user corrections that train new rules

use super::models::{ExpenseCategoryRule, MatchType};

/// Applies a single rule to a transaction description.
/// Returns true if the rule matches.
pub fn rule_matches(rule: &ExpenseCategoryRule, description: &str) -> bool {
    let desc_lower = description.to_lowercase();
    let pattern_lower = rule.match_pattern.to_lowercase();
    match rule.match_type {
        MatchType::Contains => desc_lower.contains(&pattern_lower),
        MatchType::Exact => desc_lower == pattern_lower,
        MatchType::StartsWith => desc_lower.starts_with(&pattern_lower),
        MatchType::Regex => {
            // TODO (Phase 5): use regex crate
            // For now, fall back to Contains
            desc_lower.contains(&pattern_lower)
        }
    }
}

/// Categorize a transaction description using a sorted list of rules.
///
/// Rules must be sorted by priority descending (highest first).
/// Returns the matched category_id, or "other" if no rule matches.
pub fn categorize(description: &str, rules: &[ExpenseCategoryRule]) -> String {
    for rule in rules {
        if rule_matches(rule, description) {
            return rule.category_id.clone();
        }
    }
    "other".to_string()
}
```

#### `service.rs`

```rust
//! Expense categorization service.
//!
//! TODO (Phase 5): Implement full service with:
//! - CRUD for expense categories
//! - CRUD for categorization rules
//! - Bulk categorization of existing activities
//! - Spending analysis queries

// Phase 5 placeholder
```

### Register in `lib.rs`

Find `crates/core/src/lib.rs` and add:
```rust
pub mod expenses;
```

**IMPORTANT**: If P1-T7 already added `pub mod tax;` to `lib.rs`, make sure to add `pub mod expenses;` alongside it, not replace it.

### Tests

In `categorizer.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::expenses::models::{ExpenseCategoryRule, MatchType};

    fn make_rule(pattern: &str, match_type: MatchType, category_id: &str, priority: i32) -> ExpenseCategoryRule {
        ExpenseCategoryRule {
            id: "r1".to_string(),
            category_id: category_id.to_string(),
            match_pattern: pattern.to_string(),
            match_type,
            priority,
        }
    }

    #[test]
    fn test_contains_match() {
        let rule = make_rule("ZOMATO", MatchType::Contains, "dining", 100);
        assert!(rule_matches(&rule, "ZOMATO ORDER #12345"));
        assert!(rule_matches(&rule, "zomato payment"));
        assert!(!rule_matches(&rule, "AMAZON SHOPPING"));
    }

    #[test]
    fn test_exact_match() {
        let rule = make_rule("ATM CASH", MatchType::Exact, "atm", 100);
        assert!(rule_matches(&rule, "ATM CASH"));
        assert!(!rule_matches(&rule, "ATM CASH WITHDRAWAL"));
    }

    #[test]
    fn test_starts_with_match() {
        let rule = make_rule("SBI CARD", MatchType::StartsWith, "emi", 100);
        assert!(rule_matches(&rule, "SBI CARD EMI 3456"));
        assert!(!rule_matches(&rule, "PAYMENT TO SBI CARD"));
    }

    #[test]
    fn test_categorize_first_match_wins() {
        let rules = vec![
            make_rule("ZOMATO", MatchType::Contains, "dining", 100),
            make_rule("ZOMATO", MatchType::Contains, "food", 50),
        ];
        assert_eq!(categorize("ZOMATO ORDER", &rules), "dining");
    }

    #[test]
    fn test_categorize_fallback() {
        let rules = vec![make_rule("ZOMATO", MatchType::Contains, "dining", 100)];
        assert_eq!(categorize("UNKNOWN MERCHANT", &rules), "other");
    }
}
```

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-core 2>&1 | tail -20
```

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/expenses/ crates/core/src/lib.rs
git commit -m "feat(expenses): add expense categorization module skeleton"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-8-report.md`

Return:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
