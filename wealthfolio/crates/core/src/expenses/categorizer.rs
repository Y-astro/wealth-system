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
