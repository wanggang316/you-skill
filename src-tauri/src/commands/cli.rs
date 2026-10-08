//! The bundled `youskill` command line tool: where it is, and putting it on the PATH.
//! The binary ships next to the app executable (`bundle.externalBin`); on macOS and
//! Linux it is linked into `/usr/local/bin`, on Windows it is copied into a per-user
//! directory that is added to the user PATH.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct CliStatus {
  /// The binary shipped with this build; `None` when the build has none.
  pub bundled_path: Option<String>,
  /// Where the tool is (or would be) installed.
  pub install_path: String,
  pub installed: bool,
  /// The installed tool is this build's binary (a link to it, or an identical copy).
  pub current: bool,
}

const BINARY: &str = if cfg!(windows) {
  "youskill.exe"
} else {
  "youskill"
};

fn bundled_binary() -> Option<PathBuf> {
  let exe = std::env::current_exe().ok()?;
  let candidate = exe.parent()?.join(BINARY);
  candidate.is_file().then_some(candidate)
}

fn install_path() -> Result<PathBuf, String> {
  if cfg!(windows) {
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
    Ok(
      PathBuf::from(local)
        .join("YouSkill")
        .join("bin")
        .join(BINARY),
    )
  } else {
    Ok(PathBuf::from("/usr/local/bin").join(BINARY))
  }
}

fn status() -> Result<CliStatus, String> {
  let bundled = bundled_binary();
  let install = install_path()?;
  let installed = install.exists() || install.is_symlink();
  let current = match (&bundled, installed) {
    (Some(bundled), true) => points_to(&install, bundled),
    _ => false,
  };
  Ok(CliStatus {
    bundled_path: bundled.map(|p| p.to_string_lossy().to_string()),
    install_path: install.to_string_lossy().to_string(),
    installed,
    current,
  })
}

/// A symlink to `bundled`, or a byte-identical copy of it.
fn points_to(install: &Path, bundled: &Path) -> bool {
  if let Ok(target) = fs::read_link(install) {
    return same_file(&target, bundled);
  }
  match (fs::read(install), fs::read(bundled)) {
    (Ok(a), Ok(b)) => a == b,
    _ => false,
  }
}

fn same_file(a: &Path, b: &Path) -> bool {
  match (a.canonicalize(), b.canonicalize()) {
    (Ok(a), Ok(b)) => a == b,
    _ => a == b,
  }
}

#[tauri::command]
pub fn cli_status() -> Result<CliStatus, String> {
  status()
}

#[tauri::command]
pub fn install_cli() -> Result<CliStatus, String> {
  let bundled = bundled_binary().ok_or("This build does not include the command line tool")?;
  let install = install_path()?;
  platform::install(&bundled, &install)?;
  status()
}

#[tauri::command]
pub fn uninstall_cli() -> Result<CliStatus, String> {
  let install = install_path()?;
  if install.exists() || install.is_symlink() {
    platform::remove(&install)?;
  }
  status()
}

#[cfg(unix)]
mod platform {
  use std::fs;
  use std::io::ErrorKind;
  use std::os::unix::fs::symlink;
  use std::path::Path;
  use std::process::Command;

  pub fn install(bundled: &Path, install: &Path) -> Result<(), String> {
    match link(bundled, install) {
      Ok(()) => Ok(()),
      Err(err) if err.kind() == ErrorKind::PermissionDenied => privileged(&format!(
        "mkdir -p '{}' && ln -sf '{}' '{}'",
        parent(install),
        bundled.display(),
        install.display()
      )),
      Err(err) => Err(format!("Failed to link {}: {}", install.display(), err)),
    }
  }

  pub fn remove(install: &Path) -> Result<(), String> {
    match fs::remove_file(install) {
      Ok(()) => Ok(()),
      Err(err) if err.kind() == ErrorKind::PermissionDenied => {
        privileged(&format!("rm -f '{}'", install.display()))
      },
      Err(err) => Err(format!("Failed to remove {}: {}", install.display(), err)),
    }
  }

  fn link(bundled: &Path, install: &Path) -> std::io::Result<()> {
    if let Some(dir) = install.parent() {
      fs::create_dir_all(dir)?;
    }
    if install.exists() || install.is_symlink() {
      fs::remove_file(install)?;
    }
    symlink(bundled, install)
  }

  fn parent(path: &Path) -> String {
    path
      .parent()
      .map(|p| p.display().to_string())
      .unwrap_or_default()
  }

  /// Run a shell command with administrator rights through the system prompt (macOS);
  /// elsewhere tell the user what to run.
  fn privileged(script: &str) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
      return Err(format!("Permission denied. Run: sudo sh -c \"{}\"", script));
    }
    let output = Command::new("osascript")
      .arg("-e")
      .arg(format!(
        "do shell script \"{}\" with administrator privileges",
        script.replace('\\', "\\\\").replace('"', "\\\"")
      ))
      .output()
      .map_err(|e| format!("Failed to run osascript: {}", e))?;
    if output.status.success() {
      Ok(())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      if stderr.contains("User canceled") || stderr.contains("(-128)") {
        Err("Cancelled".to_string())
      } else {
        Err(format!("Failed to install: {}", stderr.trim()))
      }
    }
  }
}

#[cfg(windows)]
mod platform {
  use std::fs;
  use std::path::Path;
  use std::process::Command;

  pub fn install(bundled: &Path, install: &Path) -> Result<(), String> {
    let dir = install
      .parent()
      .ok_or("Install path has no parent directory")?;
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    fs::copy(bundled, install).map_err(|e| format!("Failed to copy to {}: {}", install.display(), e))?;
    add_to_user_path(&dir.display().to_string())
  }

  pub fn remove(install: &Path) -> Result<(), String> {
    fs::remove_file(install).map_err(|e| format!("Failed to remove {}: {}", install.display(), e))
  }

  /// Append `dir` to the user PATH in the registry (new terminals pick it up). `setx` is
  /// avoided because it truncates long values.
  fn add_to_user_path(dir: &str) -> Result<(), String> {
    let script = format!(
      "$p = [Environment]::GetEnvironmentVariable('Path', 'User'); \
       if (($p -split ';') -notcontains '{dir}') {{ \
         [Environment]::SetEnvironmentVariable('Path', (($p.TrimEnd(';')) + ';{dir}'), 'User') }}",
      dir = dir.replace('\'', "''")
    );
    let output = Command::new("powershell")
      .args(["-NoProfile", "-NonInteractive", "-Command", &script])
      .output()
      .map_err(|e| format!("Failed to run powershell: {}", e))?;
    if output.status.success() {
      Ok(())
    } else {
      Err(format!(
        "Failed to update PATH: {}",
        String::from_utf8_lossy(&output.stderr).trim()
      ))
    }
  }
}
