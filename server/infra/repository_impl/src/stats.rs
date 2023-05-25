use std::collections::HashMap;

use crate::schema;
use crate::structures::{ComputeDiff, DiffPoint, FullSnapshotPoint, SnapshotDiff};
use crate::structures::{DiffSequence, SnapshotPoint};
use chrono::{DateTime, TimeZone, Utc};
use diesel::mysql::Mysql;
use diesel::query_dsl::methods::*;
use diesel::ExpressionMethods;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;

use domain::models::{BreakCount, Player, PlayerUuidString, StatsSnapshot};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use TimeBasedSnapshotSearchCondition::{NewestBefore, OldestAfter};

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTables: Sized {
    async fn create_full_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        conn: &mut Conn,
    ) -> anyhow::Result<u64>;

    async fn insert_all_stats_at_full_snapshot_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        fresh_full_snapshot_point_id: u64,
        player_stats: HashMap<Player, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()>;

    async fn create_diff_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        base_point_id: u64,
        previous_diff_point_id: Option<u64>,
        timestamp: DateTime<Utc>,
        conn: &mut Conn,
    ) -> anyhow::Result<u64>;

    async fn insert_all_stats_at_diff_snapshot_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        fresh_diff_snapshot_point_id: u64,
        player_stats_diffs: HashMap<PlayerUuidString, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()>;

    async fn find_snapshot_point_with_condition<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<SnapshotPoint<Self>>>;

    async fn construct_diff_sequence_leading_up_to_diff_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        diff_point: DiffPoint<Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<DiffSequence<Self>>;

    async fn create_full_snapshot<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        snapshot: StatsSnapshot<Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()> {
        let inserted_point_id = Self::create_full_snapshot_point(conn).await?;

        Self::insert_all_stats_at_full_snapshot_point(
            inserted_point_id,
            snapshot.player_stats,
            conn,
        )
        .await
    }

    async fn create_diff_snapshot_point_on<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        diff_sequence: DiffSequence<Self>,
        snapshot: StatsSnapshot<Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()> {
        let root_point_id = diff_sequence.base_point.id;
        let previous_point_id = diff_sequence.diff_points.last().map(|p| p.id);
        let diff = diff_sequence.into_snapshot_at_the_end().diff_to(snapshot);

        let inserted_diff_point_id = Self::create_diff_snapshot_point(
            root_point_id,
            previous_point_id,
            diff.utc_timestamp,
            conn,
        )
        .await?;

        Self::insert_all_stats_at_diff_snapshot_point(
            inserted_diff_point_id,
            diff.player_stats_diffs,
            conn,
        )
        .await
    }

    async fn construct_diff_sequence_leading_up_to<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        snapshot_point: SnapshotPoint<Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<DiffSequence<Self>> {
        match snapshot_point {
            SnapshotPoint::Full(snapshot_point) => {
                Ok(DiffSequence::without_any_diffs(snapshot_point))
            }
            SnapshotPoint::Diff(diff_point) => {
                Self::construct_diff_sequence_leading_up_to_diff_point(diff_point, conn).await
            }
        }
    }
}

