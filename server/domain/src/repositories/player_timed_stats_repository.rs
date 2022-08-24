use crate::models::{PlayerUuidString, StatsSnapshot};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ReadFilter {
    Unlimited,
    OnlyPlayersWithUuidIn(HashSet<PlayerUuidString>),
}

#[async_trait::async_trait]
pub trait PlayerTimedStatsRepository<PlayerStats> {
    async fn record_snapshot(&self, snapshot: StatsSnapshot<PlayerStats>) -> anyhow::Result<()>;

    async fn read_latest_stats_snapshot_before(
        &self,
        timestamp: DateTime<Utc>,
        filter: ReadFilter,
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;

    async fn read_first_stats_snapshot_after(
        &self,
        timestamp: DateTime<Utc>,
        filter: ReadFilter,
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;
}
