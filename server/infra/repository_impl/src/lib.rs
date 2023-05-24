use diesel::{sql_query, ExpressionMethods};
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::{scoped_futures::ScopedFutureExt, RunQueryDsl};
use diesel_async::{AsyncConnection, AsyncMysqlConnection};
use domain::{
    models::{BreakCount, StatsSnapshot},
    repositories::PlayerTimedStatsRepository,
};

mod schema;

pub struct DatabaseConnector {
    pool: Pool<AsyncMysqlConnection>,
}

async fn set_transaction_isolation_level_serializable(
    conn: &mut Object<AsyncMysqlConnection>,
) -> Result<usize, diesel::result::Error> {
    sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(conn)
        .await
}

use diesel::query_dsl::methods::*;

#[async_trait::async_trait]
impl PlayerTimedStatsRepository<BreakCount> for DatabaseConnector {
    async fn record_snapshot(&self, snapshot: StatsSnapshot<BreakCount>) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;
        conn.transaction(|conn| {
            async move {
                set_transaction_isolation_level_serializable(conn).await?;
                {
                    use schema::break_count_full_snapshot_point::dsl::*;
                    diesel::insert_into(break_count_full_snapshot_point)
                        .values(record_timestamp.eq(snapshot.utc_timestamp.naive_utc()))
                        .execute(conn)
                        .await?;
                }
                let inserted_point_id = {
                    use schema::break_count_full_snapshot_point::dsl::*;
                    break_count_full_snapshot_point
                        .select(id)
                        .order(id.desc())
                        .first::<u64>(conn)
                        .await?
                };
                {
                    use schema::break_count_full_snapshot::dsl::*;
                    for (player, break_count) in snapshot.player_stats.iter() {
                        diesel::insert_into(break_count_full_snapshot)
                            .values((
                                full_snapshot_point_id.eq(inserted_point_id),
                                player_uuid.eq(player.uuid.as_str()?),
                                value.eq(break_count.0),
                            ))
                            .execute(conn)
                            .await?;
                    }
                }
                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    async fn read_latest_stats_snapshot_before(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
        filter: domain::repositories::ReadFilter,
    ) -> anyhow::Result<Option<StatsSnapshot<BreakCount>>> {
        todo!()
    }

    async fn read_first_stats_snapshot_after(
        &self,
        timestamp: chrono::DateTime<chrono::Utc>,
        filter: domain::repositories::ReadFilter,
    ) -> anyhow::Result<Option<StatsSnapshot<BreakCount>>> {
        todo!()
    }
}
