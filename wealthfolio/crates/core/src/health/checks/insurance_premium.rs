//! Insurance Premium Due / Renewal health check.
//!
//! Flags insurance policies (Term, Health, ULIP, Endowment) that have overdue premiums
//! or premiums due within 30 days.

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};

use crate::assets::Asset;
use crate::errors::Result;
use crate::health::model::{
    AffectedItem, HealthCategory, HealthIssue, NavigateAction, Severity,
};
use crate::health::traits::{HealthCheck, HealthContext};

/// Insurance policy info for premium due analysis.
#[derive(Debug, Clone)]
pub struct InsurancePolicyInfo {
    pub asset_id: String,
    pub name: String,
    pub symbol: Option<String>,
    pub preset: String,
    pub due_date: NaiveDate,
}

/// Health check that detects insurance policies with overdue or upcoming premium payments.
pub struct InsurancePremiumCheck;

impl InsurancePremiumCheck {
    /// Creates a new insurance premium check.
    pub fn new() -> Self {
        Self
    }

    /// Analyzes insurance policy items and returns health issues for overdue or due-soon premiums.
    pub fn analyze(
        &self,
        items: &[InsurancePolicyInfo],
        ctx: &HealthContext,
    ) -> Vec<HealthIssue> {
        let mut health_issues = Vec::new();
        let today = ctx.now.date_naive();
        let thirty_days_later = today + Duration::days(30);

        let mut overdue = Vec::new();
        let mut due_soon = Vec::new();

        for item in items {
            if item.due_date <= today {
                overdue.push(item);
            } else if item.due_date <= thirty_days_later {
                due_soon.push(item);
            }
        }

        if !overdue.is_empty() {
            let count = overdue.len() as u32;
            let record_ids: Vec<String> = overdue.iter().map(|i| i.asset_id.clone()).collect();
            let data_hash = compute_data_hash(&record_ids, Severity::Warning);
            let affected_items: Vec<AffectedItem> = overdue
                .iter()
                .map(|i| {
                    AffectedItem::asset_with_name(
                        &i.asset_id,
                        i.symbol.as_deref().unwrap_or(&i.name),
                        Some(i.name.clone()),
                    )
                })
                .collect();

            health_issues.push(
                HealthIssue::builder()
                    .id(format!("insurance_overdue:{}", data_hash))
                    .severity(Severity::Warning)
                    .category(HealthCategory::InsurancePremiumDue)
                    .title(if count == 1 {
                        "Insurance premium / renewal overdue".to_string()
                    } else {
                        format!("{} Insurance premiums / renewals overdue", count)
                    })
                    .message(
                        "Insurance policy premium or renewal date has passed. Please pay/renew to maintain coverage.",
                    )
                    .affected_count(count)
                    .affected_items(affected_items)
                    .navigate_action(NavigateAction::to_holdings(None))
                    .data_hash(data_hash)
                    .build(),
            );
        }

        if !due_soon.is_empty() {
            let count = due_soon.len() as u32;
            let record_ids: Vec<String> = due_soon.iter().map(|i| i.asset_id.clone()).collect();
            let data_hash = compute_data_hash(&record_ids, Severity::Info);
            let affected_items: Vec<AffectedItem> = due_soon
                .iter()
                .map(|i| {
                    AffectedItem::asset_with_name(
                        &i.asset_id,
                        i.symbol.as_deref().unwrap_or(&i.name),
                        Some(i.name.clone()),
                    )
                })
                .collect();

            health_issues.push(
                HealthIssue::builder()
                    .id(format!("insurance_due_soon:{}", data_hash))
                    .severity(Severity::Info)
                    .category(HealthCategory::InsurancePremiumDue)
                    .title(if count == 1 {
                        "Insurance premium due soon".to_string()
                    } else {
                        format!("{} Insurance premiums due within 30 days", count)
                    })
                    .message(
                        "Insurance policy premium or renewal is due within the next 30 days.",
                    )
                    .affected_count(count)
                    .affected_items(affected_items)
                    .navigate_action(NavigateAction::to_holdings(None))
                    .data_hash(data_hash)
                    .build(),
            );
        }

        health_issues
    }

