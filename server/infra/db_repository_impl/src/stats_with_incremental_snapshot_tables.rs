use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use crate::cycle_free_path::construct_cycle_free_path;
use crate::structures_embedded_in_rdb::{
    ComputeDiff, DiffPoint, DiffPointId, DiffSequence, FullSnapshotPoint, IdIndexedDiffPoints,
    SnapshotPoint,
};
use chrono::{DateTime, NaiveDateTime, Utc};

use domain::models::{Player, PlayerUuidString, StatsSnapshot};
use domain::repositories::TimeBasedSnapshotSearchCondition;
use TimeBasedSnapshotSearchCondition::NewestBefore;

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTables<DBConnection>: Sized + Eq + Clone {
    async fn create_full_snapshot_point(conn: &mut DBConnection) -> anyhow::Result<u64>;

    async fn insert_all_stats_at_full_snapshot_point(
        fresh_full_snapshot_point_id: u64,
        player_stats: HashMap<Player, Self>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<()>;

    async fn create_diff_snapshot_point(
        base_point_id: u64,
        previous_diff_point_id_: Option<DiffPointId>,
        timestamp: DateTime<Utc>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<DiffPointId>;

    async fn insert_all_stats_at_diff_snapshot_point(
        fresh_diff_snapshot_point_id: DiffPointId,
        player_stats_diffs: HashMap<PlayerUuidString, Self>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<()>;

    async fn read_full_snapshot_point(
        full_snapshot_point_id: u64,
        conn: &mut DBConnection,
    ) -> anyhow::Result<FullSnapshotPoint<Self>>;

    async fn read_diff_snapshot_points(
        diff_snapshot_point_ids: HashSet<DiffPointId>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>>;

    async fn read_diff_snapshot_points_over_full_point(
        full_snapshot_point_id: u64,
        conn: &mut DBConnection,
    ) -> anyhow::Result<IdIndexedDiffPoints<Self>>;

    async fn find_id_and_timestamp_of_full_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut DBConnection,
    ) -> anyhow::Result<Option<(u64, NaiveDateTime)>>;

    async fn find_id_and_timestamp_of_diff_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut DBConnection,
    ) -> anyhow::Result<Option<(DiffPointId, NaiveDateTime)>>;

    async fn find_id_of_latest_full_snapshot_before(
        timestamp: DateTime<Utc>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<Option<u64>>;

    async fn id_of_root_full_snapshot_of_diff_point(
        diff_point_id: DiffPointId,
        conn: &mut DBConnection,
    ) -> anyhow::Result<u64>;

    async fn diff_point_id_to_previous_diff_point_id(
        forest_base_full_snapshot_point_id: u64,
        timestamp_upper_bound: DateTime<Utc>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<HashMap<DiffPointId, Option<DiffPointId>>>;
}

#[async_trait::async_trait]
pub trait HasIncrementalSnapshotTablesDefaultMethods<DBConnection: Send>:
    HasIncrementalSnapshotTables<DBConnection> + Debug
{
    #[tracing::instrument(skip(conn))]
    async fn create_full_snapshot(
        snapshot: StatsSnapshot<Self>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<()> {
        let inserted_point_id = Self::create_full_snapshot_point(conn).await?;

        Self::insert_all_stats_at_full_snapshot_point(
            inserted_point_id,
            snapshot.player_stats,
            conn,
        )
        .await
    }

    #[tracing::instrument(skip(conn))]
    async fn find_latest_full_snapshot_before(
        timestamp: DateTime<Utc>,
        conn: &mut DBConnection,
    ) -> anyhow::Result<Option<FullSnapshotPoint<Self>>> {
        if let Some(id) = Self::find_id_of_latest_full_snapshot_before(timestamp, conn).await? {
            let full_snapshot = Self::read_full_snapshot_point(id, conn).await?;
            Ok(Some(full_snapshot))
        } else {
            Ok(None)
        }
    }

    #[tracing::instrument(skip(conn))]
    async fn find_snapshot_point_with_condition(
        time_based_condition: TimeBasedSnapshotSearchCondition,
        conn: &mut DBConnection,
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

    #[tracing::instrument(skip(conn))]
    async fn create_diff_snapshot_point_on(
        diff_sequence: DiffSequence<Self>,
        snapshot: StatsSnapshot<Self>,
        conn: &mut DBConnection,
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

    #[tracing::instrument(skip(conn))]
    async fn construct_diff_sequence_leading_up_to(
        snapshot_point: SnapshotPoint<Self>,
        conn: &mut DBConnection,
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

    #[tracing::instrument(skip(conn))]
    async fn construct_diff_sequence_leading_up_to_diff_point(
        diff_point: DiffPoint<Self>,
        conn: &mut DBConnection,
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

impl<T: Debug + HasIncrementalSnapshotTables<DBConnection>, DBConnection: Send>
    HasIncrementalSnapshotTablesDefaultMethods<DBConnection> for T
{
}
