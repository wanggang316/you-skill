use std::time::{SystemTime, UNIX_EPOCH};

pub fn to_millis(time: SystemTime) -> Option<i64> {
  time
    .duration_since(UNIX_EPOCH)
    .ok()
    .map(|duration| duration.as_millis() as i64)
}
