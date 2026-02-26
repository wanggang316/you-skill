pub fn normalize_optional_string(value: Option<String>) -> Option<String> {
  value.and_then(|v| {
    let trimmed = v.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  })
}
