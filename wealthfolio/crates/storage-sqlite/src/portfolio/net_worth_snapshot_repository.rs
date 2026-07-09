use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;

use super::net_worth_snapshot::NetWorthSnapshot;
use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::net_worth_snapshots;
use crate::schema::net_worth_snapshots::dsl::*;
use wealthfolio_core::errors::Result;
use wealthfolio_core::portfolio::net_worth::{NetWorthSnapshotInput, NetWorthSnapshotRepositoryTrait};

pub struct NetWorthSnapshotRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl NetWorthSnapshotRepository {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>, writer: WriteHandle) -> Self {
        Self { pool, writer }
    }
}

#[async_trait]
impl NetWorthSnapshotRepositoryTrait for NetWorthSnapshotRepository {
    async fn save_snapshot(&self, input: NetWorthSnapshotInput) -> Result<()> {
        let record = NetWorthSnapshot::from(input);

        self.writer
            .exec(move |conn| {
                diesel::replace_into(net_worth_snapshots::table)
                    .values(&record)
                    .execute(conn)
                    .map_err(StorageError::from)?;
                Ok(())
            })
            .await
    }

    async fn load_snapshots(&self, input_currency: &str) -> Result<Vec<NetWorthSnapshotInput>> {
        let mut conn = get_connection(&self.pool)?;

        let db_records = net_worth_snapshots::table
            .filter(currency.eq(input_currency))
            .order(snapshot_date.asc())
            .load::<NetWorthSnapshot>(&mut conn)
            .map_err(StorageError::from)?;

        let domain_records = db_records
            .into_iter()
            .map(NetWorthSnapshotInput::from)
            .collect();

        Ok(domain_records)
    }

    async fn load_latest_snapshot(&self, input_currency: &str) -> Result<Option<NetWorthSnapshotInput>> {
        let mut conn = get_connection(&self.pool)?;

        let db_record = net_worth_snapshots::table
            .filter(currency.eq(input_currency))
            .order(snapshot_date.desc())
            .first::<NetWorthSnapshot>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;

        Ok(db_record.map(NetWorthSnapshotInput::from))
    }
}
