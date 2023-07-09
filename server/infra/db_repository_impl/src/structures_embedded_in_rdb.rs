use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use domain::models::{Player, PlayerUuidString, StatsSnapshot};
use ordered_float::OrderedFloat;

#[derive(Clone)]
pub struct FullSnapshotPoint<Stats> {
    pub id: u64,
    pub full_snapshot: StatsSnapshot<Stats>,
}

impl<Stats: Debug> Debug for FullSnapshotPoint<Stats> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullSnapshotPoint")
            .field("id", &self.id)
            .field("player_stats_count", &self.full_snapshot.len())
            .finish()
    }
}

pub struct SnapshotDiff<Stats> {
    pub utc_timestamp: DateTime<Utc>,
    pub player_stats_diffs: HashMap<PlayerUuidString, Stats>,
}

impl<Stats: Debug> Debug for SnapshotDiff<Stats> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotDiff")
            .field("player_stats_diffs_count", &self.player_stats_diffs.len())
            .finish()
    }
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
            base_snapshot
                .player_stats
                .insert(Player { uuid: *player_uuid }, diff.clone());
        }
    }
}

pub trait ComputeDiff {
    type Diff;

    // TODO: rename to `diff_from` and flip arguments
    fn diff_to(&self, other: &Self) -> Self::Diff;
    fn size_of_diff_to(&self, other: &Self) -> usize;
}

impl<Stats: Eq + Clone> ComputeDiff for StatsSnapshot<Stats> {
    type Diff = SnapshotDiff<Stats>;

