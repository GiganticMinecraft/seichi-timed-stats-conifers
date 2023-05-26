use diesel::sql_query;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{scoped_futures::ScopedFutureExt, RunQueryDsl};
use diesel_async::{AsyncConnection, AsyncMysqlConnection};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use domain::{models::StatsSnapshot, repositories::PlayerTimedStatsRepository};

mod schema;
mod stats_with_incremental_snapshot_tables;
mod structures_embedded_in_rdb;
mod util;

pub struct DatabaseConnector {
    pool: Pool<AsyncMysqlConnection>,
}

use crate::structures_embedded_in_rdb::DiffSequence;
use stats_with_incremental_snapshot_tables::HasIncrementalSnapshotTables;

#[async_trait::async_trait]
impl<Stats: HasIncrementalSnapshotTables + Clone + Send + 'static> PlayerTimedStatsRepository<Stats>
    for DatabaseConnector
{
    async fn record_snapshot(&self, snapshot: StatsSnapshot<Stats>) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        conn.transaction(|conn| {
            async move {
                sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .execute(conn)
                    .await?;

                let latest_snapshot_point = Stats::find_snapshot_point_with_condition(
                    TimeBasedSnapshotSearchCondition::NewestBefore(snapshot.utc_timestamp),
                    conn,
                )
                .await?;

                match latest_snapshot_point {
                    Some(previous_snapshot_point) => {
                        let diff_sequence = Stats::construct_diff_sequence_leading_up_to(
                            previous_snapshot_point,
                            conn,
                        )
                        .await?;

                        // TODO: create_diff_snapshot_point_on に渡される diff sequence は
                        //       「復元するために必要な diff の総数が少ない」ように、貪欲に選択すると良い。
                        if diff_sequence.is_sufficiently_short_to_extend() {
                            Stats::create_diff_snapshot_point_on(diff_sequence, snapshot, conn)
                                .await
                        } else {
                            Stats::create_full_snapshot(snapshot, conn).await
                        }
                    }
                    None => Stats::create_full_snapshot(snapshot, conn).await,
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

        Ok(diff_sequence_upto_latest_snapshot.map(DiffSequence::into_snapshot_at_the_end))
    }
}
