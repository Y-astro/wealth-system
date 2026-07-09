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
    pub total_assets: String,
    pub total_liabilities: String,
    pub net_worth: String,
    pub liquid_assets: String,
    pub illiquid_assets: String,
    pub safe_assets: String,
    pub moderate_assets: String,
    pub aggressive_assets: String,
    pub component_breakdown_json: Option<String>,
    pub created_at: NaiveDateTime,
}
