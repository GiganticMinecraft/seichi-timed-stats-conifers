use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use diesel::mysql::Mysql;
use diesel::query_dsl::methods::*;
use diesel::ExpressionMethods;
use diesel_async::{AsyncConnection, RunQueryDsl};
use domain::models::{BreakCount, BuildCount, PlayTicks, Player, VoteCount};
use std::collections::{HashMap, HashSet};

use anyhow::anyhow;

use crate::structures_embedded_in_rdb::{
    DiffPoint, DiffPointId, FullSnapshotPoint, IdIndexedDiffPoints, SnapshotDiff,
};
use crate::TimeBasedSnapshotSearchCondition::{NewestBefore, OldestAfter};
use domain::models::{PlayerUuidString, StatsSnapshot};
use domain::repositories::TimeBasedSnapshotSearchCondition;

use super::schema;
use crate::diesel_based_impl::query_utils::RunFirstOptionalDsl;
use crate::stats_with_incremental_snapshot_tables::HasIncrementalSnapshotTables;

pub trait FromValueColumn {
    type ValueColumnType;
    fn from_value_column(value_column: Self::ValueColumnType) -> Self;
}

macro_rules! impl_from_value_column {
    ($stats_type:ty, $stats_type_constructor:ident, $column_type:ty) => {
        impl FromValueColumn for $stats_type {
            type ValueColumnType = $column_type;
            fn from_value_column(value_column: Self::ValueColumnType) -> Self {
                $stats_type_constructor(value_column)
            }
        }
    };
}

impl_from_value_column!(BreakCount, BreakCount, u64);
impl_from_value_column!(BuildCount, BuildCount, u64);
impl_from_value_column!(PlayTicks, PlayTicks, u64);
impl_from_value_column!(VoteCount, VoteCount, u64);

