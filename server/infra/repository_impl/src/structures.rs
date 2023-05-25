use std::collections::HashMap;

use chrono::{DateTime, Utc};
use domain::models::{Player, PlayerUuidString, StatsSnapshot};

pub struct FullSnapshotPoint<Stats> {
    pub id: u64,
    pub full_snapshot: StatsSnapshot<Stats>,
}

pub struct SnapshotDiff<Stats> {
    pub utc_timestamp: DateTime<Utc>,
    pub player_stats_diffs: HashMap<PlayerUuidString, Stats>,
}

pub trait ComputeDiff {
    type Diff;

    fn diff_to(self, other: Self) -> Self::Diff;
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
}

pub struct DiffPoint<Stats> {
    pub id: u64,
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

impl<Stats> DiffSequence<Stats> {
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

    pub fn into_snapshot_at_the_end(self) -> StatsSnapshot<Stats> {
        let end_timestamp = if self.diff_points.is_empty() {
            self.base_point.full_snapshot.utc_timestamp
        } else {
            self.diff_points.last().unwrap().diff.utc_timestamp
        };

        let mut restored_stats = self.base_point.full_snapshot.player_stats;

        for diff_point in self.diff_points {
            for (player_uuid, diff) in diff_point.diff.player_stats_diffs {
                restored_stats.insert(Player { uuid: player_uuid }, diff);
            }
        }

        StatsSnapshot {
            utc_timestamp: end_timestamp,
            player_stats: restored_stats,
        }
    }

    pub fn is_sufficiently_short_to_extend(&self) -> bool {
        self.diff_points.len() <= 1000
    }
}
