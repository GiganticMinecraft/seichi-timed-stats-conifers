use crate::types::TimeStamped;

pub struct StatsSnapshot<PlayerStats>(TimeStamped<Vec<PlayerStats>>);
