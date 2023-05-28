use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use domain::models::{Player, PlayerUuidString, StatsSnapshot};
use ordered_float::OrderedFloat;

#[derive(Clone)]
pub struct FullSnapshotPoint<Stats> {
    pub id: u64,
    pub full_snapshot: StatsSnapshot<Stats>,
}

pub struct SnapshotDiff<Stats> {
    pub utc_timestamp: DateTime<Utc>,
    pub player_stats_diffs: HashMap<PlayerUuidString, Stats>,
}

impl<Stats: Clone> SnapshotDiff<Stats> {
    pub fn apply_to(&self, base_snapshot: StatsSnapshot<Stats>) -> StatsSnapshot<Stats> {
        let mut base_snapshot = base_snapshot;
        self.apply_to_mut(&mut base_snapshot);

        StatsSnapshot {
            utc_timestamp: self.utc_timestamp,
            player_stats: base_snapshot.player_stats,
        }
    }

    pub fn apply_to_mut(&self, base_snapshot: &mut StatsSnapshot<Stats>) {
        for (player_uuid, diff) in &self.player_stats_diffs {
            base_snapshot.player_stats.insert(
                Player {
                    uuid: player_uuid.clone(),
                },
                diff.clone(),
            );
        }
    }
}

pub trait ComputeDiff {
    type Diff;

    fn diff_to(self, other: Self) -> Self::Diff;
    fn size_of_diff_to(&self, other: &Self) -> usize;
}

impl<Stats: Eq + Clone> ComputeDiff for StatsSnapshot<Stats> {
    type Diff = SnapshotDiff<Stats>;

    fn diff_to(self, other: Self) -> Self::Diff {
        let mut player_stats_diffs = HashMap::new();

        for (player, stats) in other.player_stats {
            if Some(stats.clone()) != self.player_stats.get(&player).cloned() {
                player_stats_diffs.insert(player.uuid.clone(), stats);
            }
        }

        SnapshotDiff {
            utc_timestamp: other.utc_timestamp,
            player_stats_diffs,
        }
    }

    fn size_of_diff_to(&self, other: &Self) -> usize {
        let players = self
            .player_stats
            .keys()
            .chain(other.player_stats.keys())
            .collect::<HashSet<_>>();

        players
            .into_iter()
            .filter(|player| self.player_stats.get(player) != other.player_stats.get(player))
            .count()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, diesel::AsExpression, diesel::FromSqlRow, Debug)]
#[diesel(sql_type = diesel::sql_types::Unsigned<diesel::sql_types::BigInt>)]
pub struct DiffPointId(pub u64);

pub struct DiffPoint<Stats> {
    pub id: DiffPointId,
    pub previous_diff_point_id: Option<DiffPointId>,
    pub diff: SnapshotDiff<Stats>,
}

pub enum SnapshotPoint<Stats> {
    Full(FullSnapshotPoint<Stats>),
    Diff(DiffPoint<Stats>),
}

pub struct DiffSequence<Stats> {
    pub base_point: FullSnapshotPoint<Stats>,
    pub diff_points: Vec<DiffPoint<Stats>>,
}

impl<Stats: Clone> DiffSequence<Stats> {
    pub fn without_any_diffs(base_point: FullSnapshotPoint<Stats>) -> Self {
        Self {
            base_point,
            diff_points: Vec::new(),
        }
    }

    pub fn new(base_point: FullSnapshotPoint<Stats>, diff_points: Vec<DiffPoint<Stats>>) -> Self {
        Self {
            base_point,
            diff_points,
        }
    }

    pub fn into_snapshot_at_the_tip(self) -> StatsSnapshot<Stats> {
        if self.diff_points.is_empty() {
            self.base_point.full_snapshot
        } else {
            let mut updated_snapshot = self.base_point.full_snapshot;
            for diff_point in self.diff_points {
                updated_snapshot = diff_point.diff.apply_to(updated_snapshot);
            }
            updated_snapshot
        }
    }

    fn len(&self) -> usize {
        self.diff_points.len() + 1
    }
}

pub enum DiffSequenceChoice<Stats> {
    OptimalAccordingToHeuristics(DiffSequence<Stats>),
    NoAppropriatePointFound,
}

