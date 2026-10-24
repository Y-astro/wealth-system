//! Fixed Deposit (FD) and Recurring Deposit (RD) maturity health check.
//!
//! Flags FDs/RDs that have matured or are approaching maturity within 30 days.

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};

use crate::assets::Asset;
use crate::errors::Result;
use crate::health::model::{
    AffectedItem, HealthCategory, HealthIssue, NavigateAction, Severity,
};
use crate::health::traits::{HealthCheck, HealthContext};

/// Asset info for FD/RD maturity analysis.
#[derive(Debug, Clone)]
pub struct FdMaturityAssetInfo {
    pub asset_id: String,
    pub name: String,
    pub symbol: Option<String>,
    pub preset: String,
    pub maturity_date: NaiveDate,
}

/// Health check that detects FDs and RDs near or past maturity.
pub struct FdMaturityCheck;

impl FdMaturityCheck {
    /// Creates a new FD maturity check.
    pub fn new() -> Self {
        Self
    }

    /// Analyzes FD/RD asset items and returns health issues for matured or maturing soon deposits.
    pub fn analyze(
        &self,
        items: &[FdMaturityAssetInfo],
        ctx: &HealthContext,
    ) -> Vec<HealthIssue> {
        let mut health_issues = Vec::new();
        let today = ctx.now.date_naive();
        let thirty_days_later = today + Duration::days(30);

        let mut matured = Vec::new();
        let mut maturing_soon = Vec::new();

        for item in items {
            if item.maturity_date <= today {
                matured.push(item);
            } else if item.maturity_date <= thirty_days_later {
                maturing_soon.push(item);
            }
        }

        if !matured.is_empty() {
            let count = matured.len() as u32;
            let record_ids: Vec<String> = matured.iter().map(|i| i.asset_id.clone()).collect();
            let data_hash = compute_data_hash(&record_ids, Severity::Warning);
            let affected_items: Vec<AffectedItem> = matured
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
                    .id(format!("fd_matured:{}", data_hash))
                    .severity(Severity::Warning)
                    .category(HealthCategory::FdMaturity)
                    .title(if count == 1 {
                        "Fixed/Recurring Deposit has matured".to_string()
                    } else {
                        format!("{} Fixed/Recurring Deposits have matured", count)
                    })
                    .message(
                        "The maturity date for these deposits has passed. Consider updating or reinvesting them.",
                    )
                    .affected_count(count)
                    .affected_items(affected_items)
                    .navigate_action(NavigateAction::to_holdings(None))
                    .data_hash(data_hash)
                    .build(),
            );
        }

        if !maturing_soon.is_empty() {
            let count = maturing_soon.len() as u32;
            let record_ids: Vec<String> = maturing_soon.iter().map(|i| i.asset_id.clone()).collect();
            let data_hash = compute_data_hash(&record_ids, Severity::Info);
            let affected_items: Vec<AffectedItem> = maturing_soon
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
                    .id(format!("fd_maturing_soon:{}", data_hash))
                    .severity(Severity::Info)
                    .category(HealthCategory::FdMaturity)
                    .title(if count == 1 {
                        "Fixed/Recurring Deposit maturing soon".to_string()
                    } else {
                        format!("{} Fixed/Recurring Deposits maturing within 30 days", count)
                    })
                    .message(
                        "These deposits are reaching maturity within the next 30 days.",
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

    /// Analyzes a slice of raw assets by parsing FD/RD metadata.
    pub fn analyze_assets(&self, assets: &[Asset], ctx: &HealthContext) -> Vec<HealthIssue> {
        let mut items = Vec::new();

        for asset in assets {
            if !asset.is_active {
                continue;
            }
            if let Some(metadata) = &asset.metadata {
                if let Some(preset_val) = metadata.get("instrument_preset").and_then(|v| v.as_str()) {
                    let preset_norm = preset_val.to_lowercase();
                    if preset_norm == "fixed_deposit" || preset_norm == "recurring_deposit" || preset_norm == "fd" || preset_norm == "rd" {
                        if let Some(mat_str) = metadata.get("maturity_date").and_then(|v| v.as_str()) {
                            if let Ok(mat_date) = NaiveDate::parse_from_str(mat_str, "%Y-%m-%d") {
                                items.push(FdMaturityAssetInfo {
                                    asset_id: asset.id.clone(),
                                    name: asset.name.clone().unwrap_or_else(|| asset.id.clone()),
                                    symbol: asset.display_code.clone(),
                                    preset: preset_norm,
                                    maturity_date: mat_date,
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

impl Default for FdMaturityCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthCheck for FdMaturityCheck {
    fn id(&self) -> &'static str {
        "fd_maturity"
    }

    fn category(&self) -> HealthCategory {
        HealthCategory::FdMaturity
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
    fn test_fd_matured_issue() {
        let check = FdMaturityCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![FdMaturityAssetInfo {
            asset_id: "FD-001".to_string(),
            name: "HDFC FD".to_string(),
            symbol: Some("HDFC-FD".to_string()),
            preset: "fixed_deposit".to_string(),
            maturity_date: today - Duration::days(5),
        }];

        let issues = check.analyze(&items, &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].category, HealthCategory::FdMaturity);
        assert_eq!(issues[0].affected_count, 1);
        assert!(issues[0].title.contains("matured"));
    }

    #[test]
    fn test_fd_maturing_soon_issue() {
        let check = FdMaturityCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![FdMaturityAssetInfo {
            asset_id: "RD-001".to_string(),
            name: "SBI RD".to_string(),
            symbol: Some("SBI-RD".to_string()),
            preset: "recurring_deposit".to_string(),
            maturity_date: today + Duration::days(15),
        }];

        let issues = check.analyze(&items, &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Info);
        assert_eq!(issues[0].category, HealthCategory::FdMaturity);
        assert_eq!(issues[0].affected_count, 1);
        assert!(issues[0].title.contains("maturing soon"));
    }

    #[test]
    fn test_fd_future_maturity_no_issue() {
        let check = FdMaturityCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let items = vec![FdMaturityAssetInfo {
            asset_id: "FD-002".to_string(),
            name: "ICICI FD".to_string(),
            symbol: None,
            preset: "fixed_deposit".to_string(),
            maturity_date: today + Duration::days(100),
        }];

        let issues = check.analyze(&items, &ctx);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_analyze_assets_from_json() {
        let check = FdMaturityCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let matured_date_str = (today - Duration::days(10)).format("%Y-%m-%d").to_string();
        let asset = Asset {
            id: "FD-100".to_string(),
            name: Some("Axis FD".to_string()),
            metadata: Some(json!({
                "instrument_preset": "fixed_deposit",
                "maturity_date": matured_date_str,
            })),
            is_active: true,
            ..Default::default()
        };

        let issues = check.analyze_assets(&[asset], &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
    }
}
