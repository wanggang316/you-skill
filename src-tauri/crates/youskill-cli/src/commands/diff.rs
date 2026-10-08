use crate::output::{emit_json, Ctx, EXIT_ERROR, EXIT_OK};
use std::path::{Path, PathBuf};
use youskill_core::models::{DiffLineKind, DiffStatus, SkillDiff, SkillSource};
use youskill_core::services::env::Env;
use youskill_core::services::lock_service::store;
use youskill_core::services::{diff_service, source_service};
use youskill_core::utils::folder::is_staged_temp_path;
use youskill_core::utils::path::remove_path_any;

#[derive(clap::Args)]
pub struct Args {
  /// Skill name as listed by `youskill list`.
  name: String,
  /// Compare with this install target (its path as shown by `youskill show`).
  #[arg(long, conflicts_with = "source", required_unless_present = "source")]
  target: Option<String>,
  /// Compare with the recorded source: a local folder, or the GitHub repository, which
  /// is downloaded to a temporary directory first.
  #[arg(long)]
  source: bool,
}

/// Exits 0 when both sides are identical and 1 when they differ, like `diff(1)`.
pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  super::skill_or_error(&env, &args.name)?;
  let diff = match args.target {
    Some(target) => diff_service::diff_hub_against(&env, &args.name, Path::new(&target), &target)?,
    None => diff_source(&env, &args.name)?,
  };
  let code = if diff.files.is_empty() {
    EXIT_OK
  } else {
    EXIT_ERROR
  };
  if ctx.json {
    emit_json(&diff)?;
    return Ok(code);
  }
  print_unified(&diff);
  Ok(code)
}

fn diff_source(env: &Env, name: &str) -> Result<SkillDiff, String> {
  let record = store(env)
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  match record.source {
    SkillSource::Folder { path } => {
      diff_service::diff_hub_against(env, name, Path::new(&path), &path)
    },
    SkillSource::Github {
      repo, url, branch, ..
    } => {
      let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
      let detected = runtime.block_on(source_service::stage_github_source(
        &repo,
        &url,
        branch.as_deref(),
        name,
      ))?;
      let label = match detected.branch.as_deref().or(branch.as_deref()) {
        Some(b) if !b.is_empty() => format!("{}@{}", repo, b),
        _ => repo.clone(),
      };
      let staged = PathBuf::from(detected.tmp_path);
      let result = diff_service::diff_hub_against(env, name, &staged, &label);
      if is_staged_temp_path(&staged) {
        let _ = remove_path_any(&staged);
      }
      result
    },
    _ => Err("This skill has no source to compare with".to_string()),
  }
}

/// Unified diff with the hub on the left (`a/`) and the target or source on the right.
pub fn print_unified(diff: &SkillDiff) {
  if diff.files.is_empty() {
    println!(
      "{} and {} are identical ({} files).",
      diff.left_label, diff.right_label, diff.unchanged
    );
    return;
  }
  println!("# hub:   {}", diff.left_label);
  println!("# right: {}", diff.right_label);
  for file in &diff.files {
    let (left, right) = match file.status {
      DiffStatus::Added => ("/dev/null".to_string(), format!("b/{}", file.path)),
      DiffStatus::Removed => (format!("a/{}", file.path), "/dev/null".to_string()),
      DiffStatus::Modified => (format!("a/{}", file.path), format!("b/{}", file.path)),
    };
    println!("--- {}", left);
    println!("+++ {}", right);
    if file.binary {
      println!("Binary files differ");
      continue;
    }
    for hunk in &file.hunks {
      println!("{}", hunk.header);
      for line in &hunk.lines {
        let prefix = match line.kind {
          DiffLineKind::Context => ' ',
          DiffLineKind::Delete => '-',
          DiffLineKind::Insert => '+',
        };
        println!("{}{}", prefix, line.text);
      }
    }
    if file.truncated {
      println!("(diff truncated)");
    }
  }
  println!(
    "# {} files differ, {} unchanged",
    diff.files.len(),
    diff.unchanged
  );
}