fn choose_sub_diff_sequence_for_snapshot_with_heuristics<Stats: Clone + Eq>(
    sequence: DiffSequence<Stats>,
    snapshot: &StatsSnapshot<Stats>,
) -> DiffSequence<Stats> {
    /// `snapshot` に対する、 `sequence` 内の diff point の損失関数。
    /// この損失関数は、 `sequence` 内の diff point のうち、
    ///  - `snapshot` にできるだけ近く、
    ///  - 復元時のコストができるだけ低く
    /// なるようなものを選ぶために利用する。
    fn loss_function(size_at_depth: &DiffSizeAtParticularDepth) -> OrderedFloat<f64> {
        let DiffSizeAtParticularDepth {
            diff_sequence_depth: depth,
            total_diffs_on_current_diff_sequence: total_diffs,
            diffs_from_tip_to_snapshot: diffs,
        } = size_at_depth;

        OrderedFloat::from(((diffs + 1) as f64) * ((total_diffs + depth + 1) as f64).log(20.0))
    }

    let base_point = sequence.base_point;
    let diff_points = sequence.diff_points;

    struct ScanState<Stats> {
        current_snapshot: StatsSnapshot<Stats>,
        current_diff_sequence_depth: usize,
        current_total_diff_size: usize,
    }

    impl<Stats> ScanState<Stats> {
        fn new(base_point: FullSnapshotPoint<Stats>) -> Self {
            Self {
                current_snapshot: base_point.full_snapshot,
                current_diff_sequence_depth: 0,
                current_total_diff_size: 0,
            }
        }
    }

    // 特定の diff point の (`sequence` と `snapshot` に基づいた) 情報のうち、
    // 損失関数を計算するのに必要なものを集めたもの。
    struct DiffSizeAtParticularDepth {
        /// その diff point の `sequence` 内のインデックス。
        diff_sequence_depth: usize,
        /// `sequence.take(diff_sequence_depth)` に含まれる `player_stats_diffs` の総数。
        total_diffs_on_current_diff_sequence: usize,
        /// `sequence.take(diff_sequence_depth).into_snapshot_at_the_tip()` と
        /// `snapshot` の差分の `player_stats_diffs` の総数。
        diffs_from_tip_to_snapshot: usize,
    }

    let virtual_empty_diff_at_base = SnapshotDiff {
        utc_timestamp: base_point.full_snapshot.utc_timestamp,
        player_stats_diffs: HashMap::new(),
    };

    let optimal_depth_size_pair = std::iter::once(&virtual_empty_diff_at_base)
        .chain(diff_points.iter().map(|diff_point| &diff_point.diff))
        .scan(ScanState::new(base_point.clone()), |state, diff| {
            diff.apply_to_mut(&mut state.current_snapshot);
            state.current_diff_sequence_depth += 1;
            state.current_total_diff_size += diff.player_stats_diffs.len();

            Some(DiffSizeAtParticularDepth {
                diff_sequence_depth: state.current_diff_sequence_depth,
                total_diffs_on_current_diff_sequence: state.current_total_diff_size,
                diffs_from_tip_to_snapshot: state.current_snapshot.size_of_diff_to(&snapshot),
            })
        })
        .min_by_key(loss_function)
        .unwrap();

    DiffSequence {
        base_point,
        diff_points: diff_points
            .into_iter()
            .take(optimal_depth_size_pair.diff_sequence_depth)
            .collect(),
    }
}

pub struct IdIndexedDiffPoints<Stats>(HashMap<DiffPointId, DiffPoint<Stats>>);

impl<Stats: Clone> IdIndexedDiffPoints<Stats> {
    pub fn new(diff_points: Vec<DiffPoint<Stats>>) -> Self {
        Self(
            diff_points
                .into_iter()
                .map(|diff_point| (diff_point.id, diff_point))
                .collect(),
        )
    }

    fn size(&self) -> usize {
        self.0.len()
    }

    fn latest(&self) -> Option<&DiffPoint<Stats>> {
        self.0
            .values()
            .max_by_key(|diff_point| diff_point.diff.utc_timestamp)
    }

    fn unsafe_get(&self, id: DiffPointId) -> &DiffPoint<Stats> {
        self.0.get(&id).unwrap()
    }

    pub fn points_before(self, timestamp: DateTime<Utc>) -> Self {
        Self(
            self.0
                .into_iter()
                .filter(|(_, diff_point)| diff_point.diff.utc_timestamp < timestamp)
                .collect(),
        )
    }

    pub fn remove(&mut self, id: &DiffPointId) -> Option<DiffPoint<Stats>> {
        self.0.remove(id)
    }

    pub fn map_ids_to_diff_points(mut self, ids: &[DiffPointId]) -> Vec<DiffPoint<Stats>> {
        ids.iter().map(|id| self.0.remove(&id).unwrap()).collect()
    }

