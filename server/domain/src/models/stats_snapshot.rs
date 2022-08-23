use crate::types::TimeStamped;
use derive_more::{From, Into};

#[derive(Debug, Clone, From, Into)]
pub struct StatsSnapshot<PlayerStats>(TimeStamped<Vec<PlayerStats>>);
