use diesel::sql_query;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{scoped_futures::ScopedFutureExt, RunQueryDsl};
use diesel_async::{AsyncConnection, AsyncMysqlConnection};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use domain::{models::StatsSnapshot, repositories::PlayerTimedStatsRepository};

use crate::structures_embedded_in_rdb::{
    choose_base_diff_sequence_for_snapshot_with_heuristics, DiffSequence, DiffSequenceChoice,
};
use stats_with_incremental_snapshot_tables::{
    HasIncrementalSnapshotTables, HasIncrementalSnapshotTablesDefaultMethods,
};

mod cycle_free_path;
mod diesel_based_impl;
mod stats_with_incremental_snapshot_tables;
mod structures_embedded_in_rdb;

pub mod config {
    #[derive(Debug, serde::Deserialize, Clone)]
    pub struct Database {
        pub db_connection_url: String,
        pub db_connection_pool_size: usize,
    }

    impl Database {
        pub fn from_env() -> anyhow::Result<Self> {
            Ok(envy::from_env::<Self>()?)
        }
    }
}

pub struct DatabaseConnector {
    pool: Pool<AsyncMysqlConnection>,
}

impl DatabaseConnector {
    pub async fn try_new(config: config::Database) -> anyhow::Result<Self> {
        let connection_manager =
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(config.db_connection_url);

        let pool = Pool::builder(connection_manager)
            .max_size(config.db_connection_pool_size)
            .build()?;

        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl<Stats: HasIncrementalSnapshotTables<Object<AsyncMysqlConnection>> + Send + 'static>
    PlayerTimedStatsRepository<Stats> for DatabaseConnector
{
    async fn record_snapshot(&self, snapshot: StatsSnapshot<Stats>) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        conn.transaction(|conn| {
            async move {
                sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .execute(conn)
                    .await?;

                if let Some(full_snapshot) =
                    Stats::find_latest_full_snapshot_before(snapshot.utc_timestamp, conn).await?
                {
                    let diff_points_over_full_snapshot =
                        Stats::read_diff_snapshot_points_over_full_point(full_snapshot.id, conn)
                            .await?
                            .points_before(snapshot.utc_timestamp);

                    let diff_sequence_choice =
                        choose_base_diff_sequence_for_snapshot_with_heuristics(
                            full_snapshot,
                            diff_points_over_full_snapshot,
                            &snapshot,
                        )?;

                    if let DiffSequenceChoice::OptimalAccordingToHeuristics(diff_sequence) =
                        diff_sequence_choice
                    {
                        Stats::create_diff_snapshot_point_on(diff_sequence, snapshot, conn).await
                    } else {
                        Stats::create_full_snapshot(snapshot, conn).await
                    }
                } else {
                    Stats::create_full_snapshot(snapshot, conn).await
                }
            }
            .scope_boxed()
        })
        .await
    }

    async fn search_snapshot(
        &self,
        condition: TimeBasedSnapshotSearchCondition,
    ) -> anyhow::Result<Option<StatsSnapshot<Stats>>> {
        let mut conn = self.pool.get().await?;
        let diff_sequence_upto_latest_snapshot = conn
            .transaction(|conn| {
                async move {
                    let snapshot_point =
                        Stats::find_snapshot_point_with_condition(condition, conn).await?;

                    if let Some(snapshot_point) = snapshot_point {
                        let sequence =
                            Stats::construct_diff_sequence_leading_up_to(snapshot_point, conn)
                                .await?;

                        anyhow::Ok(Some(sequence))
                    } else {
                        anyhow::Ok(None)
                    }
                }
                .scope_boxed()
            })
            .await?;

        Ok(diff_sequence_upto_latest_snapshot.map(DiffSequence::into_snapshot_at_the_tip))
    }
}