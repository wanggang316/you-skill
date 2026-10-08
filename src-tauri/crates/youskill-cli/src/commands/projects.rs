use crate::output::{emit_json, Ctx, Table, EXIT_OK};
use clap::Subcommand;
use youskill_core::models::UserProjectView;
use youskill_core::services::{user_projects_service, workspace_service};

#[derive(clap::Args)]
pub struct Args {
  #[command(subcommand)]
  command: Option<ProjectsCommand>,
}

#[derive(Subcommand)]
enum ProjectsCommand {
  /// List registered projects (the default).
  List,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  match args.command.unwrap_or(ProjectsCommand::List) {
    ProjectsCommand::List => list(ctx),
  }
}

/// Registered projects as the app lists them: projects of removed workspaces are dropped
/// first, and a project whose folder is gone is flagged rather than hidden.
fn list(ctx: &Ctx) -> Result<i32, String> {
  let workspace_paths: Vec<String> = workspace_service::list_workspaces()?
    .into_iter()
    .map(|workspace| workspace.path)
    .collect();
  user_projects_service::tidy_user_projects(&workspace_paths)?;
  let projects: Vec<UserProjectView> = user_projects_service::list_user_projects()?
    .into_iter()
    .map(|project| UserProjectView {
      missing: !std::path::Path::new(&project.path).is_dir(),
      project,
    })
    .collect();
  if ctx.json {
    emit_json(&projects)?;
    return Ok(EXIT_OK);
  }
  if projects.is_empty() {
    ctx.say("No registered projects.");
    return Ok(EXIT_OK);
  }
  let mut table = Table::new(&["NAME", "PATH", "WORKSPACE"]);
  for view in &projects {
    let mut path = view.project.path.clone();
    if view.missing {
      path.push_str(" (missing)");
    }
    table.row(vec![
      view.project.name.clone(),
      path,
      view
        .project
        .workspace_path
        .clone()
        .unwrap_or_else(|| "-".to_string()),
    ]);
  }
  table.print();
  Ok(EXIT_OK)
}
