use anyhow::anyhow;
use std::collections::{HashMap, HashSet};

use crate::schema;
use crate::structures::{ComputeDiff, DiffPoint, FullSnapshotPoint, SnapshotDiff};
use crate::structures::{DiffSequence, SnapshotPoint};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use diesel::mysql::Mysql;
use diesel::query_dsl::methods::*;
use diesel::ExpressionMethods;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;

use domain::models::{BreakCount, Player, PlayerUuidString, StatsSnapshot};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use TimeBasedSnapshotSearchCondition::{NewestBefore, OldestAfter};

pub trait FromValueColumn {
    type ValueColumnType;
    fn from_value_column(value_column: Self::ValueColumnType) -> Self;
}

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTables: Sized + Eq + Clone + FromValueColumn {
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

    async fn read_full_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        full_snapshot_point_id: u64,
        conn: &mut Conn,
    ) -> anyhow::Result<FullSnapshotPoint<Self>>;

    async fn read_diff_snapshot_points<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        diff_snapshot_point_ids: HashSet<u64>,
        conn: &mut Conn,
    ) -> anyhow::Result<HashMap<u64, DiffPoint<Self>>>;

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

impl FromValueColumn for BreakCount {
    type ValueColumnType = u64;

    fn from_value_column(value_column: Self::ValueColumnType) -> Self {
        Self(value_column)
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
        let created_full_snapshot_point_id = {
            use schema::break_count_full_snapshot_point::dsl::*;
            break_count_full_snapshot_point
                .select(id)
                .order(id.desc())
                .first::<u64>(conn)
                .await?
        };

        Ok(created_full_snapshot_point_id)
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
                anyhow::Ok((
                    full_snapshot_point_id.eq(fresh_full_snapshot_point_id),
                    player_uuid.eq(player.uuid.as_str()?),
                    value.eq(break_count.0),
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;

        diesel::insert_into(break_count_full_snapshot)
            .values(records_to_insert)
            .execute(conn)
            .await?;

        Ok(())
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
        let created_diff_snapshot_point_id = {
            use schema::break_count_diff_point::dsl::*;
            break_count_diff_point
                .select(id)
                .order(id.desc())
                .first::<u64>(conn)
                .await?
        };
        Ok(created_diff_snapshot_point_id)
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
            .map(|(uuid, break_count)| {
                anyhow::Ok((
                    diff_point_id.eq(fresh_diff_snapshot_point_id),
                    player_uuid.eq(uuid.as_str()?),
                    new_value.eq(break_count.0),
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;

        diesel::insert_into(break_count_diff)
            .values(records_to_insert)
            .execute(conn)
            .await?;

        Ok(())
    }

    async fn read_full_snapshot_point<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        full_snapshot_point_id: u64,
        conn: &mut Conn,
    ) -> anyhow::Result<FullSnapshotPoint<Self>> {
        let snapshot_timestamp = {
            use schema::break_count_full_snapshot_point::dsl::*;
            break_count_full_snapshot_point
                .select(record_timestamp)
                .filter(id.eq(full_snapshot_point_id))
                .first::<NaiveDateTime>(conn)
                .await?
        };

        let player_stats = {
            use schema::break_count_full_snapshot::dsl::*;
            break_count_full_snapshot
                .select((player_uuid, value))
                .filter(full_snapshot_point_id.eq(full_snapshot_point_id))
                .load::<(String, u64)>(conn)
                .await?
                .into_iter()
                .map(|(uuid, stats_value)| {
                    let player = Player {
                        uuid: PlayerUuidString::from_string(&uuid)?,
                    };
                    let stats_value = Self::from_value_column(stats_value);
                    anyhow::Ok((player, stats_value))
                })
                .collect::<Result<HashMap<Player, Self>, _>>()?
        };

        Ok(FullSnapshotPoint {
            id: full_snapshot_point_id,
            full_snapshot: StatsSnapshot {
                utc_timestamp: Utc.from_utc_datetime(&snapshot_timestamp),
                player_stats,
            },
        })
    }

    async fn read_diff_snapshot_points<Conn: AsyncConnection<Backend = Mysql> + Send + 'static>(
        diff_snapshot_point_ids: HashSet<u64>,
        conn: &mut Conn,
    ) -> anyhow::Result<HashMap<u64, DiffPoint<Self>>> {
        let diff_point_timestamps = {
            use schema::break_count_diff_point::dsl::*;
            break_count_diff_point
                .select((id, record_timestamp))
                .filter(id.eq_any(&diff_snapshot_point_ids))
                .load::<(u64, NaiveDateTime)>(conn)
                .await?
                .into_iter()
                .collect::<HashMap<_, _>>()
        };

        if diff_point_timestamps.len() != diff_snapshot_point_ids.len() {
            let ids_without_timestamp = diff_snapshot_point_ids
                .difference(&diff_point_timestamps.keys().copied().collect())
                .collect::<Vec<_>>();
            return Err(anyhow!(
                "diff point ids {:?} are not found in diff_point table",
                ids_without_timestamp
            ));
        }

        let diffs = {
            use schema::break_count_diff::dsl::*;
            break_count_diff
                .select((diff_point_id, player_uuid, new_value))
                .filter(diff_point_id.eq_any(&diff_snapshot_point_ids))
                .load::<(u64, String, u64)>(conn)
                .await?
        };

        let mut diff_points = HashMap::new();
        for (diff_point_id, uuid, new_value) in diffs {
            diff_points
                .entry(diff_point_id)
                .or_insert_with(|| HashMap::new())
                .insert(
                    PlayerUuidString::from_string(&uuid)?,
                    Self::from_value_column(new_value),
                );
        }

        Ok(diff_points
            .into_iter()
            .map(|(diff_point_id, player_stats_diffs)| {
                let timestamp = diff_point_timestamps[&diff_point_id];
                (
                    diff_point_id,
                    DiffPoint {
                        id: diff_point_id,
                        diff: SnapshotDiff {
                            utc_timestamp: Utc.from_utc_datetime(&timestamp),
                            player_stats_diffs,
                        },
                    },
                )
            })
            .collect())
    }

    async fn find_snapshot_point_with_condition<
        Conn: AsyncConnection<Backend = Mysql> + Send + 'static,
    >(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<SnapshotPoint<Self>>> {
        let found_full_snapshot_point: (u64, NaiveDateTime) = {
            use schema::break_count_full_snapshot_point::dsl::*;
            match time_based_condition {
                NewestBefore(timestamp) => {
                    break_count_full_snapshot_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.le(timestamp.naive_utc()))
                        .order(record_timestamp.desc())
                        .limit(1)
                        .first::<(u64, NaiveDateTime)>(conn)
                        .await?
                }
                OldestAfter(timestamp) => {
                    break_count_full_snapshot_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.ge(timestamp.naive_utc()))
                        .order(record_timestamp.asc())
                        .limit(1)
                        .first::<(u64, NaiveDateTime)>(conn)
                        .await?
                }
            }
        };

        let found_diff_snapshot_point: (u64, NaiveDateTime) = {
            use schema::break_count_diff_point::dsl::*;
            match time_based_condition {
                NewestBefore(timestamp) => {
                    break_count_diff_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.le(timestamp.naive_utc()))
                        .order(record_timestamp.desc())
                        .limit(1)
                        .first::<(u64, NaiveDateTime)>(conn)
                        .await?
                }
                OldestAfter(timestamp) => {
                    break_count_diff_point
                        .select((id, record_timestamp))
                        .filter(record_timestamp.ge(timestamp.naive_utc()))
                        .order(record_timestamp.asc())
                        .limit(1)
                        .first::<(u64, NaiveDateTime)>(conn)
                        .await?
                }
            }
        };

        if matches!(time_based_condition, NewestBefore(_))
            && found_full_snapshot_point.1 > found_diff_snapshot_point.1
        {
            // full snapshot point の方を採用する
            let full_snapshot =
                Self::read_full_snapshot_point(found_full_snapshot_point.0, conn).await?;

            Ok(Some(SnapshotPoint::Full(full_snapshot)))
        } else {
            // diff snapshot point の方を採用する
            let set_containing_id = std::iter::once(found_diff_snapshot_point.0).collect();
            let diff_point = Self::read_diff_snapshot_points(set_containing_id, conn)
                .await?
                .remove(&found_diff_snapshot_point.0)
                .unwrap();

            Ok(Some(SnapshotPoint::Diff(diff_point)))
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
