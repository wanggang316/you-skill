use crate::output::{emit_json, state_str, Ctx, Table, EXIT_OK};
use serde::Serialize;
use youskill_core::models::{ImportOutcome, ScanDecision, ScanItem, ScanResolution, ScanStatus};
use youskill_core::services::env::Env;
use youskill_core::services::scan_roots_service;
use youskill_core::services::scan_service::{self, DEFAULT_SCAN_DEPTH};
use youskill_core::utils::path::path_to_string;

#[derive(clap::Args)]
pub struct Args {
  /// Folder to look through for `SKILL.md` directories.
  dir: String,
  /// How many levels deep to look.
  #[arg(long, default_value_t = DEFAULT_SCAN_DEPTH)]
  depth: usize,
  /// Take every skill that is not in the hub yet into it. A copy inside an agent skills
  /// directory is recorded as an install of that agent.
  #[arg(long)]
  import: bool,
  /// Record copies of skills the hub already has as install locations, without touching
  /// their content.
  #[arg(long)]
  register: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
  found: Vec<ScanItem>,
  imported: Vec<ImportOutcome>,
  registered: Vec<String>,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let dir = path_to_string(&super::absolute(&args.dir)?);
  let found = scan_service::scan_folder(&env, &dir, args.depth)?;
  if let Err(err) = scan_roots_service::touch_scan_root(&dir, found.len()) {
    tracing::warn!("recording the scan of '{}' failed: {}", dir, err);
  }

  let mut decisions = Vec::new();
  let mut registered = Vec::new();
  let mut seen_new: Vec<String> = Vec::new();
  for item in &found {
    match item.status {
      // Only one folder can become the hub copy of a name; later copies are registered.
      ScanStatus::New if args.import => {
        let first = !seen_new.contains(&item.name);
        if first {
          seen_new.push(item.name.clone());
        }
        let in_agent_root = item.in_agent_root.is_some();
        if !first && !in_agent_root {
          continue;
        }
        if !first {
          registered.push(item.path.clone());
        }
        decisions.push(ScanDecision {
          name: item.name.clone(),
          path: item.path.clone(),
          resolution: if first {
            ScanResolution::Import
          } else {
            ScanResolution::RegisterOnly
          },
          register_install: in_agent_root,
          source: item.source_hint.clone(),
        });
      },
      ScanStatus::Identical | ScanStatus::Different if args.register => {
        if item.in_agent_root.is_none() {
          continue;
        }
        registered.push(item.path.clone());
        decisions.push(ScanDecision {
          name: item.name.clone(),
          path: item.path.clone(),
          resolution: ScanResolution::RegisterOnly,
          register_install: true,
          source: None,
        });
      },
      _ => {},
    }
  }
  let imported = if decisions.is_empty() {
    Vec::new()
  } else {
    scan_service::import_scanned(&env, decisions)?
  };

  if ctx.json {
    emit_json(&Report {
      found,
      imported,
      registered,
    })?;
    return Ok(EXIT_OK);
  }
  if found.is_empty() {
    ctx.say(&format!("No skills found under {}", dir));
    return Ok(EXIT_OK);
  }
  if !ctx.quiet {
    let mut table = Table::new(&["NAME", "STATUS", "AGENTS", "PATH"]);
    for item in &found {
      let agents = item
        .in_agent_root
        .as_ref()
        .map(|m| m.agent_ids.join(","))
        .unwrap_or_else(|| "-".to_string());
      table.row(vec![
        item.name.clone(),
        state_str(&item.status),
        agents,
        item.path.clone(),
      ]);
    }
    table.print();
  }
  if !imported.is_empty() {
    ctx.say(&format!(
      "Imported {}: {}",
      imported.len(),
      imported
        .iter()
        .map(|o| o.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
    ));
  }
  if !registered.is_empty() {
    ctx.say(&format!(
      "Registered {} install location(s).",
      registered.len()
    ));
  }
  if !args.import && found.iter().any(|item| item.status == ScanStatus::New) {
    ctx.say("Run again with --import to take the new skills into the hub.");
  }
  Ok(EXIT_OK)
}
