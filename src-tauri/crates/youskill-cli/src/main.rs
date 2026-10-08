//! `youskill`: manage the hub in `~/.youskill` from the terminal. Every command runs
//! through `youskill-core`, the same crate the desktop app uses, so both see one hub.

mod commands;
mod output;

use clap::{Parser, Subcommand};
use output::{Ctx, EXIT_ERROR};
use std::process::exit;

#[derive(Parser)]
#[command(
  name = "youskill",
  version,
  about = "Manage the YouSkill hub from the terminal",
  propagate_version = true
)]
struct Cli {
  /// Print machine-readable JSON instead of tables.
  #[arg(long, global = true)]
  json: bool,
  /// Print only errors and the data a command is asked for.
  #[arg(short, long, global = true)]
  quiet: bool,
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// List the skills in the hub with their drift state.
  List(commands::list::Args),
  /// Show one skill: source, hub state and every install target.
  Show(commands::show::Args),
  /// Report every skill that is not fully in sync.
  Status(commands::status::Args),
  /// Diff the hub copy of a skill against an install target or its source.
  Diff(commands::diff::Args),
  /// List the agent apps and their skills directories.
  Agents(commands::agents::Args),
  /// Registered projects.
  Projects(commands::projects::Args),
}

fn main() {
  reset_sigpipe();
  tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
    )
    .init();

  let cli = Cli::parse();
  let ctx = Ctx {
    json: cli.json,
    quiet: cli.quiet,
  };
  let result = match cli.command {
    Command::List(args) => commands::list::run(&ctx, args),
    Command::Show(args) => commands::show::run(&ctx, args),
    Command::Status(args) => commands::status::run(&ctx, args),
    Command::Diff(args) => commands::diff::run(&ctx, args),
    Command::Agents(args) => commands::agents::run(&ctx, args),
    Command::Projects(args) => commands::projects::run(&ctx, args),
  };
  match result {
    Ok(code) => exit(code),
    Err(message) => {
      output::error(&ctx, &message);
      exit(EXIT_ERROR);
    },
  }
}

/// Rust ignores SIGPIPE, so `youskill list | head` would panic on the closed pipe. Restore
/// the default so the process ends quietly like any other command line tool.
fn reset_sigpipe() {
  #[cfg(unix)]
  // SAFETY: setting a signal disposition before any other thread exists.
  unsafe {
    libc::signal(libc::SIGPIPE, libc::SIG_DFL);
  }
}
