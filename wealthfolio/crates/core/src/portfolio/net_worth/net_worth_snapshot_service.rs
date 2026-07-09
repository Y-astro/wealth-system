//! Monthly net worth snapshot service.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A net worth snapshot to be stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetWorthSnapshotInput {
    pub snapshot_date: NaiveDate,
    pub currency: String,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub net_worth: Decimal,
    pub liquid_assets: Decimal,
    pub illiquid_assets: Decimal,
    pub safe_assets: Decimal,
    pub moderate_assets: Decimal,
    pub aggressive_assets: Decimal,
    pub component_breakdown_json: Option<String>,
}

/// Repository trait for net worth snapshots.
#[async_trait::async_trait]
pub trait NetWorthSnapshotRepositoryTrait: Send + Sync {
    /// Save a snapshot (upsert by snapshot_date + currency).
    async fn save_snapshot(&self, snapshot: NetWorthSnapshotInput) -> crate::errors::Result<()>;

    /// Load all snapshots ordered by date ascending.
    async fn load_snapshots(&self, currency: &str) -> crate::errors::Result<Vec<NetWorthSnapshotInput>>;

    /// Load the most recent snapshot.
    async fn load_latest_snapshot(&self, currency: &str) -> crate::errors::Result<Option<NetWorthSnapshotInput>>;
}

/// Service for recording monthly net worth snapshots.
///
/// Call `maybe_record_snapshot` from the net worth calculation path.
/// It will save a snapshot if none exists for the current month.
pub struct NetWorthSnapshotService {
    pub repository: std::sync::Arc<dyn NetWorthSnapshotRepositoryTrait>,
}

impl NetWorthSnapshotService {
    pub fn new(repository: std::sync::Arc<dyn NetWorthSnapshotRepositoryTrait>) -> Self {
        Self { repository }
    }

    /// Records a snapshot for the given date if the month is not yet recorded.
    pub async fn maybe_record_snapshot(
        &self,
        input: NetWorthSnapshotInput,
    ) -> crate::errors::Result<()> {
        // Check if a snapshot already exists for this month
        let existing = self.repository.load_latest_snapshot(&input.currency).await?;
        let should_save = match existing {
            None => true,
            Some(ref existing) => {
                // Save if the latest snapshot is from a previous month
                existing.snapshot_date.year() < input.snapshot_date.year()
                    || (existing.snapshot_date.year() == input.snapshot_date.year()
                        && existing.snapshot_date.month() < input.snapshot_date.month())
            }
        };

        if should_save {
            self.repository.save_snapshot(input).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockRepo {
        snapshots: Mutex<Vec<NetWorthSnapshotInput>>,
    }

    #[async_trait::async_trait]
    impl NetWorthSnapshotRepositoryTrait for MockRepo {
        async fn save_snapshot(&self, s: NetWorthSnapshotInput) -> crate::errors::Result<()> {
            self.snapshots.lock().unwrap().push(s);
            Ok(())
        }
        async fn load_snapshots(&self, _: &str) -> crate::errors::Result<Vec<NetWorthSnapshotInput>> {
            Ok(self.snapshots.lock().unwrap().clone())
        }
        async fn load_latest_snapshot(&self, currency: &str) -> crate::errors::Result<Option<NetWorthSnapshotInput>> {
            Ok(self.snapshots.lock().unwrap().iter()
                .filter(|s| s.currency == currency)
                .last().cloned())
        }
    }

    fn make_input(year: i32, month: u32) -> NetWorthSnapshotInput {
        NetWorthSnapshotInput {
            snapshot_date: NaiveDate::from_ymd_opt(year, month, 1).unwrap(),
            currency: "INR".to_string(),
            total_assets: Decimal::ZERO,
            total_liabilities: Decimal::ZERO,
            net_worth: Decimal::ZERO,
            liquid_assets: Decimal::ZERO,
            illiquid_assets: Decimal::ZERO,
            safe_assets: Decimal::ZERO,
            moderate_assets: Decimal::ZERO,
            aggressive_assets: Decimal::ZERO,
            component_breakdown_json: None,
        }
    }

    #[tokio::test]
    async fn test_saves_first_snapshot() {
        let repo = std::sync::Arc::new(MockRepo { snapshots: Mutex::new(vec![]) });
        let svc = NetWorthSnapshotService::new(repo.clone());
        svc.maybe_record_snapshot(make_input(2026, 7)).await.unwrap();
        assert_eq!(repo.snapshots.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_does_not_duplicate_same_month() {
        let repo = std::sync::Arc::new(MockRepo { snapshots: Mutex::new(vec![]) });
        let svc = NetWorthSnapshotService::new(repo.clone());
        svc.maybe_record_snapshot(make_input(2026, 7)).await.unwrap();
        svc.maybe_record_snapshot(make_input(2026, 7)).await.unwrap();
        assert_eq!(repo.snapshots.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_saves_new_month() {
        let repo = std::sync::Arc::new(MockRepo { snapshots: Mutex::new(vec![]) });
        let svc = NetWorthSnapshotService::new(repo.clone());
        svc.maybe_record_snapshot(make_input(2026, 6)).await.unwrap();
        svc.maybe_record_snapshot(make_input(2026, 7)).await.unwrap();
        assert_eq!(repo.snapshots.lock().unwrap().len(), 2);
    }
}
