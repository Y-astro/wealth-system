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

use std::str::FromStr;
use wealthfolio_core::constants::DECIMAL_PRECISION;
use wealthfolio_core::portfolio::net_worth::NetWorthSnapshotInput;

impl From<NetWorthSnapshotInput> for NetWorthSnapshot {
    fn from(domain: NetWorthSnapshotInput) -> Self {
        let id = format!("NWS-{}-{}", domain.snapshot_date, domain.currency);
        NetWorthSnapshot {
            id,
            snapshot_date: domain.snapshot_date,
            currency: domain.currency,
            total_assets: domain.total_assets.round_dp(DECIMAL_PRECISION).to_string(),
            total_liabilities: domain.total_liabilities.round_dp(DECIMAL_PRECISION).to_string(),
            net_worth: domain.net_worth.round_dp(DECIMAL_PRECISION).to_string(),
            liquid_assets: domain.liquid_assets.round_dp(DECIMAL_PRECISION).to_string(),
            illiquid_assets: domain.illiquid_assets.round_dp(DECIMAL_PRECISION).to_string(),
            safe_assets: domain.safe_assets.round_dp(DECIMAL_PRECISION).to_string(),
            moderate_assets: domain.moderate_assets.round_dp(DECIMAL_PRECISION).to_string(),
            aggressive_assets: domain.aggressive_assets.round_dp(DECIMAL_PRECISION).to_string(),
            component_breakdown_json: domain.component_breakdown_json,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<NetWorthSnapshot> for NetWorthSnapshotInput {
    fn from(db: NetWorthSnapshot) -> Self {
        NetWorthSnapshotInput {
            snapshot_date: db.snapshot_date,
            currency: db.currency,
            total_assets: Decimal::from_str(&db.total_assets).unwrap_or_default(),
            total_liabilities: Decimal::from_str(&db.total_liabilities).unwrap_or_default(),
            net_worth: Decimal::from_str(&db.net_worth).unwrap_or_default(),
            liquid_assets: Decimal::from_str(&db.liquid_assets).unwrap_or_default(),
            illiquid_assets: Decimal::from_str(&db.illiquid_assets).unwrap_or_default(),
            safe_assets: Decimal::from_str(&db.safe_assets).unwrap_or_default(),
            moderate_assets: Decimal::from_str(&db.moderate_assets).unwrap_or_default(),
            aggressive_assets: Decimal::from_str(&db.aggressive_assets).unwrap_or_default(),
            component_breakdown_json: db.component_breakdown_json,
        }
    }
}

