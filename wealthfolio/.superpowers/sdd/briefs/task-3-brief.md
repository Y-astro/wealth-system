# P1-T3: Create `net_worth_snapshots` DB Migration

## Context

You are implementing **Phase 1, Task 3** of a wealth system transformation for Wealthfolio.

**Repo root**: `/home/astro/wealth-system/wealthfolio`
**Working branch**: `main`

The app uses **Diesel ORM with embedded SQLite migrations**. Migrations are in `crates/storage-sqlite/migrations/` and are embedded at compile time via `embed_migrations!()`.

## Your Task

Create a new Diesel migration for the `net_worth_snapshots` table.

### Migration naming convention

Look at existing migration folder names — they follow `YYYY-MM-DD-NNNNNN_description`. Use today's date `2026-07-09` and index `000001`:

Migration dir: `crates/storage-sqlite/migrations/2026-07-09-000001_net_worth_snapshots/`

### Files to create

1. `crates/storage-sqlite/migrations/2026-07-09-000001_net_worth_snapshots/up.sql`
2. `crates/storage-sqlite/migrations/2026-07-09-000001_net_worth_snapshots/down.sql`

### `up.sql` content

```sql
-- Net worth monthly snapshots for the wealth growth timeline.
-- Stores a point-in-time breakdown of wealth across categories.
-- Populated by the monthly snapshot scheduler (Phase 1, Task 6).
CREATE TABLE IF NOT EXISTS net_worth_snapshots (
    id TEXT NOT NULL PRIMARY KEY,
    snapshot_date DATE NOT NULL,
    currency TEXT NOT NULL DEFAULT 'INR',

    -- Asset totals
    total_assets NUMERIC NOT NULL DEFAULT 0,
    total_liabilities NUMERIC NOT NULL DEFAULT 0,
    net_worth NUMERIC NOT NULL DEFAULT 0,

    -- Liquidity split
    liquid_assets NUMERIC NOT NULL DEFAULT 0,
    illiquid_assets NUMERIC NOT NULL DEFAULT 0,

    -- Risk classification
    safe_assets NUMERIC NOT NULL DEFAULT 0,
    moderate_assets NUMERIC NOT NULL DEFAULT 0,
    aggressive_assets NUMERIC NOT NULL DEFAULT 0,

    -- Full component breakdown as JSON
    -- Schema: { "equities": 0, "fixed_income": 0, "real_estate": 0,
    --           "gold": 0, "insurance": 0, "cash": 0, "other": 0,
    --           "home_loans": 0, "car_loans": 0, "credit_cards": 0,
    --           "other_liabilities": 0 }
    component_breakdown_json TEXT,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(snapshot_date, currency)
);

CREATE INDEX idx_net_worth_snapshots_date ON net_worth_snapshots(snapshot_date);
```

### `down.sql` content

```sql
DROP TABLE IF EXISTS net_worth_snapshots;
```

### Schema registration

After creating the migration files, update `crates/storage-sqlite/src/schema.rs` to add the `net_worth_snapshots` table. 

Look at the existing `schema.rs` file for the DSL pattern. Add:

```rust
diesel::table! {
    net_worth_snapshots (id) {
        id -> Text,
        snapshot_date -> Date,
        currency -> Text,
        total_assets -> Numeric,
        total_liabilities -> Numeric,
        net_worth -> Numeric,
        liquid_assets -> Numeric,
        illiquid_assets -> Numeric,
        safe_assets -> Numeric,
        moderate_assets -> Numeric,
        aggressive_assets -> Numeric,
        component_breakdown_json -> Nullable<Text>,
        created_at -> Timestamp,
    }
}
```

### Rust model

Create `crates/storage-sqlite/src/portfolio/net_worth_snapshot.rs` with:

```rust
//! Storage model for net worth snapshots.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::schema::net_worth_snapshots;

/// A stored net worth snapshot record.
#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = net_worth_snapshots)]
pub struct NetWorthSnapshot {
    pub id: String,
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
    pub created_at: NaiveDateTime,
}
```

Then ensure `crates/storage-sqlite/src/portfolio/mod.rs` includes `pub mod net_worth_snapshot;` — check if the file exists and follow the existing `pub mod` pattern.

### Compile check

```bash
cd /home/astro/wealth-system/wealthfolio && cargo check -p wealthfolio-storage-sqlite 2>&1 | tail -30
```

Fix all errors before reporting done.

### Commit

```bash
cd /home/astro/wealth-system/wealthfolio
git add crates/storage-sqlite/migrations/2026-07-09-000001_net_worth_snapshots/ \
        crates/storage-sqlite/src/schema.rs \
        crates/storage-sqlite/src/portfolio/
git commit -m "feat(db): add net_worth_snapshots migration and storage model"
```

## Report Contract

Write your full report to: `/home/astro/wealth-system/wealthfolio/.superpowers/sdd/briefs/task-3-report.md`

Return only:
- Status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
- Commits: short hash(es)
- Test summary: N/A (no unit tests — migration compile check only)
- Concerns (if any)
