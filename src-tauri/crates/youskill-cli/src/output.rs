//! Output helpers shared by every command: the global flags, exit codes, JSON, tables and
//! confirmations.

use serde::Serialize;
use std::io::{self, BufRead, IsTerminal, Write};
use youskill_core::models::{ActionResult, SkillSource};

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
/// An action was refused because it would discard local changes, or a confirmation was
/// needed and stdin is not a terminal.
pub const EXIT_BLOCKED: i32 = 2;
/// `status --exit-code` found drift.
pub const EXIT_DRIFT: i32 = 3;

/// Global flags, passed to every command.
pub struct Ctx {
  pub json: bool,
  pub yes: bool,
  pub quiet: bool,
}

impl Ctx {
  /// Print a line unless `--quiet` or `--json` is set.
  pub fn say(&self, line: &str) {
    if !self.quiet && !self.json {
      println!("{}", line);
    }
  }

  /// Ask a yes/no question. `--yes` answers it; without a terminal on stdin the answer is
  /// no, and the caller reports why it stopped.
  pub fn confirm(&self, question: &str) -> bool {
    if self.yes {
      return true;
    }
    let stdin = io::stdin();
    if !stdin.is_terminal() {
      eprintln!("{} (needs a terminal or --yes)", question);
      return false;
    }
    eprint!("{} [y/N] ", question);
    let _ = io::stderr().flush();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
      return false;
    }
    matches!(line.trim(), "y" | "Y" | "yes" | "YES")
  }
}

pub fn error(ctx: &Ctx, message: &str) {
  if ctx.json {
    eprintln!("{}", serde_json::json!({ "error": message }));
  } else {
    eprintln!("error: {}", message);
  }
}

pub fn emit_json<T: Serialize>(value: &T) -> Result<(), String> {
  let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
  println!("{}", text);
  Ok(())
}

/// The serde name of a state enum (`in_sync`, `update_available`, ...), which is what the
/// app shows and what `--json` prints.
pub fn state_str<T: Serialize>(value: &T) -> String {
  match serde_json::to_value(value) {
    Ok(serde_json::Value::String(s)) => s,
    Ok(other) => other.to_string(),
    Err(_) => "?".to_string(),
  }
}

pub fn source_label(source: &SkillSource) -> String {
  match source {
    SkillSource::Github { repo, branch, .. } => match branch.as_deref() {
      Some(b) if !b.is_empty() => format!("{}@{}", repo, b),
      _ => repo.clone(),
    },
    SkillSource::Folder { path } => path.clone(),
    SkillSource::Zip { path } => path.clone(),
    SkillSource::None => "-".to_string(),
  }
}

/// Report one mutating action: blockers go to stderr with exit 2, an applied action prints
/// `label` and the resulting target states. With `--json` the `ActionResult` is printed as
/// is and the exit code is the only difference.
pub fn report_action(ctx: &Ctx, label: &str, result: &ActionResult) -> Result<i32, String> {
  if ctx.json {
    emit_json(result)?;
  } else if result.applied {
    ctx.say(label);
  } else {
    eprintln!("{}: blocked", label);
    for blocker in &result.blockers {
      eprintln!("  - {}", blocker);
    }
    eprintln!("Retry with --force to discard the local changes.");
  }
  Ok(if result.applied {
    EXIT_OK
  } else {
    EXIT_BLOCKED
  })
}

/// A fixed-width table. Columns are sized to their widest cell; the last column is not
/// padded so lines do not end in whitespace.
pub struct Table {
  headers: Vec<String>,
  rows: Vec<Vec<String>>,
}

impl Table {
  pub fn new(headers: &[&str]) -> Self {
    Self {
      headers: headers.iter().map(|h| h.to_string()).collect(),
      rows: Vec::new(),
    }
  }

  pub fn row(&mut self, cells: Vec<String>) {
    self.rows.push(cells);
  }

  pub fn is_empty(&self) -> bool {
    self.rows.is_empty()
  }

  pub fn print(&self) {
    let columns = self.headers.len();
    let mut widths: Vec<usize> = self.headers.iter().map(|h| h.chars().count()).collect();
    for row in &self.rows {
      for (i, cell) in row.iter().enumerate().take(columns) {
        widths[i] = widths[i].max(cell.chars().count());
      }
    }
    println!("{}", format_row(&self.headers, &widths));
    for row in &self.rows {
      println!("{}", format_row(row, &widths));
    }
  }
}

fn format_row(cells: &[String], widths: &[usize]) -> String {
  let mut line = String::new();
  let last = widths.len().saturating_sub(1);
  for (i, width) in widths.iter().enumerate() {
    let cell = cells.get(i).map(String::as_str).unwrap_or("");
    if i == last {
      line.push_str(cell);
    } else {
      let pad = width.saturating_sub(cell.chars().count()) + 2;
      line.push_str(cell);
      line.extend(std::iter::repeat_n(' ', pad));
    }
  }
  line.trim_end().to_string()
}
