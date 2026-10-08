//! `youskill instructions ...`: the instruction library (`~/.youskill/instructions`) and
//! the agent files (`AGENTS.md`, `CLAUDE.md`, ...) templates install to. Entries are
//! keyed by id; a command accepts the id or a unique name.

use crate::output::{emit_json, report_action, state_str, Ctx, Table, EXIT_BLOCKED, EXIT_OK};
use clap::Subcommand;
use std::path::Path;
use youskill_core::config::load_config;
use youskill_core::models::{
  DetectedInstruction, InstallMode, InstallRequest, InstallScope, InstructionImportItem,
  InstructionSource, InstructionView, SyncAction, UninstallRequest,
};
use youskill_core::services::env::Env;
use youskill_core::services::instruction_service;
use youskill_core::utils::github::GithubHelper;
use youskill_core::utils::path::path_to_string;

#[derive(clap::Args)]
pub struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// List the templates in the library with their drift state.
  List,
  /// Show one template: source, library state and every agent file it is installed to.
  Show { key: String },
  /// Print the library content of a template.
  Cat { key: String },
  /// List the agent instruction files of the user level and every registered project.
  Files,
  /// Import Markdown files into the library: files, folders (Markdown files inside,
  /// up to six levels deep) or a GitHub reference.
  Import {
    #[arg(required = true)]
    sources: Vec<String>,
    /// Only import the files whose relative path or suggested name is listed.
    #[arg(long, value_delimiter = ',')]
    pick: Vec<String>,
    /// Library name; only with a single file.
    #[arg(long)]
    name: Option<String>,
  },
  /// Install a template to the instruction file of agent apps.
  Install {
    key: String,
    /// Agent ids (comma separated, see `youskill agents`), or `all`.
    #[arg(short, long, value_delimiter = ',', required = true)]
    agents: Vec<String>,
    /// Install into a project instead of the user level. Without a value the current
    /// directory is used; put the flag last or write `--project=<path>`.
    #[arg(short, long, num_args = 0..=1, default_missing_value = ".")]
    project: Option<String>,
    /// Copy the file or link it to the library. Defaults to the app's sync mode setting.
    #[arg(long, value_enum)]
    mode: Option<super::install::Mode>,
    /// Overwrite a file with different content, switch its mode, or take the location
    /// over from another template.
    #[arg(long)]
    force: bool,
  },
  /// Remove a template from the instruction file of agent apps.
  Uninstall {
    key: String,
    /// Agent ids (comma separated), or `all` for every agent recorded in this scope.
    #[arg(short, long, value_delimiter = ',', required = true)]
    agents: Vec<String>,
    #[arg(short, long, num_args = 0..=1, default_missing_value = ".")]
    project: Option<String>,
    /// Delete a file even when it has local changes.
    #[arg(long)]
    force: bool,
  },
  /// Remove templates from the library.
  Remove {
    #[arg(required = true)]
    keys: Vec<String>,
    /// Leave installed files in place as unmanaged files.
    #[arg(long)]
    keep_targets: bool,
  },
  /// Resolve drift between the library and the installed files.
  Sync {
    key: String,
    #[arg(value_enum)]
    action: SyncKind,
    /// Installed file path(s) as shown by `show`. Required for `adopt-target`.
    #[arg(long)]
    target: Vec<String>,
    #[arg(long)]
    force: bool,
  },
  /// Diff the library copy against an installed file.
  Diff {
    key: String,
    #[arg(long)]
    target: String,
  },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum SyncKind {
  PushTargets,
  AdoptTarget,
  AcceptHub,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  match args.command {
    Command::List => list(ctx, &env),
    Command::Show { key } => {
      let view = resolve(&env, &key)?;
      if ctx.json {
        emit_json(&view)?;
      } else {
        print_view(&view);
      }
      Ok(EXIT_OK)
    },
    Command::Cat { key } => {
      let view = resolve(&env, &key)?;
      print!("{}", instruction_service::read_instruction(&env, &view.id)?);
      Ok(EXIT_OK)
    },
    Command::Files => files(ctx, &env),
    Command::Import {
      sources,
      pick,
      name,
    } => import(ctx, &env, sources, pick, name),
    Command::Install {
      key,
      agents,
      project,
      mode,
      force,
    } => {
      let view = resolve(&env, &key)?;
      let scope = super::Scope::from_project_flag(project.as_deref())?;
      let agent_ids = super::resolve_agents(&env, &agents, &scope, super::Unit::Instruction)?;
      let mode = match mode {
        Some(mode) => InstallMode::from(mode),
        None => InstallMode::from_setting(&load_config()?.sync_mode),
      };
      let result = instruction_service::install_instruction(
        &env,
        InstallRequest {
          name: view.id.clone(),
          targets: scope.targets(&agent_ids),
          mode: Some(mode),
          force,
        },
      )?;
      report_action(
        ctx,
        &format!(
          "Installed '{}' to {} ({})",
          view.name,
          scope.label(),
          agent_ids.join(",")
        ),
        &result,
      )
    },
    Command::Uninstall {
      key,
      agents,
      project,
      force,
    } => {
      let view = resolve(&env, &key)?;
      let scope = super::Scope::from_project_flag(project.as_deref())?;
      let agent_ids = if agents.iter().any(|id| id == "all") {
        recorded_agents(&view, &scope)
      } else {
        agents
      };
      if agent_ids.is_empty() {
        ctx.say(&format!(
          "'{}' is not installed at {}",
          view.name,
          scope.label()
        ));
        return Ok(EXIT_OK);
      }
      let result = instruction_service::uninstall_instruction(
        &env,
        UninstallRequest {
          name: view.id.clone(),
          targets: scope.targets(&agent_ids),
          paths: Vec::new(),
          force,
        },
      )?;
      report_action(
        ctx,
        &format!(
          "Uninstalled '{}' from {} ({})",
          view.name,
          scope.label(),
          agent_ids.join(",")
        ),
        &result,
      )
    },
    Command::Remove { keys, keep_targets } => {
      let mut removed = Vec::new();
      for key in &keys {
        let view = resolve(&env, key)?;
        let question = if keep_targets || view.installs.is_empty() {
          format!("Remove '{}' from the library?", view.name)
        } else {
          format!(
            "Remove '{}' from the library and delete its {} installed file(s)?",
            view.name,
            view.installs.len()
          )
        };
        if !ctx.confirm(&question) {
          return Ok(EXIT_BLOCKED);
        }
        instruction_service::remove_instruction(&env, &view.id, !keep_targets)?;
        ctx.say(&format!("Removed '{}'", view.name));
        removed.push(view.id);
      }
      if ctx.json {
        println!("{}", serde_json::json!({ "removed": removed }));
      }
      Ok(EXIT_OK)
    },
    Command::Sync {
      key,
      action,
      target,
      force,
    } => {
      let view = resolve(&env, &key)?;
      let (action, label) = match action {
        SyncKind::PushTargets => (
          SyncAction::PushTargets {
            targets: (!target.is_empty()).then(|| target.clone()),
            force,
          },
          "pushed to targets",
        ),
        SyncKind::AdoptTarget => {
          let path = match target.as_slice() {
            [path] => path.clone(),
            _ => return Err("adopt-target needs exactly one --target <path>".to_string()),
          };
          (SyncAction::AdoptTarget { path, force }, "adopted target")
        },
        SyncKind::AcceptHub => (SyncAction::AcceptHub, "accepted library content"),
      };
      let result = instruction_service::sync_instruction(&env, &view.id, action)?;
      report_action(ctx, &format!("'{}': {}", view.name, label), &result)
    },
    Command::Diff { key, target } => {
      let view = resolve(&env, &key)?;
      let diff = instruction_service::diff_instruction(&env, &view.id, &target)?;
      let code = if diff.files.is_empty() {
        EXIT_OK
      } else {
        crate::output::EXIT_ERROR
      };
      if ctx.json {
        emit_json(&diff)?;
      } else {
        super::diff::print_unified(&diff);
      }
      Ok(code)
    },
  }
}

