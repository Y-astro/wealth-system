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
