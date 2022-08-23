use crate::models::{
    PlayerBreakCount, PlayerBuildCount, PlayerPlayTicks, PlayerVoteCount, StatsSnapshot,
};

#[async_trait::async_trait]
pub trait PlayerStatsRepository<PlayerStats> {
    async fn fetch_stats_snapshot_of_all_players(&self) -> StatsSnapshot<PlayerStats>;
}

pub trait PlayerBreakCountRepository: PlayerStatsRepository<PlayerBreakCount> {}
impl<T> PlayerBreakCountRepository for T where T: PlayerStatsRepository<PlayerBreakCount> {}

pub trait PlayerBuildCountRepository: PlayerStatsRepository<PlayerBuildCount> {}
impl<T> PlayerBuildCountRepository for T where T: PlayerStatsRepository<PlayerBuildCount> {}

pub trait PlayerVoteCountRepository: PlayerStatsRepository<PlayerVoteCount> {}
impl<T> PlayerVoteCountRepository for T where T: PlayerStatsRepository<PlayerVoteCount> {}

pub trait PlayerPlayTicksRepository: PlayerStatsRepository<PlayerPlayTicks> {}
impl<T> PlayerPlayTicksRepository for T where T: PlayerStatsRepository<PlayerPlayTicks> {}