    /// Analyzes a slice of raw assets by parsing Insurance metadata.
    pub fn analyze_assets(&self, assets: &[Asset], ctx: &HealthContext) -> Vec<HealthIssue> {
        let mut items = Vec::new();

        for asset in assets {
            if !asset.is_active {
                continue;
            }
            if let Some(metadata) = &asset.metadata {
                if let Some(preset_val) = metadata.get("instrument_preset").and_then(|v| v.as_str()) {
                    let preset_norm = preset_val.to_lowercase();
                    let is_insurance = preset_norm.starts_with("insurance")
                        || preset_norm.contains("term")
                        || preset_norm.contains("health")
                        || preset_norm.contains("ulip")
                        || preset_norm.contains("endowment");

                    if is_insurance {
                        let date_str = metadata
                            .get("premium_due_date")
                            .or_else(|| metadata.get("renewal_date"))
                            .and_then(|v| v.as_str());

                        if let Some(due_str) = date_str {
                            if let Ok(due_date) = NaiveDate::parse_from_str(due_str, "%Y-%m-%d") {
                                items.push(InsurancePolicyInfo {
                                    asset_id: asset.id.clone(),
                                    name: asset.name.clone().unwrap_or_else(|| asset.id.clone()),
                                    symbol: asset.display_code.clone(),
                                    preset: preset_norm,
                                    due_date,
                                });
                            }
                        }
                    }
                }
            }
        }

        self.analyze(&items, ctx)
    }
}

impl Default for InsurancePremiumCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthCheck for InsurancePremiumCheck {
    fn id(&self) -> &'static str {
        "insurance_premium"
    }

    fn category(&self) -> HealthCategory {
        HealthCategory::InsurancePremiumDue
    }

    async fn run(&self, _ctx: &HealthContext) -> Result<Vec<HealthIssue>> {
        Ok(Vec::new())
    }
}

/// Computes a data hash for issue identity and change detection.
fn compute_data_hash(asset_ids: &[String], severity: Severity) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let mut sorted_ids = asset_ids.to_vec();
    sorted_ids.sort();
    for id in &sorted_ids {
        id.hash(&mut hasher);
    }
    severity.as_str().hash(&mut hasher);

    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::model::HealthConfig;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn test_insurance_overdue_issue() {
        let check = InsurancePremiumCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![InsurancePolicyInfo {
            asset_id: "INS-001".to_string(),
            name: "HDFC Life Term".to_string(),
            symbol: Some("TERM-01".to_string()),
            preset: "insurance_term".to_string(),
            due_date: today - Duration::days(3),
        }];

        let issues = check.analyze(&items, &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].category, HealthCategory::InsurancePremiumDue);
        assert_eq!(issues[0].affected_count, 1);
        assert!(issues[0].title.contains("overdue"));
    }

    #[test]
    fn test_insurance_due_soon_issue() {
        let check = InsurancePremiumCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![InsurancePolicyInfo {
            asset_id: "INS-002".to_string(),
            name: "Niva Bupa Health".to_string(),
            symbol: Some("HEALTH-01".to_string()),
            preset: "insurance_health".to_string(),
            due_date: today + Duration::days(10),
        }];

        let issues = check.analyze(&items, &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Info);
        assert_eq!(issues[0].category, HealthCategory::InsurancePremiumDue);
        assert_eq!(issues[0].affected_count, 1);
        assert!(issues[0].title.contains("due soon"));
    }

    #[test]
    fn test_insurance_future_due_no_issue() {
        let check = InsurancePremiumCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![InsurancePolicyInfo {
            asset_id: "INS-003".to_string(),
            name: "LIC Endowment".to_string(),
            symbol: None,
            preset: "insurance_endowment".to_string(),
            due_date: today + Duration::days(180),
        }];

        let issues = check.analyze(&items, &ctx);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_analyze_assets_from_json() {
        let check = InsurancePremiumCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let due_date_str = (today - Duration::days(5)).format("%Y-%m-%d").to_string();
        let asset = Asset {
            id: "INS-100".to_string(),
            name: Some("Star Health Policy".to_string()),
            metadata: Some(json!({
                "instrument_preset": "insurance_health",
                "renewal_date": due_date_str,
            })),
            is_active: true,
            ..Default::default()
        };

        let issues = check.analyze_assets(&[asset], &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
    }
}
