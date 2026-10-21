use crate::models::StatsSnapshot;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub enum TimeBasedSnapshotSearchCondition {
    NewestBefore(DateTime<Utc>),
    OldestAfter(DateTime<Utc>),
}

/// スナップショットをどの形式で記録したか
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordedSnapshotKind {
    /// 全プレーヤー分の値をそのまま記録した
    Full,
    /// 既存のスナップショットからの差分だけを記録した
    Diff,
}

impl RecordedSnapshotKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Diff => "diff",
        }
    }
}

/// [`PlayerTimedStatsRepository::record_snapshot`] の結果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedSnapshot {
    pub kind: RecordedSnapshotKind,
    /// 書き込んだプレーヤーごとの値の件数 (full なら全プレーヤー数、diff なら値が変わったプレーヤー数)
    pub recorded_entries: usize,
}

#[async_trait::async_trait]
pub trait PlayerTimedStatsRepository<PlayerStats> {
    async fn record_snapshot(
        &self,
        snapshot: StatsSnapshot<PlayerStats>,
    ) -> anyhow::Result<RecordedSnapshot>;

    async fn search_snapshot(
        &self,
        condition: TimeBasedSnapshotSearchCondition,
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;
}
