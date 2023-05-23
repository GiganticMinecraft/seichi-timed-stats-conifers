use std::collections::HashMap;

use chrono::{DateTime, Utc};
use derive_more::{From, Into};

use super::Player;

#[derive(Debug, Clone, From, Into)]
pub struct StatsSnapshot<PlayerStats> {
    pub utc_timestamp: DateTime<Utc>,
    pub player_stats: HashMap<Player, PlayerStats>,
}
