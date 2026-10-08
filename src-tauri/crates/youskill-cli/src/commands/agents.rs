use crate::output::{emit_json, Ctx, Table, EXIT_OK};
use youskill_core::services::agent_apps_service::{all_agent_apps, local_agent_apps};

#[derive(clap::Args)]
pub struct Args {
  /// Include agent apps that are not detected on this machine.
  #[arg(long)]
  all: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let local = local_agent_apps();
  let apps = if args.all {
    all_agent_apps()
  } else {
    local.clone()
  };
  if ctx.json {
    emit_json(&apps)?;
    return Ok(EXIT_OK);
  }
  let mut table = Table::new(&["ID", "NAME", "DETECTED", "USER SKILLS", "PROJECT SKILLS"]);
  for app in &apps {
    let detected = local.iter().any(|local| local.id == app.id);
    table.row(vec![
      app.id.clone(),
      app.display_name.clone(),
      if detected { "yes" } else { "no" }.to_string(),
      app.global_path.clone().unwrap_or_else(|| "-".to_string()),
      app.project_path.clone().unwrap_or_else(|| "-".to_string()),
    ]);
  }
  if table.is_empty() {
    ctx.say("No agent apps detected. Use --all to list every known app.");
  } else {
    table.print();
  }
  Ok(EXIT_OK)
}
