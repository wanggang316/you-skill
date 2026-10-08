use crate::output::{emit_json, source_label, state_str, Ctx, Table, EXIT_OK};
use youskill_core::models::HubSkillView;
use youskill_core::services::env::Env;

#[derive(clap::Args)]
pub struct Args {
  /// Only skills in one of these hub, source or target states (comma separated), e.g.
  /// `outdated,modified,update_available`.
  #[arg(long, value_delimiter = ',')]
  state: Vec<String>,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let mut skills = super::hub_skills(&env)?;
  if !args.state.is_empty() {
    skills.retain(|skill| in_any_state(skill, &args.state));
  }
  if ctx.json {
    emit_json(&skills)?;
    return Ok(EXIT_OK);
  }
  if skills.is_empty() {
    ctx.say(if args.state.is_empty() {
      "No skills in the hub."
    } else {
      "No skills in the requested states."
    });
    return Ok(EXIT_OK);
  }
  let mut table = Table::new(&["NAME", "HUB", "SOURCE", "SOURCE STATE", "TARGETS"]);
  for skill in &skills {
    table.row(vec![
      skill.name.clone(),
      state_str(&skill.hub_state),
      source_label(&skill.source),
      state_str(&skill.source_state),
      super::targets_summary(skill),
    ]);
  }
  table.print();
  Ok(EXIT_OK)
}

fn in_any_state(skill: &HubSkillView, states: &[String]) -> bool {
  let hub = state_str(&skill.hub_state);
  let source = state_str(&skill.source_state);
  states.iter().any(|wanted| {
    *wanted == hub
      || *wanted == source
      || skill
        .installs
        .iter()
        .any(|install| state_str(&install.state) == *wanted)
  })
}
