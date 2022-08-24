use crate::models::StatsSnapshot;

#[async_trait::async_trait]
pub trait PlayerStatsRepository<PlayerStats> {
    async fn fetch_stats_snapshot_of_all_players(&self) -> StatsSnapshot<PlayerStats>;
}
