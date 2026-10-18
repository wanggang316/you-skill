use crate::output::{emit_json, Ctx, Table, EXIT_OK};
use std::path::Path;
use youskill_core::models::{DetectedSkill, ImportItem, ImportOutcome, SkillSource};
use youskill_core::services::env::Env;
use youskill_core::services::{hub_service, skill_service};
use youskill_core::utils::folder::is_staged_temp_path;
use youskill_core::utils::github::GithubHelper;
use youskill_core::utils::path::{path_to_string, remove_path_any};

#[derive(clap::Args)]
pub struct Args {
  /// A GitHub URL or `owner/repo[/tree/<branch>[/<path>]]`, a `.zip` / `.skill` archive,
  /// or a local folder. Every skill found in it is imported.
  #[arg(required = true)]
  sources: Vec<String>,
  /// Only import these skill names (comma separated) from what was found.
  #[arg(long, value_delimiter = ',')]
  pick: Vec<String>,
  /// Replace a skill that is already in the hub; the old copy goes to the trash. A source
  /// that is already in the hub replaces its skill; a zip skill replaces the newest zip
  /// skill of the same name.
  #[arg(long)]
  overwrite: bool,
  /// Replace this skill (an id, a source key or a unique name) instead of the default.
  /// Needs `--overwrite` and one skill to import.
  #[arg(long, value_name = "SKILL", conflicts_with = "add")]
  replace: Option<String>,
  /// Add new skills also when a zip skill of the same name would be replaced.
  #[arg(long)]
  add: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let mut items = Vec::new();
  for source in &args.sources {
    items.extend(detect(source)?);
  }
  if !args.pick.is_empty() {
    let (picked, dropped): (Vec<ImportItem>, Vec<ImportItem>) = items
      .into_iter()
      .partition(|item| args.pick.iter().any(|name| name == &item.name));
    for item in dropped {
      discard(&item);
    }
    if picked.is_empty() {
      return Err("None of the picked names was found in the given sources".to_string());
    }
    items = picked;
  }
  let mut items = super::block_on(fill_remote_shas(items))?;
  choose_replacements(&env, &args, &mut items)?;
  let outcomes = hub_service::import_skills(&env, items, args.overwrite)?;
  if ctx.json {
    emit_json(&outcomes)?;
    return Ok(EXIT_OK);
  }
  print_outcomes(ctx, &outcomes);
  Ok(EXIT_OK)
}

/// Fill `replace_id` from `--replace`, `--add` or the default of the import plan.
fn choose_replacements(env: &Env, args: &Args, items: &mut [ImportItem]) -> Result<(), String> {
  if let Some(key) = &args.replace {
    let [item] = items else {
      return Err("--replace needs exactly one skill to import; use --pick".to_string());
    };
    item.replace_id = Some(hub_service::resolve_skill(env, key)?.0);
    return Ok(());
  }
  let plans = hub_service::plan_import(env, items)?;
  for (item, plan) in items.iter_mut().zip(plans) {
    item.replace_id = if args.add && !plan.replace_required {
      None
    } else {
      plan.default_replace_id
    };
  }
  Ok(())
}

fn detect(source: &str) -> Result<Vec<ImportItem>, String> {
  let path = Path::new(source);
  if path.is_dir() {
    let dir = super::absolute(source)?;
    let found = skill_service::detect_folder(path_to_string(&dir))?;
    return Ok(
      found
        .into_iter()
        .map(|skill| {
          // The folder that holds this skill's SKILL.md is its source.
          let skill_dir = match Path::new(&skill.skill_path).parent() {
            Some(parent) if !parent.as_os_str().is_empty() => dir.join(parent),
            _ => dir.clone(),
          };
          ImportItem {
            name: skill.name,
            tmp_path: skill.tmp_path,
            source: SkillSource::Folder {
              path: path_to_string(&skill_dir),
            },
            replace_id: None,
          }
        })
        .collect(),
    );
  }
  if path.is_file() {
    let file = super::absolute(source)?;
    let found = skill_service::detect_zip(path_to_string(&file))?;
    return Ok(
      found
        .into_iter()
        .map(|skill| ImportItem {
          name: skill.name,
          tmp_path: skill.tmp_path,
          source: SkillSource::Zip {
            path: path_to_string(&file),
          },
          replace_id: None,
        })
        .collect(),
    );
  }
  let reference = GithubHelper::parse_github_ref(source)
    .map_err(|_| format!("Not a folder, archive or GitHub reference: {}", source))?;
  let repo = format!("{}/{}", reference.owner, reference.repo);
  let found: Vec<DetectedSkill> =
    super::block_on(skill_service::detect_github_manual(source.to_string()))??;
  Ok(
    found
      .into_iter()
      .map(|skill| ImportItem {
        name: skill.name,
        tmp_path: skill.tmp_path,
        source: SkillSource::Github {
          repo: repo.clone(),
          url: format!("https://github.com/{}.git", repo),
          skill_path: skill.skill_path,
          branch: skill.branch,
          remote_sha: None,
          marketplace_id: None,
          latest_remote_sha: None,
          checked_at: None,
        },
        replace_id: None,
      })
      .collect(),
  )
}

/// GitHub imports record the tree sha of the skill folder so `update` has a baseline.
/// Best effort: a network failure leaves it empty, as in the app.
async fn fill_remote_shas(items: Vec<ImportItem>) -> Vec<ImportItem> {
  let mut filled = Vec::with_capacity(items.len());
  for mut item in items {
    if let SkillSource::Github {
      url,
      skill_path,
      branch,
      remote_sha,
      ..
    } = &mut item.source
    {
      if let Ok(sha) = GithubHelper::get_skill_folder_hash(url, skill_path, branch.as_deref()).await
      {
        *remote_sha = Some(sha);
      }
    }
    filled.push(item);
  }
  filled
}

fn discard(item: &ImportItem) {
  let staged = Path::new(&item.tmp_path);
  if is_staged_temp_path(staged) {
    let _ = remove_path_any(staged);
  }
}

fn print_outcomes(ctx: &Ctx, outcomes: &[ImportOutcome]) {
  if ctx.quiet {
    return;
  }
  let mut table = Table::new(&["NAME", "ID", "RESULT", "HUB PATH"]);
  for outcome in outcomes {
    table.row(vec![
      outcome.name.clone(),
      outcome.id.clone(),
      if outcome.replaced {
        "replaced"
      } else {
        "imported"
      }
      .to_string(),
      outcome.hub_path.clone(),
    ]);
  }
  table.print();
}