/// A template by id, or by name when exactly one template has it (case-insensitive).
fn resolve(env: &Env, key: &str) -> Result<InstructionView, String> {
  let key = key.trim();
  if let Some(view) = instruction_service::get_instruction(env, key)? {
    return Ok(view);
  }
  let matches: Vec<InstructionView> = instruction_service::list_instructions(env)?
    .into_iter()
    .filter(|view| view.name.eq_ignore_ascii_case(key))
    .collect();
  match matches.len() {
    0 => Err(format!("Instruction '{}' is not in the library", key)),
    1 => Ok(matches.into_iter().next().unwrap()),
    _ => Err(format!(
      "Several instructions are named '{}'; use an id: {}",
      key,
      matches
        .iter()
        .map(|view| view.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
    )),
  }
}

fn recorded_agents(view: &InstructionView, scope: &super::Scope) -> Vec<String> {
  let mut ids: Vec<String> = view
    .installs
    .iter()
    .filter(|install| install.record.scope == scope.scope)
    .filter(
      |install| match (&install.record.project_path, &scope.project_path) {
        (Some(a), Some(b)) => youskill_core::utils::path::same_path(Path::new(a), Path::new(b)),
        (None, None) => true,
        _ => false,
      },
    )
    .flat_map(|install| install.record.agent_ids.clone())
    .collect();
  ids.sort();
  ids.dedup();
  ids
}

fn list(ctx: &Ctx, env: &Env) -> Result<i32, String> {
  let views = instruction_service::list_instructions(env)?;
  if ctx.json {
    emit_json(&views)?;
    return Ok(EXIT_OK);
  }
  if views.is_empty() {
    ctx.say("No instructions in the library.");
    return Ok(EXIT_OK);
  }
  let mut table = Table::new(&["ID", "NAME", "HUB", "SOURCE", "TARGETS"]);
  for view in &views {
    table.row(vec![
      view.id.clone(),
      view.name.clone(),
      state_str(&view.hub_state),
      source_label(&view.source),
      targets_summary(view),
    ]);
  }
  table.print();
  Ok(EXIT_OK)
}

fn files(ctx: &Ctx, env: &Env) -> Result<i32, String> {
  let files = instruction_service::list_instruction_files(env)?;
  if ctx.json {
    emit_json(&files)?;
    return Ok(EXIT_OK);
  }
  if files.is_empty() {
    ctx.say("No agent instruction files found.");
    return Ok(EXIT_OK);
  }
  let mut table = Table::new(&["SCOPE", "AGENTS", "TEMPLATE", "STATE", "PATH"]);
  for file in &files {
    let scope = match file.scope {
      InstallScope::User => "user".to_string(),
      InstallScope::Project => file
        .project_path
        .clone()
        .unwrap_or_else(|| "project".to_string()),
    };
    let (template, state) = match &file.template {
      Some(template) => (template.name.clone(), state_str(&template.state)),
      None => ("-".to_string(), "-".to_string()),
    };
    table.row(vec![
      scope,
      file.agent_ids.join(","),
      template,
      state,
      file.path.clone(),
    ]);
  }
  table.print();
  Ok(EXIT_OK)
}

fn import(
  ctx: &Ctx,
  env: &Env,
  sources: Vec<String>,
  pick: Vec<String>,
  name: Option<String>,
) -> Result<i32, String> {
  let mut found: Vec<DetectedInstruction> = Vec::new();
  for source in &sources {
    let path = Path::new(source);
    if path.exists() {
      let absolute = super::absolute(source)?;
      found.extend(instruction_service::detect_instruction_files(
        env,
        &[path_to_string(&absolute)],
      )?);
    } else {
      GithubHelper::parse_github_ref(source)
        .map_err(|_| format!("Not a file, folder or GitHub reference: {}", source))?;
      found.extend(super::block_on(
        instruction_service::detect_instruction_github(env, source),
      )??);
    }
  }
  if !pick.is_empty() {
    found.retain(|item| {
      pick
        .iter()
        .any(|wanted| wanted == &item.rel_path || wanted.eq_ignore_ascii_case(&item.name))
    });
    if found.is_empty() {
      return Err("None of the picked files was found in the given sources".to_string());
    }
  }
  if found.is_empty() {
    return Err("No Markdown files found".to_string());
  }
  if name.is_some() && found.len() != 1 {
    return Err(format!(
      "--name needs a single file, but {} were found; use --pick",
      found.len()
    ));
  }
  let items: Vec<InstructionImportItem> = found
    .into_iter()
    .map(|item| InstructionImportItem {
      name: name.clone().unwrap_or(item.name),
      path: item.path,
      source: item.source,
    })
    .collect();
  let outcomes = instruction_service::import_instructions(env, items)?;
  if ctx.json {
    emit_json(&outcomes)?;
    return Ok(EXIT_OK);
  }
  if !ctx.quiet {
    let mut table = Table::new(&["ID", "NAME", "LIBRARY PATH"]);
    for outcome in &outcomes {
      table.row(vec![
        outcome.id.clone(),
        outcome.name.clone(),
        outcome.hub_path.clone(),
      ]);
    }
    table.print();
  }
  Ok(EXIT_OK)
}

fn print_view(view: &InstructionView) {
  println!("Id:           {}", view.id);
  println!("Name:         {}", view.name);
  if let Some(description) = &view.description {
    println!("Description:  {}", description);
  }
  println!("Library path: {}", view.hub_path);
  println!("Hub state:    {}", state_str(&view.hub_state));
  println!("Source:       {}", source_label(&view.source));
  println!("Imported:     {}", view.imported_at);
  println!("Updated:      {}", view.updated_at);
  println!();
  if view.installs.is_empty() {
    println!("Not installed anywhere.");
    return;
  }
  let mut table = Table::new(&["SCOPE", "AGENTS", "MODE", "STATE", "PATH"]);
  for install in &view.installs {
    let scope = match install.record.scope {
      InstallScope::User => "user".to_string(),
      InstallScope::Project => install
        .record
        .project_path
        .clone()
        .unwrap_or_else(|| "project".to_string()),
    };
    table.row(vec![
      scope,
      install.record.agent_ids.join(","),
      state_str(&install.record.mode),
      state_str(&install.state),
      install.record.path.clone(),
    ]);
  }
  table.print();
}

fn source_label(source: &InstructionSource) -> String {
  match source {
    InstructionSource::Github {
      repo,
      file_path,
      branch,
    } => match branch.as_deref() {
      Some(b) if !b.is_empty() => format!("{}@{}:{}", repo, b, file_path),
      _ => format!("{}:{}", repo, file_path),
    },
    InstructionSource::File { path } | InstructionSource::Agent { path } => path.clone(),
    InstructionSource::None => "-".to_string(),
  }
}

fn targets_summary(view: &InstructionView) -> String {
  if view.installs.is_empty() {
    return "-".to_string();
  }
  let mut counts: Vec<(String, usize)> = Vec::new();
  for install in &view.installs {
    let state = state_str(&install.state);
    match counts.iter_mut().find(|(s, _)| *s == state) {
      Some((_, n)) => *n += 1,
      None => counts.push((state, 1)),
    }
  }
  counts
    .iter()
    .map(|(state, n)| format!("{} {}", n, state))
    .collect::<Vec<_>>()
    .join(", ")
}
