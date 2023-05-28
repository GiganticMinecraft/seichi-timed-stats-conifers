use anyhow::anyhow;
use std::collections::{HashMap, HashSet};

use crate::cycle_free_path::construct_cycle_free_path;
use crate::structures_embedded_in_rdb::{
    ComputeDiff, DiffPoint, DiffPointId, DiffSequence, FullSnapshotPoint, IdIndexedDiffPoints,
    SnapshotDiff, SnapshotPoint,
};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use diesel_async::AsyncConnection;

use domain::models::{BreakCount, Player, PlayerUuidString, StatsSnapshot};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use TimeBasedSnapshotSearchCondition::{NewestBefore, OldestAfter};

pub trait FromValueColumn {
    type ValueColumnType;
    fn from_value_column(value_column: Self::ValueColumnType) -> Self;
}

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTables<Conn>: Sized + Eq + Clone + FromValueColumn {
    async fn create_full_snapshot_point(conn: &mut Conn) -> anyhow::Result<u64>;

    async fn insert_all_stats_at_full_snapshot_point(
        fresh_full_snapshot_point_id: u64,
        player_stats: HashMap<Player, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()>;

    async fn create_diff_snapshot_point(
        base_point_id: u64,
        previous_diff_point_id_: Option<DiffPointId>,
        timestamp: DateTime<Utc>,
        conn: &mut Conn,
    ) -> anyhow::Result<DiffPointId>;

    async fn insert_all_stats_at_diff_snapshot_point(
        fresh_diff_snapshot_point_id: DiffPointId,
        player_stats_diffs: HashMap<PlayerUuidString, Self>,
        conn: &mut Conn,
    ) -> anyhow::Result<()>;

    async fn read_full_snapshot_point(
        full_snapshot_point_id: u64,
        conn: &mut Conn,
    ) -> anyhow::Result<FullSnapshotPoint<Self>>;

    async fn read_diff_snapshot_points(
        diff_snapshot_point_ids: HashSet<DiffPointId>,
        conn: &mut Conn,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>>;

    async fn read_diff_snapshot_points_over_full_point(
        full_snapshot_point_id: u64,
        conn: &mut Conn,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>>;

    async fn find_id_and_timestamp_of_full_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<(u64, NaiveDateTime)>>;

    async fn find_id_and_timestamp_of_diff_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<(DiffPointId, NaiveDateTime)>>;

    async fn find_id_of_latest_full_snapshot_before(
        timestamp: DateTime<Utc>,
        conn: &mut Conn,
    ) -> anyhow::Result<Option<u64>>;

    async fn id_of_root_full_snapshot_of_diff_point(
        diff_point_id: DiffPointId,
        conn: &mut Conn,
    ) -> anyhow::Result<u64>;

    async fn diff_point_id_to_previous_diff_point_id(
        forest_base_full_snapshot_point_id: u64,
        timestamp_upper_bound: DateTime<Utc>,
        conn: &mut Conn,
    ) -> anyhow::Result<HashMap<DiffPointId, Option<DiffPointId>>>;
}

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTablesDefaultMethods<Connection: AsyncConnection + Send + 'static>:
    HasIncrementalSnapshotTables<Connection>
{
    async fn create_full_snapshot(
        snapshot: StatsSnapshot<Self>,
        conn: &mut Connection,
    ) -> anyhow::Result<()> {
        let inserted_point_id = Self::create_full_snapshot_point(conn).await?;

        Self::insert_all_stats_at_full_snapshot_point(
            inserted_point_id,
            snapshot.player_stats,
            conn,
        )
        .await
    }

    async fn find_latest_full_snapshot_before(
        timestamp: DateTime<Utc>,
        conn: &mut Connection,
    ) -> anyhow::Result<Option<FullSnapshotPoint<Self>>> {
        if let Some(id) = Self::find_id_of_latest_full_snapshot_before(timestamp, conn).await? {
            let full_snapshot = Self::read_full_snapshot_point(id, conn).await?;
            Ok(Some(full_snapshot))
        } else {
            Ok(None)
        }
    }

    async fn find_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Connection,
    ) -> anyhow::Result<Option<SnapshotPoint<Self>>> {
        let found_full_snapshot_point =
            Self::find_id_and_timestamp_of_full_snapshot_point_with_condition(
                time_based_condition,
                conn,
            )
            .await?;

        let found_diff_snapshot_point =
            Self::find_id_and_timestamp_of_diff_snapshot_point_with_condition(
                time_based_condition,
                conn,
            )
            .await?;

        match (found_full_snapshot_point, found_diff_snapshot_point) {
            (None, None) => return Ok(None),
            (Some((full_id, _)), None) => {
                // full snapshot point の方を採用する
                Ok(Some(SnapshotPoint::Full(
                    Self::read_full_snapshot_point(full_id, conn).await?,
                )))
            }
            (Some((full_id, full_timestamp)), Some((_, diff_timestamp)))
                if matches!(time_based_condition, NewestBefore(_))
                    && full_timestamp > diff_timestamp =>
            {
                // full snapshot point の方を採用する
                Ok(Some(SnapshotPoint::Full(
                    Self::read_full_snapshot_point(full_id, conn).await?,
                )))
            }
            (_, Some((diff_id, _))) => {
                // 条件に合致する full snapshot point が存在しないか、
                // diff snapshot point の方が time_based_condition の timestamp に近いため、
                // diff snapshot point の方を採用する
                let diff_point =
                    Self::read_diff_snapshot_points(vec![diff_id].into_iter().collect(), conn)
                        .await?
                        .remove(&diff_id)
                        .unwrap();

                Ok(Some(SnapshotPoint::Diff(diff_point)))
            }
        }
    }

    async fn create_diff_snapshot_point_on(
        diff_sequence: DiffSequence<Self>,
        snapshot: StatsSnapshot<Self>,
        conn: &mut Connection,
    ) -> anyhow::Result<()> {
        let root_point_id = diff_sequence.base_point.id;
        let previous_point_id = diff_sequence.diff_points.last().map(|p| p.id);
        let diff = diff_sequence.into_snapshot_at_the_tip().diff_to(snapshot);

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

    async fn construct_diff_sequence_leading_up_to(
        snapshot_point: SnapshotPoint<Self>,
        conn: &mut Connection,
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

    async fn construct_diff_sequence_leading_up_to_diff_point(
        diff_point: DiffPoint<Self>,
        conn: &mut Connection,
    ) -> anyhow::Result<DiffSequence<Self>> {
        let root_point_id =
            Self::id_of_root_full_snapshot_of_diff_point(diff_point.id, conn).await?;
        let diff_point_id_to_previous_id_map = Self::diff_point_id_to_previous_diff_point_id(
            root_point_id,
            diff_point.diff.utc_timestamp,
            conn,
        )
        .await?;

        // `diff_point` に対応する diff point からその root full snapshot point までの
        // diff point の ID をさかのぼるような `Vec`。
        let ids_of_diff_points_towards_root =
            construct_cycle_free_path(diff_point.id, |id| diff_point_id_to_previous_id_map[&id])?;

        let diff_points_towards_given_point = {
            let ids_of_diff_points_towards_tip = {
                let mut ids = ids_of_diff_points_towards_root;
                ids.reverse();
                ids
            };

            let ids_set = ids_of_diff_points_towards_tip.iter().copied().collect();
            Self::read_diff_snapshot_points(ids_set, conn)
                .await?
                .map_ids_to_diff_points(&ids_of_diff_points_towards_tip)
        };

        let full_snapshot = Self::read_full_snapshot_point(root_point_id, conn).await?;
        Ok(DiffSequence::new(
            full_snapshot,
            diff_points_towards_given_point,
        ))
    }
}

impl<T: HasIncrementalSnapshotTables<Connection>, Connection: AsyncConnection + Send + 'static>
    HasIncrementalSnapshotTablesDefaultMethods<Connection> for T
{
}

impl FromValueColumn for BreakCount {
    type ValueColumnType = u64;

    fn from_value_column(value_column: Self::ValueColumnType) -> Self {
        Self(value_column)
    }
}

use crate::query_utils::RunFirstOptionalDsl;
use crate::schema;
use diesel::mysql::Mysql;
use diesel::query_dsl::methods::*;
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;

#[async_trait::async_trait]
impl<Connection: AsyncConnection<Backend = Mysql> + Send + 'static>
    HasIncrementalSnapshotTables<Connection> for BreakCount
{
    async fn create_full_snapshot_point(conn: &mut Connection) -> anyhow::Result<u64> {
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

    async fn insert_all_stats_at_full_snapshot_point(
        fresh_full_snapshot_point_id: u64,
        player_stats: HashMap<Player, Self>,
        conn: &mut Connection,
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

    async fn create_diff_snapshot_point(
        base_point_id: u64,
        previous_diff_point_id_: Option<DiffPointId>,
        timestamp: DateTime<Utc>,
        conn: &mut Connection,
    ) -> anyhow::Result<DiffPointId> {
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
        Ok(DiffPointId(created_diff_snapshot_point_id))
    }

    async fn insert_all_stats_at_diff_snapshot_point(
        fresh_diff_snapshot_point_id: DiffPointId,
        player_stats_diffs: HashMap<PlayerUuidString, Self>,
        conn: &mut Connection,
    ) -> anyhow::Result<()> {
        use schema::break_count_diff::dsl::*;
        let records_to_insert = player_stats_diffs
            .iter()
            .map(|(uuid, break_count)| {
                anyhow::Ok((
                    diff_point_id.eq(fresh_diff_snapshot_point_id.0),
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

    async fn read_full_snapshot_point(
        full_snapshot_point_id: u64,
        conn: &mut Connection,
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
        }
        .await?
        .into_iter()
        .map(|(uuid, value)| {
            let player = Player {
                uuid: PlayerUuidString::from_string(&uuid)?,
            };
            anyhow::Ok((player, Self::from_value_column(value)))
        })
        .collect::<Result<HashMap<Player, Self>, _>>()?;

        Ok(FullSnapshotPoint {
            id: full_snapshot_point_id,
            full_snapshot: StatsSnapshot {
                utc_timestamp: Utc.from_utc_datetime(&snapshot_timestamp),
                player_stats,
            },
        })
    }

    async fn read_diff_snapshot_points(
        diff_snapshot_point_ids: HashSet<DiffPointId>,
        conn: &mut Connection,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>> {
        let diff_point_data_map = {
            use schema::break_count_diff_point::dsl::*;
            break_count_diff_point
                .select((id, previous_diff_point_id, record_timestamp))
                .filter(id.eq_any(&diff_snapshot_point_ids))
                .load::<(DiffPointId, Option<DiffPointId>, NaiveDateTime)>(conn)
        }
        .await?
        .into_iter()
        .map(|(id, previous_diff_point_id, record_timestamp)| {
            (id, (previous_diff_point_id, record_timestamp))
        })
        .collect::<HashMap<_, _>>();

        if diff_point_data_map.len() != diff_snapshot_point_ids.len() {
            let ids_in_table = diff_point_data_map.keys().copied().collect();
            let ids_without_timestamp = diff_snapshot_point_ids
                .difference(&ids_in_table)
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
                .load::<(DiffPointId, String, u64)>(conn)
        }
        .await?
        .into_iter()
        .map(|(point_id, uuid, value)| {
            anyhow::Ok((
                point_id,
                PlayerUuidString::from_string(&uuid)?,
                Self::from_value_column(value),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

        let mut diff_points = HashMap::new();
        for (diff_point_id, uuid, new_value) in diffs {
            diff_points
                .entry(diff_point_id)
                .or_insert_with(|| HashMap::new())
                .insert(uuid, new_value);
        }

        Ok(IdIndexedDiffPoints::new(
            diff_points
                .into_iter()
                .map(|(diff_point_id, player_stats_diffs)| {
                    let (previous_diff_point_id, timestamp) = diff_point_data_map[&diff_point_id];

                    DiffPoint {
                        id: diff_point_id,
                        previous_diff_point_id,
                        diff: SnapshotDiff {
                            utc_timestamp: Utc.from_utc_datetime(&timestamp),
                            player_stats_diffs,
                        },
                    }
                })
                .collect(),
        ))
    }

    async fn read_diff_snapshot_points_over_full_point(
        full_snapshot_point_id: u64,
        conn: &mut Connection,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>> {
        use schema::break_count_diff_point::dsl::*;

        let diff_point_ids = break_count_diff_point
            .select(id)
            .filter(root_full_snapshot_point_id.eq(full_snapshot_point_id))
            .load::<DiffPointId>(conn)
            .await?;

        Ok(Self::read_diff_snapshot_points(diff_point_ids.into_iter().collect(), conn).await?)
    }

    async fn find_id_and_timestamp_of_full_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Connection,
    ) -> anyhow::Result<Option<(u64, NaiveDateTime)>> {
        use schema::break_count_full_snapshot_point::dsl::*;
        match time_based_condition {
            NewestBefore(timestamp) => Ok(break_count_full_snapshot_point
                .select((id, record_timestamp))
                .filter(record_timestamp.le(timestamp.naive_utc()))
                .order(record_timestamp.desc())
                .first_optional::<(u64, NaiveDateTime)>(conn)
                .await?),
            OldestAfter(timestamp) => Ok(break_count_full_snapshot_point
                .select((id, record_timestamp))
                .filter(record_timestamp.ge(timestamp.naive_utc()))
                .order(record_timestamp.asc())
                .first_optional::<(u64, NaiveDateTime)>(conn)
                .await?),
        }
    }

    async fn find_id_and_timestamp_of_diff_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut Connection,
    ) -> anyhow::Result<Option<(DiffPointId, NaiveDateTime)>> {
        use schema::break_count_diff_point::dsl::*;
        match time_based_condition {
            NewestBefore(timestamp) => Ok(break_count_diff_point
                .select((id, record_timestamp))
                .filter(record_timestamp.le(timestamp.naive_utc()))
                .order(record_timestamp.desc())
                .first_optional::<(DiffPointId, NaiveDateTime)>(conn)
                .await?),
            OldestAfter(timestamp) => Ok(break_count_diff_point
                .select((id, record_timestamp))
                .filter(record_timestamp.ge(timestamp.naive_utc()))
                .order(record_timestamp.asc())
                .first_optional::<(DiffPointId, NaiveDateTime)>(conn)
                .await?),
        }
    }

    async fn find_id_of_latest_full_snapshot_before(
        timestamp: DateTime<Utc>,
        conn: &mut Connection,
    ) -> anyhow::Result<Option<u64>> {
        use schema::break_count_full_snapshot_point::dsl::*;
        Ok(break_count_full_snapshot_point
            .select(id)
            .filter(record_timestamp.le(timestamp.naive_utc()))
            .order(record_timestamp.desc())
            .first_optional::<u64>(conn)
            .await?)
    }

    async fn id_of_root_full_snapshot_of_diff_point(
        diff_point_id: DiffPointId,
        conn: &mut Connection,
    ) -> anyhow::Result<u64> {
        use schema::break_count_diff_point::dsl::*;
        Ok(break_count_diff_point
            .select(root_full_snapshot_point_id)
            .filter(id.eq(diff_point_id))
            .first::<u64>(conn)
            .await?)
    }

    async fn diff_point_id_to_previous_diff_point_id(
        forest_base_full_snapshot_point_id: u64,
        timestamp_upper_bound: DateTime<Utc>,
        conn: &mut Connection,
    ) -> anyhow::Result<HashMap<DiffPointId, Option<DiffPointId>>> {
        use schema::break_count_diff_point::dsl::*;
        let diff_point_id_to_previous_diff_point_id = break_count_diff_point
            .select((id, previous_diff_point_id))
            .filter(root_full_snapshot_point_id.eq(forest_base_full_snapshot_point_id))
            .filter(record_timestamp.le(timestamp_upper_bound.naive_utc()))
            .load::<(DiffPointId, Option<DiffPointId>)>(conn)
            .await?
            .into_iter()
            .collect();
        Ok(diff_point_id_to_previous_diff_point_id)
    }
}
