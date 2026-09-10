pub fn now_rfc3339() -> String {
  chrono::Utc::now().to_rfc3339()
}

/// Compact timestamp for file names, e.g. `20260903-153012`.
pub fn now_file_stamp() -> String {
  chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}
