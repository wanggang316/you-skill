//! Native WebView context menu policy.
//!
//! Release builds hide the native menu (Back / Reload, and on WebView2 also Save as / Print /
//! Inspect) on non-editable targets, but keep the editing menu (Cut / Copy / Paste) in text
//! fields. Debug builds keep the native menu everywhere for inspection. The gate is
//! `debug_assertions`, not the frontend build mode, because `tauri build --debug` also runs a
//! production frontend build.

/// Prevents the default `contextmenu` action unless the target is editable. It listens in the
/// capture phase and does not stop propagation, so page handlers still receive the event.
const SUPPRESS_SCRIPT: &str = r#"(() => {
  window.addEventListener(
    "contextmenu",
    (event) => {
      const target = event.composedPath()[0];
      if (
        target instanceof Element &&
        (target.closest("input, textarea") ||
          (target instanceof HTMLElement && target.isContentEditable))
      ) {
        return;
      }
      event.preventDefault();
    },
    true,
  );
})();"#;

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
    assert_eq!(init_script().is_some(), !cfg!(debug_assertions));
  }

  #[test]
  fn suppress_script_keeps_editable_targets() {
    assert!(SUPPRESS_SCRIPT.contains(r#"closest("input, textarea")"#));
    assert!(SUPPRESS_SCRIPT.contains("isContentEditable"));
    assert!(SUPPRESS_SCRIPT.contains("event.preventDefault()"));
  }
}
