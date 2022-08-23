use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct TimeStamped<T> {
    pub data: T,
    pub utc_timestamp: DateTime<Utc>,
}
