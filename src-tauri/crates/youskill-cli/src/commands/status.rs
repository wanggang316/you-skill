use crate::output::{emit_json, state_str, Ctx, Table, EXIT_DRIFT, EXIT_OK};
use youskill_core::services::env::Env;

#[derive(clap::Args)]
pub struct Args {
  /// Exit with 3 when any skill has drift, for scripts and hooks.
  #[arg(long)]
  exit_code: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let mut skills = super::hub_skills(&env)?;
  skills.retain(|skill| skill.has_drift);
  let code = if args.exit_code && !skills.is_empty() {
    EXIT_DRIFT
  } else {
    EXIT_OK
  };
  if ctx.json {
    emit_json(&skills)?;
    return Ok(code);
  }
  if skills.is_empty() {
    ctx.say("All skills are in sync.");
    return Ok(code);
  }
  let mut table = Table::new(&["NAME", "HUB", "SOURCE STATE", "TARGETS"]);
  for skill in &skills {
    table.row(vec![
      skill.name.clone(),
      state_str(&skill.hub_state),
      state_str(&skill.source_state),
      super::targets_summary(skill),
    ]);
  }
  table.print();
  Ok(code)
}
