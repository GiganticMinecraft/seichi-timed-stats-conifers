use chrono::{DateTime, Utc};

pub struct TimeStamped<T> {
    pub data: T,
    pub utc_timestamp: DateTime<Utc>,
}
