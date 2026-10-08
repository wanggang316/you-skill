use crate::output::{emit_json, source_label, state_str, Ctx, Table, EXIT_OK};
use youskill_core::models::{HubSkillView, InstallScope};
use youskill_core::services::env::Env;

#[derive(clap::Args)]
pub struct Args {
  /// Skill name as listed by `youskill list`.
  name: String,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let skill = super::skill_or_error(&env, &args.name)?;
  if ctx.json {
    emit_json(&skill)?;
    return Ok(EXIT_OK);
  }
  print_skill(&skill);
  Ok(EXIT_OK)
}

pub fn print_skill(skill: &HubSkillView) {
  println!("Name:         {}", skill.name);
  if let Some(description) = &skill.description {
    println!("Description:  {}", description);
  }
  println!("Hub path:     {}", skill.hub_path);
  println!("Hub state:    {}", state_str(&skill.hub_state));
  println!("Source:       {}", source_label(&skill.source));
  println!("Source state: {}", state_str(&skill.source_state));
  println!("Imported:     {}", skill.imported_at);
  println!("Updated:      {}", skill.updated_at);
  println!();
  if skill.installs.is_empty() {
    println!("Not installed anywhere.");
    return;
  }
  let mut table = Table::new(&["SCOPE", "AGENTS", "MODE", "STATE", "PATH"]);
  for install in &skill.installs {
    let scope = match install.record.scope {
      InstallScope::User => "user".to_string(),
      InstallScope::Project => install
        .record
        .project_path
        .clone()
        .unwrap_or_else(|| "project".to_string()),
    };
    let mut state = state_str(&install.state);
    if install.project_missing {
      state.push_str(" (project missing)");
    }
    table.row(vec![
      scope,
      install.record.agent_ids.join(","),
      state_str(&install.record.mode),
      state,
      install.record.path.clone(),
    ]);
  }
  table.print();
}