    fn diff_sequence_towards_latest_diff_point(
        self,
        base_point: FullSnapshotPoint<Stats>,
    ) -> anyhow::Result<DiffSequence<Stats>> {
        let ids_of_diff_points_towards_base_point =
            crate::cycle_free_path::construct_cycle_free_path(self.latest().unwrap().id, |id| {
                self.unsafe_get(id).previous_diff_point_id
            })?;

        let diff_points_towards_latest_point = {
            let mut ids = ids_of_diff_points_towards_base_point;
            ids.reverse();
            self.map_ids_to_diff_points(&ids)
        };

        Ok(DiffSequence::new(
            base_point,
            diff_points_towards_latest_point,
        ))
    }
}

pub fn choose_base_diff_sequence_for_snapshot_with_heuristics<Stats: Clone + Eq>(
    base_point: FullSnapshotPoint<Stats>,
    all_diff_points_over_base_point: IdIndexedDiffPoints<Stats>,
    snapshot: &StatsSnapshot<Stats>,
) -> anyhow::Result<DiffSequenceChoice<Stats>> {
    // クエリ速度を最適化する場合、 diff sequence 内のトータルの diff レコード数が最小になるようにすればよい。
    // しかし、ストレージを最小化するには、diff レコード数を少しだけ増やしても sequence を伸ばすべきである。
    //
    // 例えば、初期の full snapshot が `F = { a: 1, b: 1, c: 1 }` であり、
    // `F` の後続の diff point として `D = { a: 2, b: 2 }` があるとする。
    // ここで、 `snapshot = { a: 3, b: 2, c: 1 }` の base diff sequence を選択する場合、
    //  - diff sequence 上のレコード数を最小化すると `[F]` が、
    //  - 追加する diff レコードが最小になるようにすると、 `[F, D]` が
    // 選択される。というのも、`snapshot` までの diff sequence は
    //  - `[F]` を base diff sequence とした時、 `[F, { a: 3, b: 2 }]` であり、
    //    この diff sequence 内の diff レコード数は 2、追加する diff レコード数は 2 である一方、
    //  - `[F, D]` を base diff sequence とした時、 `[F, D, { a: 3 }]` であり、
    //    この diff sequence 内の diff レコード数は 3、追加する diff レコード数は 1 であるためである。
    //
    // このように、一般には diff sequence の小ささと追加する diff の小ささはトレードオフの関係にある。
    // 当アプリケーションは read-intensive ではないためストレージの最小化の方を優先するが、
    // かといって diff sequence の大きさで full snapshot の頻度を決定するため、
    // diff sequence の大きさを無視するわけにもいかない。
    //
    // そこで、 `choose_sub_diff_sequence_for_snapshot_with_heuristics` 内の `loss_function` によって、
    //  - diff sequence の大きさ (長さと、diff sequence 内の合計 diff レコード数)
    //  - 追加する diff レコード数
    // の両方を加味した「損失」を定義し、これを最小化するように基底の diff point を採用するようにする。
    //
    // 基底の full snapshot point `base_point` 上のすべての diff point `d` について
    // 損失を計算するのはコストが高いため、近似解として、最新の diff point の
    // 祖先のうち、最も損失が小さいものを基底の diff point とすることとした。

    {
        let diff_points_count = all_diff_points_over_base_point.size();

        // diff point が存在しない場合は full snapshot 上に diff point を作成すればよい。
        if diff_points_count == 0 {
            return Ok(DiffSequenceChoice::OptimalAccordingToHeuristics(
                DiffSequence::new(base_point, vec![]),
            ));
        }

        // diff point がすでに多すぎる場合は、 diff point の追加を諦める
        // (上位の処理は、代わりに full snapshot を作成するはず)。
        if diff_points_count > 2500 {
            return Ok(DiffSequenceChoice::NoAppropriatePointFound);
        }
    }

    let optimal_sub_diff_sequence_towards_latest_point =
        choose_sub_diff_sequence_for_snapshot_with_heuristics(
            all_diff_points_over_base_point.diff_sequence_towards_latest_diff_point(base_point)?,
            snapshot,
        );

    // 最良と判断された diff sequence が十分に長ければ、
    // diff point の追加を諦める (上位の処理は、代わりに full snapshot を作成するはず)。
    if optimal_sub_diff_sequence_towards_latest_point.len() > 1000 {
        return Ok(DiffSequenceChoice::NoAppropriatePointFound);
    }

    Ok(DiffSequenceChoice::OptimalAccordingToHeuristics(
        optimal_sub_diff_sequence_towards_latest_point,
    ))
}
