use std::collections::HashMap;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use derive_more::{From, Into};

use super::Player;

#[derive(Clone, From, Into)]
pub struct StatsSnapshot<PlayerStats> {
    pub utc_timestamp: DateTime<Utc>,
    pub player_stats: HashMap<Player, PlayerStats>,
}

impl<Stats> StatsSnapshot<Stats> {
    pub fn len(&self) -> usize {
        self.player_stats.len()
    }
}

impl<Stats> Debug for StatsSnapshot<Stats> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatsSnapshot")
            .field("player_stats_count", &self.len())
            .finish()
    }
}
