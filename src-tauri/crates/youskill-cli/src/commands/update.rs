use crate::output::{emit_json, Ctx, Table, EXIT_BLOCKED, EXIT_OK};
use serde::Serialize;
use youskill_core::models::{ActionResult, SourceUpdate};
use youskill_core::services::env::Env;
use youskill_core::services::source_service;

#[derive(clap::Args)]
pub struct Args {
  /// Skills to update; every skill with a GitHub source when omitted.
  names: Vec<String>,
  /// Only refresh the source state; do not pull anything.
  #[arg(long)]
  check: bool,
  /// Pull even when the hub copy has local changes.
  #[arg(long)]
  force: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
  checked: Vec<SourceUpdate>,
  pulled: Vec<ActionResult>,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  for name in &args.names {
    super::skill_or_error(&env, name)?;
  }
  let names = (!args.names.is_empty()).then(|| args.names.clone());
  let checked = super::block_on(source_service::check_source_updates(env.clone(), names))??;

  let mut pulled = Vec::new();
  let mut code = EXIT_OK;
  if !args.check {
    for update in checked.iter().filter(|u| u.update_available) {
      let result = super::block_on(source_service::pull_github_source(
        env.clone(),
        update.name.clone(),
        args.force,
      ))??;
      if result.applied {
        ctx.say(&format!("Updated '{}'", update.name));
      } else {
        code = EXIT_BLOCKED;
        if !ctx.json {
          eprintln!("'{}': blocked", update.name);
          for blocker in &result.blockers {
            eprintln!("  - {}", blocker);
          }
        }
      }
      pulled.push(result);
    }
  }

  if ctx.json {
    emit_json(&Report { checked, pulled })?;
    return Ok(code);
  }
  if checked.is_empty() {
    ctx.say("No skills with a GitHub source.");
    return Ok(code);
  }
  let mut table = Table::new(&["NAME", "STATE", "NOTE"]);
  for update in &checked {
    let state = match (update.update_available, &update.error) {
      (_, Some(_)) => "error",
      (true, None) if args.check => "update_available",
      (true, None) => "updated",
      (false, None) => "in_sync",
    };
    table.row(vec![
      update.name.clone(),
      state.to_string(),
      update.error.clone().unwrap_or_default(),
    ]);
  }
  if !ctx.quiet {
    table.print();
  }
  if code == EXIT_BLOCKED {
    eprintln!("Retry with --force to discard the local changes.");
  }
  Ok(code)
}