    fn diff_to(&self, other: &Self) -> Self::Diff {
        let mut player_stats_diffs = HashMap::new();

        for (player, stats) in &other.player_stats {
            if Some(stats) != self.player_stats.get(&player) {
                player_stats_diffs.insert(player.uuid, stats.clone());
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

#[derive(Debug)]
pub struct DiffPoint<Stats> {
    pub id: DiffPointId,
    pub previous_diff_point_id: Option<DiffPointId>,
    pub diff: SnapshotDiff<Stats>,
}

#[derive(Debug)]
pub enum SnapshotPoint<Stats> {
    Full(FullSnapshotPoint<Stats>),
    Diff(DiffPoint<Stats>),
}

pub struct DiffSequence<Stats> {
    pub base_point: FullSnapshotPoint<Stats>,
    pub diff_points: Vec<DiffPoint<Stats>>,
}

impl<Stats: Debug> Debug for DiffSequence<Stats> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffSequence")
            .field("base_point", &self.base_point)
            .field("diff_points_count", &self.diff_points.len())
            .finish()
    }
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

#[tracing::instrument]
fn choose_sub_diff_sequence_for_snapshot_with_heuristics<Stats: Debug + Clone + Eq>(
    sequence: DiffSequence<Stats>,
    snapshot: &StatsSnapshot<Stats>,
) -> DiffSequence<Stats> {
    // `sequence` 内の特定の diff point の (`snapshot` に基づいた) 情報のうち、
    // 損失関数を計算するのに必要なものを集めたもの。
    struct LossFactorAtParticularDepthInSequence {
        /// その diff point の `sequence` 内のインデックス。
        diff_sequence_depth: usize,
        /// `sequence.take(diff_sequence_depth)` に含まれる `player_stats_diffs` の総数。
        total_diffs_on_current_diff_sequence: usize,
        /// `sequence.take(diff_sequence_depth)` から `snapshot` を記録するにあたって
        /// 発生する差分レコードの数 (の見積もり)。これは厳密な数である必要は無いため、
        /// 計算するにあたって統計データの単調増加性などを仮定することにする。
        diffs_required_to_extend_from_the_point: usize,
    }

    /// `snapshot` に対する、 `sequence` 内の diff point の損失関数。
    /// この損失関数は、 `sequence` 内の diff point のうち、
    ///  - `snapshot` にできるだけ近く、
    ///  - 復元時のコストができるだけ低く
    /// なるようなものを選ぶために利用する。
    fn loss_function(size_at_depth: &LossFactorAtParticularDepthInSequence) -> OrderedFloat<f64> {
        let LossFactorAtParticularDepthInSequence {
            diff_sequence_depth: depth,
            total_diffs_on_current_diff_sequence: total_diffs,
            diffs_required_to_extend_from_the_point: diffs,
        } = size_at_depth;

        OrderedFloat::from(
            ((diffs + 1) as f64) * ((diffs + total_diffs + depth + 1) as f64).log(20.0),
        )
    }

    let base_point = sequence.base_point;
    let diff_points = sequence.diff_points;

    struct ScanState<Stats> {
        current_stats_different_from_target_snapshot:
            HashMap<PlayerUuidString, /* value at target snapshot */ Stats>,
        current_diff_sequence_depth: usize,
        current_total_diff_size: usize,
    }

    impl<Stats: Eq + Clone> ScanState<Stats> {
        fn new_against_snapshots(
            base_full_snapshot: StatsSnapshot<Stats>,
            target_snapshot: StatsSnapshot<Stats>,
        ) -> Self {
            Self {
                current_stats_different_from_target_snapshot: target_snapshot
                    .diff_to(&base_full_snapshot)
                    .player_stats_diffs,
                current_diff_sequence_depth: 0,
                current_total_diff_size: 0,
            }
        }
    }

    let virtual_empty_diff_at_base = SnapshotDiff {
        utc_timestamp: base_point.full_snapshot.utc_timestamp,
        player_stats_diffs: HashMap::new(),
    };

    let optimal_depth_size_pair = std::iter::once(&virtual_empty_diff_at_base)
        .chain(diff_points.iter().map(|diff_point| &diff_point.diff))
        .scan(
            ScanState::new_against_snapshots(base_point.full_snapshot.clone(), snapshot.clone()),
            |state, snapshot_diff| {
                let diffs = &snapshot_diff.player_stats_diffs;

                state.current_diff_sequence_depth += 1;
                state.current_total_diff_size += diffs.len();

                // 更新が掛かった統計量が `snapshot` での値 (`target`) と一致している場合には
                // `state.current_stats_different_from_target_snapshot` から削除したい。
                //
                // note 1: 各データポイントについて差分を計算するのは O(NM) (N = プレーヤー数、M = 差分ポイント数) 程度掛かる。
                // 現実的な値として N = 50K、M=5K などを想定すると、この処理はそこそこ CPU time を食ってしまうことがわかる。
                // 統計量の単調増加性を仮定する場合、ある時点での `snapshot` との差分をすべて計算せずとも
                // 更新結果が `target` と一致するかを見るだけで `state.current_stats_different_from_target_snapshot` に
                // 残すべきデータなのかを判断できる。この処理は scan を通して O(L) (L = `sequence` 内の差分レコードの総数) 程度の計算で済む。
                //
                // note 2: この処理により、`state.current_stats_different_from_target_snapshot` のサイズは `.scan` 中で単調減少する。
                for (player_uuid, updated) in diffs {
                    let target = state
                        .current_stats_different_from_target_snapshot
                        .get(player_uuid);

                    if Some(updated) == target {
                        // note: 統計量の単調増加性により diff のサイズは state.current_stats_different_from_target_snapshot 以下のはずなので、
                        // state.current_stats_different_from_target_snapshot.retain するよりも .remove していった方が速い
                        state
                            .current_stats_different_from_target_snapshot
                            .remove(player_uuid);
                    }
                }

                Some(LossFactorAtParticularDepthInSequence {
                    diff_sequence_depth: state.current_diff_sequence_depth,
                    total_diffs_on_current_diff_sequence: state.current_total_diff_size,
                    diffs_required_to_extend_from_the_point: state
                        .current_stats_different_from_target_snapshot
                        .len(),
                })
            },
        )
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
        ids.iter().map(|id| self.0.remove(id).unwrap()).collect()
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

#[tracing::instrument(skip(all_diff_points_over_base_point))]
pub fn choose_base_diff_sequence_for_snapshot_with_heuristics<Stats: Debug + Clone + Eq>(
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