macro_rules! impl_has_incremental_snapshot_tables {
    ($stats_type:ty,
     $full_snapshot_point_table:ident,
     $full_snapshot_dsl_table:ident,
     $diff_point_table:ident,
     $diff_table:ident) => {
        #[async_trait::async_trait]
        impl<Connection: AsyncConnection<Backend = Mysql> + Send + 'static>
            HasIncrementalSnapshotTables<Connection> for $stats_type
        {
            async fn create_full_snapshot_point(conn: &mut Connection) -> anyhow::Result<u64> {
                {
                    use schema::$full_snapshot_point_table::dsl;
                    diesel::insert_into(dsl::$full_snapshot_point_table)
                        .values(dsl::record_timestamp.eq(Utc::now().naive_utc()))
                        .execute(conn)
                        .await?;
                }
                let created_full_snapshot_point_id = {
                    use schema::$full_snapshot_point_table::dsl;
                    dsl::$full_snapshot_point_table
                        .select(dsl::id)
                        .order(dsl::id.desc())
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
                use schema::$full_snapshot_dsl_table::dsl;
                let records_to_insert = player_stats
                    .iter()
                    .map(|(player, stats)| {
                        anyhow::Ok((
                            dsl::full_snapshot_point_id.eq(fresh_full_snapshot_point_id),
                            dsl::player_uuid.eq(player.uuid.as_str()?),
                            dsl::value.eq(stats.0),
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                diesel::insert_into(dsl::$full_snapshot_dsl_table)
                    .values(records_to_insert)
                    .execute(conn)
                    .await?;

                Ok(())
            }

            async fn create_diff_snapshot_point(
                base_full_snapshot_point_id: u64,
                previous_diff_point_id: Option<DiffPointId>,
                timestamp: DateTime<Utc>,
                conn: &mut Connection,
            ) -> anyhow::Result<DiffPointId> {
                {
                    use schema::$diff_point_table::dsl;
                    diesel::insert_into(dsl::$diff_point_table)
                        .values((
                            dsl::root_full_snapshot_point_id.eq(base_full_snapshot_point_id),
                            dsl::previous_diff_point_id.eq(previous_diff_point_id),
                            dsl::record_timestamp.eq(timestamp.naive_utc()),
                        ))
                        .execute(conn)
                        .await?;
                }
                let created_diff_snapshot_point_id = {
                    use schema::$diff_point_table::dsl;
                    dsl::$diff_point_table
                        .select(dsl::id)
                        .order(dsl::id.desc())
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
                use schema::$diff_table::dsl;
                let records_to_insert = player_stats_diffs
                    .iter()
                    .map(|(player_uuid, stats)| {
                        anyhow::Ok((
                            dsl::diff_point_id.eq(fresh_diff_snapshot_point_id.0),
                            dsl::player_uuid.eq(player_uuid.as_str()?),
                            dsl::new_value.eq(stats.0),
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                diesel::insert_into(dsl::$diff_table)
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
                    use schema::$full_snapshot_point_table::dsl;
                    dsl::$full_snapshot_point_table
                        .select(dsl::record_timestamp)
                        .filter(dsl::id.eq(full_snapshot_point_id))
                        .first::<NaiveDateTime>(conn)
                        .await?
                };

                let player_stats = {
                    use schema::$full_snapshot_dsl_table::dsl;
                    dsl::$full_snapshot_dsl_table
                        .select((dsl::player_uuid, dsl::value))
                        .filter(dsl::full_snapshot_point_id.eq(full_snapshot_point_id))
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
                    use schema::$diff_point_table::dsl;
                    dsl::$diff_point_table
                        .select((dsl::id, dsl::previous_diff_point_id, dsl::record_timestamp))
                        .filter(dsl::id.eq_any(&diff_snapshot_point_ids))
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
                    use schema::$diff_table::dsl;
                    dsl::$diff_table
                        .select((dsl::diff_point_id, dsl::player_uuid, dsl::new_value))
                        .filter(dsl::diff_point_id.eq_any(&diff_snapshot_point_ids))
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
                            let (previous_diff_point_id, timestamp) =
                                diff_point_data_map[&diff_point_id];

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
                use schema::$diff_point_table::dsl;

                let diff_point_ids = dsl::$diff_point_table
                    .select(dsl::id)
                    .filter(dsl::root_full_snapshot_point_id.eq(full_snapshot_point_id))
                    .load::<DiffPointId>(conn)
                    .await?;

                Ok(
                    Self::read_diff_snapshot_points(diff_point_ids.into_iter().collect(), conn)
                        .await?,
                )
            }

            async fn find_id_and_timestamp_of_full_snapshot_point_with_condition(
                time_based_condition: TimeBasedSnapshotSearchCondition,
                conn: &mut Connection,
            ) -> anyhow::Result<Option<(u64, NaiveDateTime)>> {
                use schema::$full_snapshot_point_table::dsl;
                match time_based_condition {
                    NewestBefore(timestamp) => Ok(dsl::$full_snapshot_point_table
                        .select((dsl::id, dsl::record_timestamp))
                        .filter(dsl::record_timestamp.le(timestamp.naive_utc()))
                        .order(dsl::record_timestamp.desc())
                        .first_optional::<(u64, NaiveDateTime)>(conn)
                        .await?),
                    OldestAfter(timestamp) => Ok(dsl::$full_snapshot_point_table
                        .select((dsl::id, dsl::record_timestamp))
                        .filter(dsl::record_timestamp.ge(timestamp.naive_utc()))
                        .order(dsl::record_timestamp.asc())
                        .first_optional::<(u64, NaiveDateTime)>(conn)
                        .await?),
                }
            }

            async fn find_id_and_timestamp_of_diff_snapshot_point_with_condition(
                time_based_condition: TimeBasedSnapshotSearchCondition,
                conn: &mut Connection,
            ) -> anyhow::Result<Option<(DiffPointId, NaiveDateTime)>> {
                use schema::$diff_point_table::dsl;
                match time_based_condition {
                    NewestBefore(timestamp) => Ok(dsl::$diff_point_table
                        .select((dsl::id, dsl::record_timestamp))
                        .filter(dsl::record_timestamp.le(timestamp.naive_utc()))
                        .order(dsl::record_timestamp.desc())
                        .first_optional::<(DiffPointId, NaiveDateTime)>(conn)
                        .await?),
                    OldestAfter(timestamp) => Ok(dsl::$diff_point_table
                        .select((dsl::id, dsl::record_timestamp))
                        .filter(dsl::record_timestamp.ge(timestamp.naive_utc()))
                        .order(dsl::record_timestamp.asc())
                        .first_optional::<(DiffPointId, NaiveDateTime)>(conn)
                        .await?),
                }
            }

            async fn find_id_of_latest_full_snapshot_before(
                timestamp: DateTime<Utc>,
                conn: &mut Connection,
            ) -> anyhow::Result<Option<u64>> {
                use schema::$full_snapshot_point_table::dsl;
                Ok(dsl::$full_snapshot_point_table
                    .select(dsl::id)
                    .filter(dsl::record_timestamp.le(timestamp.naive_utc()))
                    .order(dsl::record_timestamp.desc())
                    .first_optional::<u64>(conn)
                    .await?)
            }

            async fn id_of_root_full_snapshot_of_diff_point(
                diff_point_id: DiffPointId,
                conn: &mut Connection,
            ) -> anyhow::Result<u64> {
                use schema::$diff_point_table::dsl;
                Ok(dsl::$diff_point_table
                    .select(dsl::root_full_snapshot_point_id)
                    .filter(dsl::id.eq(diff_point_id))
                    .first::<u64>(conn)
                    .await?)
            }

            async fn diff_point_id_to_previous_diff_point_id(
                forest_base_full_snapshot_point_id: u64,
                timestamp_upper_bound: DateTime<Utc>,
                conn: &mut Connection,
            ) -> anyhow::Result<HashMap<DiffPointId, Option<DiffPointId>>> {
                use schema::$diff_point_table::dsl;
                let diff_point_id_to_previous_diff_point_id = dsl::$diff_point_table
                    .select((dsl::id, dsl::previous_diff_point_id))
                    .filter(dsl::root_full_snapshot_point_id.eq(forest_base_full_snapshot_point_id))
                    .filter(dsl::record_timestamp.le(timestamp_upper_bound.naive_utc()))
                    .load::<(DiffPointId, Option<DiffPointId>)>(conn)
                    .await?
                    .into_iter()
                    .collect();
                Ok(diff_point_id_to_previous_diff_point_id)
            }
        }
    };
}

impl_has_incremental_snapshot_tables!(
    BreakCount,
    break_count_full_snapshot_point,
    break_count_full_snapshot,
    break_count_diff_point,
    break_count_diff
);

impl_has_incremental_snapshot_tables!(
    BuildCount,
    build_count_full_snapshot_point,
    build_count_full_snapshot,
    build_count_diff_point,
    build_count_diff
);

impl_has_incremental_snapshot_tables!(
    PlayTicks,
    play_ticks_full_snapshot_point,
    play_ticks_full_snapshot,
    play_ticks_diff_point,
    play_ticks_diff
);

impl_has_incremental_snapshot_tables!(
    VoteCount,
    vote_count_full_snapshot_point,
    vote_count_full_snapshot,
    vote_count_diff_point,
    vote_count_diff
);
