use crate::models::{
    PlayerBreakCount, PlayerBuildCount, PlayerPlayTicks, PlayerUuidString, PlayerVoteCount,
};
use crate::types::StatsSnapshot;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

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
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;

    async fn read_first_stats_snapshot_after(
        &self,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<Option<StatsSnapshot<PlayerStats>>>;
}

pub trait PlayerTimedBreakCountRepository: PlayerTimedStatsRepository<PlayerBreakCount> {}
impl<T> PlayerTimedBreakCountRepository for T where T: PlayerTimedStatsRepository<PlayerBreakCount> {}

pub trait PlayerTimedBuildCountRepository: PlayerTimedStatsRepository<PlayerBuildCount> {}
impl<T> PlayerTimedBuildCountRepository for T where T: PlayerTimedStatsRepository<PlayerBuildCount> {}

pub trait PlayerTimedVoteCountRepository: PlayerTimedStatsRepository<PlayerVoteCount> {}
impl<T> PlayerTimedVoteCountRepository for T where T: PlayerTimedStatsRepository<PlayerVoteCount> {}

pub trait PlayerTimedPlayTicksRepository: PlayerTimedStatsRepository<PlayerPlayTicks> {}
impl<T> PlayerTimedPlayTicksRepository for T where T: PlayerTimedStatsRepository<PlayerPlayTicks> {}
