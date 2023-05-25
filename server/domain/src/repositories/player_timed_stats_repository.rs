use crate::models::StatsSnapshot;
use chrono::{DateTime, Utc};

pub enum TimeBasedSnapshotSearchCondition {
    NewestBefore(DateTime<Utc>),
    OldestAfter(DateTime<Utc>),
}

#[async_trait::async_trait]
pub trait PlayerTimedStatsRepository<PlayerStats> {
    async fn record_snapshot(&self, snapshot: StatsSnapshot<PlayerStats>) -> anyhow::Result<()>;

    async fn search_snapshot(
        &self,
        condition: TimeBasedSnapshotSearchCondition,
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;
}
