use chrono::prelude::*;
pub fn get_timestamp() -> i64 {
    Utc::now().timestamp()
}

pub fn get_timestamp_millis() -> i64 {
    Utc::now().timestamp_millis()
}
