//! SIP Health Check module.
//!
//! Detects active Systematic Investment Plans (SIPs) with missed/overdue installments.

use async_trait::async_trait;

use crate::activities::sip_tracker::{detect_missed_sips, SipSchedule};
use crate::errors::Result;
use crate::health::model::{
    AffectedItem, HealthCategory, HealthIssue, NavigateAction, Severity,
};
use crate::health::traits::{HealthCheck, HealthContext};

/// SIP schedule information with optional asset metadata for health check display.
#[derive(Debug, Clone)]
pub struct SipHealthInfo {
    pub schedule: SipSchedule,
    pub asset_name: Option<String>,
    pub asset_symbol: Option<String>,
}

/// Health check that flags active SIPs with missed or overdue installments.
pub struct SipHealthCheck;

impl SipHealthCheck {
    /// Creates a new SIP health check.
    pub fn new() -> Self {
        Self
    }

    /// Analyzes SIP health info items and returns health issues for missed installments.
    pub fn analyze(
        &self,
        items: &[SipHealthInfo],
        ctx: &HealthContext,
    ) -> Vec<HealthIssue> {
        let mut health_issues = Vec::new();
        let today = ctx.now.date_naive();

        let mut missed = Vec::new();

        for item in items {
            if detect_missed_sips(&item.schedule, today) {
                missed.push(item);
            }
        }

        if !missed.is_empty() {
            let count = missed.len() as u32;
            let record_ids: Vec<String> = missed.iter().map(|i| i.schedule.asset_id.clone()).collect();
            let data_hash = compute_data_hash(&record_ids, Severity::Warning);
            let affected_items: Vec<AffectedItem> = missed
                .iter()
                .map(|i| {
                    let name = i.asset_name.clone().unwrap_or_else(|| i.schedule.asset_id.clone());
                    let symbol = i.asset_symbol.clone().unwrap_or_else(|| name.clone());
                    AffectedItem::asset_with_name(&i.schedule.asset_id, &symbol, Some(name))
                })
                .collect();

            health_issues.push(
                HealthIssue::builder()
                    .id(format!("sip_missed_installment:{}", data_hash))
                    .severity(Severity::Warning)
                    .category(HealthCategory::SipMissedInstallment)
                    .title(if count == 1 {
                        "1 SIP installment missed".to_string()
                    } else {
                        format!("{} SIP installments missed", count)
                    })
                    .message(
                        "One or more active SIP installments are overdue based on the configured schedule.",
                    )
                    .affected_count(count)
                    .affected_items(affected_items)
                    .navigate_action(NavigateAction::to_activities(Some("sip")))
                    .data_hash(data_hash)
                    .build(),
            );
        }

        health_issues
    }

    /// Analyzes a slice of raw `SipSchedule` objects.
    pub fn analyze_schedules(
        &self,
        schedules: &[SipSchedule],
        ctx: &HealthContext,
    ) -> Vec<HealthIssue> {
        let items: Vec<SipHealthInfo> = schedules
            .iter()
            .cloned()
            .map(|schedule| SipHealthInfo {
                schedule,
                asset_name: None,
                asset_symbol: None,
            })
            .collect();

        self.analyze(&items, ctx)
    }
}

impl Default for SipHealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthCheck for SipHealthCheck {
    fn id(&self) -> &'static str {
        "sip_health"
    }

    fn category(&self) -> HealthCategory {
        HealthCategory::SipMissedInstallment
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
    use chrono::{NaiveDate, Utc};
    use rust_decimal_macros::dec;

    #[test]
    fn test_sip_missed_installment_detected() {
        let check = SipHealthCheck::new();
        let now = Utc::now();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let start_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let schedule = SipSchedule {
            asset_id: "MF-001".to_string(),
            mandate_amount: dec!(5000),
            frequency_months: 1,
            start_date,
            last_installment_date: Some(start_date), // next was due 2026-02-01, today is July 2026 -> missed
        };

        let items = vec![SipHealthInfo {
            schedule,
            asset_name: Some("Parag Parikh Flexi Cap Fund".to_string()),
            asset_symbol: Some("PPFCF".to_string()),
        }];

        let issues = check.analyze(&items, &ctx);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Warning);
        assert_eq!(issues[0].category, HealthCategory::SipMissedInstallment);
        assert_eq!(issues[0].affected_count, 1);
        assert!(issues[0].title.contains("SIP installment missed"));
    }

    #[test]
    fn test_sip_uptodate_no_issue() {
        let check = SipHealthCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let schedule = SipSchedule {
            asset_id: "MF-002".to_string(),
            mandate_amount: dec!(10000),
            frequency_months: 1,
            start_date: today,
            last_installment_date: Some(today), // next is due today + 1 month -> not missed
        };

        let issues = check.analyze_schedules(&[schedule], &ctx);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_sip_future_start_date_no_issue() {
        let check = SipHealthCheck::new();
        let now = Utc::now();
        let today = now.date_naive();
        let ctx = HealthContext::with_timestamp(HealthConfig::default(), "INR", 100_000.0, now);

        let schedule = SipSchedule {
            asset_id: "MF-003".to_string(),
            mandate_amount: dec!(2000),
            frequency_months: 1,
            start_date: today + chrono::Duration::days(10),
            last_installment_date: None,
        };

        let issues = check.analyze_schedules(&[schedule], &ctx);
        assert!(issues.is_empty());
    }
}
