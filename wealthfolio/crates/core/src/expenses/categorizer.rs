//! Rule-based expense categorizer.

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
            match regex::RegexBuilder::new(&rule.match_pattern)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => re.is_match(description),
                Err(_) => false,
            }
        }
    }
}

/// Sorts a slice of rules in-place by priority descending (highest priority first).
pub fn sort_rules_by_priority(rules: &mut [ExpenseCategoryRule]) {
    rules.sort_by(|a, b| b.priority.cmp(&a.priority));
}

/// Categorize a transaction description using a list of rules and a default fallback category.
///
/// Rules are evaluated in priority descending order (highest priority first).
/// Returns the matched category_id, or `default_cat` if no rule matches.
pub fn categorize_with_fallback(
    description: &str,
    rules: &[ExpenseCategoryRule],
    default_cat: &str,
) -> String {
    let mut sorted_rules: Vec<&ExpenseCategoryRule> = rules.iter().collect();
    sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    for rule in sorted_rules {
        if rule_matches(rule, description) {
            return rule.category_id.clone();
        }
    }
    default_cat.to_string()
}

/// Categorize a transaction description using a sorted list of rules.
///
/// Rules are evaluated in priority descending order (highest priority first).
/// Returns the matched category_id, or "other" if no rule matches.
pub fn categorize(description: &str, rules: &[ExpenseCategoryRule]) -> String {
    categorize_with_fallback(description, rules, "other")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expenses::models::{ExpenseCategoryRule, MatchType};

    fn make_rule(
        pattern: &str,
        match_type: MatchType,
        category_id: &str,
        priority: i32,
    ) -> ExpenseCategoryRule {
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
    fn test_regex_match() {
        let rule = make_rule(r"^UPI/.*ZOMATO.*", MatchType::Regex, "dining", 100);
        assert!(rule_matches(&rule, "UPI/123456/ZOMATO/ORDER"));
        assert!(rule_matches(&rule, "upi/98765/zomato_pay"));
        assert!(!rule_matches(&rule, "POS ZOMATO STORE"));
        assert!(!rule_matches(&rule, "UPI/123456/SWIGGY/ORDER"));
    }

    #[test]
    fn test_invalid_regex_handling() {
        let rule = make_rule(r"[unclosed_bracket", MatchType::Regex, "dining", 100);
        assert!(!rule_matches(&rule, "SOME DESCRIPTION"));
    }

    #[test]
    fn test_sort_rules_by_priority() {
        let mut rules = vec![
            make_rule("LOW", MatchType::Contains, "cat_low", 10),
            make_rule("HIGH", MatchType::Contains, "cat_high", 200),
            make_rule("MID", MatchType::Contains, "cat_mid", 50),
        ];

        sort_rules_by_priority(&mut rules);

        assert_eq!(rules[0].priority, 200);
        assert_eq!(rules[1].priority, 50);
        assert_eq!(rules[2].priority, 10);
    }

    #[test]
    fn test_categorize_priority_ordering() {
        // Even if low priority rule appears first in array, high priority matches first
        let rules = vec![
            make_rule("ZOMATO", MatchType::Contains, "food_generic", 10),
            make_rule("ZOMATO", MatchType::Contains, "dining_premium", 100),
        ];

        assert_eq!(categorize("ZOMATO ORDER", &rules), "dining_premium");
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

    #[test]
    fn test_categorize_with_fallback_custom() {
        let rules = vec![make_rule("ZOMATO", MatchType::Contains, "dining", 100)];
        assert_eq!(
            categorize_with_fallback("UNKNOWN MERCHANT", &rules, "unassigned"),
            "unassigned"
        );
        assert_eq!(
            categorize_with_fallback("ZOMATO ORDER", &rules, "unassigned"),
            "dining"
        );
    }
}
