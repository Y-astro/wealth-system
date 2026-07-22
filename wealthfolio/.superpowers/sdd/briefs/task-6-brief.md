# P1-T6: Monthly Net Worth Snapshot Scheduler

## Context

You are implementing **Phase 1, Task 6** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

The `net_worth_snapshots` table (Task 3) and the enriched `NetWorthResponse` (Tasks 4-5) exist. This task creates a service that **writes monthly snapshots** to the `net_worth_snapshots` table.

## Your Task

Create a `NetWorthSnapshotService` that can be called to save the current net worth as a monthly snapshot, and provide a repository trait + implementation for the snapshot storage.

### Files to create/modify

1. **New**: `crates/core/src/portfolio/net_worth/net_worth_snapshot_service.rs` — Service logic
2. **Modify**: `crates/core/src/portfolio/net_worth/mod.rs` — Add module
3. **New**: `crates/storage-sqlite/src/portfolio/net_worth_snapshot_repository.rs` — Storage impl
4. **Modify**: `crates/storage-sqlite/src/portfolio/mod.rs` — Add module

### Exact requirements

#### Core service trait

In `crates/core/src/portfolio/net_worth/net_worth_snapshot_service.rs`:

```rust
//! Monthly net worth snapshot service.

use chrono::NaiveDate;
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
```

#### Storage implementation

In `crates/storage-sqlite/src/portfolio/net_worth_snapshot_repository.rs`:

Implement `NetWorthSnapshotRepositoryTrait` for a struct using Diesel. The `save_snapshot` method should use `INSERT OR REPLACE INTO` (SQLite upsert) keyed on `(snapshot_date, currency)`.

Use `uuid::Uuid::new_v4().to_string()` for the `id` field (check if `uuid` is in Cargo.toml — if not, use `format!("NWS-{}-{}", date, chrono::Utc::now().timestamp_millis())`).

Follow the pattern from other repository implementations in `crates/storage-sqlite/src/portfolio/`.

#### Module registration

- In `crates/core/src/portfolio/net_worth/mod.rs` add: `pub mod net_worth_snapshot_service;`
- In `crates/storage-sqlite/src/portfolio/mod.rs` add: `pub mod net_worth_snapshot_repository;`

### No scheduler daemon required

The "scheduler" in Phase 1 is just the `maybe_record_snapshot` function — called opportunistically when net worth is computed, not on a timer. A true cron-style scheduler can be added in Phase 6. This is sufficient for Phase 1.

### Tests

Write a unit test for `maybe_record_snapshot` using an in-memory mock:

```rust
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
        use chrono::NaiveDate;
        use rust_decimal::Decimal;
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
```

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check 2>&1 | tail -30
```

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/core/src/portfolio/net_worth/ \
        crates/storage-sqlite/src/portfolio/
git commit -m "feat(net-worth): add monthly net worth snapshot service and storage"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-6-report.md`

Return:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: X/Y passing
- Concerns (if any)
