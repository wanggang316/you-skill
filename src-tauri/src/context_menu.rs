//! Native WebView context menu policy.
//!
//! Release builds hide the native menu (Back / Reload, and on WebView2 also Save as / Print /
//! Inspect) on non-editable targets, but keep the editing menu (Cut / Copy / Paste) in text
//! fields. Debug builds keep the native menu everywhere for inspection. The gate is
//! `debug_assertions`, not the frontend build mode, because `tauri build --debug` also runs a
//! production frontend build. The script behavior is tested in `tests/context-menu.test.mjs`.

const SUPPRESS_SCRIPT: &str = include_str!("suppress_context_menu.js");

/// The init script to add to app webviews, or `None` when the native menu stays enabled.
pub fn init_script() -> Option<&'static str> {
  if cfg!(debug_assertions) {
    None
  } else {
    Some(SUPPRESS_SCRIPT)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn init_script_follows_build_type() {
    assert_eq!(
      init_script(),
      (!cfg!(debug_assertions)).then_some(SUPPRESS_SCRIPT)
    );
  }
}