#[async_trait::async_trait]
impl HasIncrementalSnapshotTables for BreakCount {
    async fn create_full_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        conn: &mut Conn,
    ) -> anyhow::Result<u64> {
        {
            use schema::break_count_full_snapshot_point::dsl::*;
            diesel::insert_into(break_count_full_snapshot_point)
                .values(record_timestamp.eq(Utc::now().naive_utc()))
                .execute(conn)
                .await?;
        }
        {
            use schema::break_count_full_snapshot_point::dsl::*;
            break_count_full_snapshot_point
                .select(id)
                .order(id.desc())
                .first::<u64>(conn)
                .await
        }
    }

    async fn insert_all_stats_at_full_snapshot_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        fresh_full_snapshot_point_id: u64,
        player_stats: HashMap<Player, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()> {
        use schema::break_count_full_snapshot::dsl::*;
        let records_to_insert = player_stats
            .iter()
            .map(|(player, break_count)| {
                (
                    full_snapshot_point_id.eq(fresh_full_snapshot_point_id),
                    player_uuid.eq(player.uuid.as_str()?),
                    value.eq(break_count.0),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(break_count_full_snapshot)
            .values(records_to_insert)
            .execute(conn)
            .await?
    }

    async fn create_diff_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        base_point_id: u64,
        previous_diff_point_id_: Option<u64>,
        timestamp: DateTime<Utc>,
        conn: &mut Conn,
    ) -> anyhow::Result<u64> {
        {
            use schema::break_count_diff_point::dsl::*;
            diesel::insert_into(break_count_diff_point)
                .values((
                    root_full_snapshot_point_id.eq(base_point_id),
                    previous_diff_point_id.eq(previous_diff_point_id_),
                    record_timestamp.eq(timestamp.naive_utc()),
                ))
                .execute(conn)
                .await?;
        }
        {
            use schema::break_count_diff_point::dsl::*;
            break_count_diff_point
                .select(id)
                .order(id.desc())
                .first::<u64>(conn)
                .await
        }
    }

    async fn insert_all_stats_at_diff_snapshot_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        fresh_diff_snapshot_point_id: u64,
        player_stats_diffs: HashMap<PlayerUuidString, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()> {
        use schema::break_count_diff::dsl::*;
        let records_to_insert = player_stats_diffs
            .iter()
            .map(|(player_uuid, break_count)| {
                (
                    diff_point_id.eq(fresh_diff_snapshot_point_id),
                    player_uuid.eq(player_uuid.as_str()?),
                    new_value.eq(break_count.0),
                )
            })
            .collect::<Vec<_>>();

        diesel::insert_into(break_count_diff)
            .values(records_to_insert)
            .execute(conn)
            .await
    }

    async fn find_snapshot_point_with_condition<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<SnapshotPoint<Self>>> {
        let found_full_snapshot_point: (u64, chrono::NaiveDateTime) = {
            use schema::break_count_full_snapshot_point::dsl::*;
            match time_based_condition {
                NewestBefore(timestamp) => {
                    break_count_full_snapshot_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.le(timestamp.naive_utc()))
                        .order(record_timestamp.desc())
                        .limit(1)
                        .first::<(u64, chrono::NaiveDateTime)>(conn)
                        .await?
                }
                OldestAfter(timestamp) => {
                    break_count_full_snapshot_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.ge(timestamp.naive_utc()))
                        .order(record_timestamp.asc())
                        .limit(1)
                        .first::<(u64, chrono::NaiveDateTime)>(conn)
                        .await?
                }
            }
        };

        let found_diff_snapshot_point: (u64, chrono::NaiveDateTime) = {
            use schema::break_count_diff_point::dsl::*;
            match time_based_condition {
                NewestBefore(timestamp) => {
                    break_count_diff_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.le(timestamp.naive_utc()))
                        .order(record_timestamp.desc())
                        .limit(1)
                        .first::<(u64, chrono::NaiveDateTime)>(conn)
                        .await?
                }
                OldestAfter(timestamp) => {
                    break_count_diff_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.ge(timestamp.naive_utc()))
                        .order(record_timestamp.asc())
                        .limit(1)
                        .first::<(u64, chrono::NaiveDateTime)>(conn)
                        .await?
                }
            }
        };

        if matches!(time_based_condition, NewestBefore(_))
            && found_full_snapshot_point.1 > found_diff_snapshot_point.1
        {
            // full snapshot point の方を採用する
            let full_snapshot = {
                use schema::break_count_full_snapshot::dsl::*;
                break_count_full_snapshot
                    .select((player_uuid, value))
                    .filter(full_snapshot_point_id.eq(found_full_snapshot_point.0))
                    .load::<(String, u64)>(conn)
                    .await?
            };
            let mut player_stats = HashMap::new();
            for (uuid, value) in full_snapshot {
                player_stats.insert(
                    Player {
                        uuid: PlayerUuidString::from_string(&uuid)?,
                    },
                    BreakCount(value as u64),
                );
            }
            Ok(Some(SnapshotPoint::Full(FullSnapshotPoint {
                id: last_full_snapshot_point.0,
                full_snapshot: StatsSnapshot {
                    utc_timestamp: Utc.from_utc_datetime(&found_full_snapshot_point.1),
                    player_stats,
                },
            })))
        } else {
            // diff snapshot point の方を採用する
            let diff = {
                use schema::break_count_diff::dsl::*;
                break_count_diff
                    .select((player_uuid, new_value))
                    .filter(diff_point_id.eq(found_diff_snapshot_point.0))
                    .load::<(String, u64)>(conn)
                    .await?
            };
            let mut player_stats_diffs = HashMap::new();
            for (uuid, new_value) in diff {
                player_stats_diffs.insert(
                    PlayerUuidString::from_string(&uuid)?,
                    BreakCount(new_value as u64),
                );
            }
            Ok(Some(SnapshotPoint::Diff(DiffPoint {
                id: last_diff_snapshot_point.0,
                diff: SnapshotDiff {
                    utc_timestamp: Utc.from_utc_datetime(&found_diff_snapshot_point.1),
                    player_stats_diffs,
                },
            })))
        }
    }

    async fn construct_diff_sequence_leading_up_to_diff_point<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        _diff_point: DiffPoint<Self>,
        _conn: &mut Conn,
    ) -> anyhow::Result<DiffSequence<Self>> {
        todo!()
    }
}
