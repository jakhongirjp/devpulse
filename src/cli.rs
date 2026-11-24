//! Command-line interface.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};

use crate::analysis::{self, Options};
use crate::config::{self, Config};
use crate::git::{GitCli, History, Query};
use crate::report::{self, Format, Report};

#[derive(Parser, Debug)]
#[command(
    name = "devpulse",
    version,
    about = "Engineering health metrics from git history"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print a report of hotspots, ownership and activity.
    Report(ReportArgs),
    /// Exit with status 1 when the repository breaks configured thresholds.
    Check(CommonArgs),
    /// Write a default .devpulse.toml to the repository.
    Init {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args, Debug)]
pub struct CommonArgs {
    /// Path to the git repository.
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,
    /// Only consider commits newer than this (any `git log --since` value).
    #[arg(long)]
    pub since: Option<String>,
    /// Restrict analysis to these paths.
    #[arg(long = "path")]
    pub paths: Vec<String>,
    /// Config file (defaults to <repo>/.devpulse.toml if present).
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ReportArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[arg(long, value_enum, default_value_t = Format::Table)]
    pub format: Format,
    /// Number of hotspots to show.
    #[arg(long, default_value_t = 10)]
    pub top: usize,
}

pub fn run(cli: Cli, out: &mut impl Write) -> Result<ExitCode> {
    match cli.command {
        Command::Report(args) => {
            let (text, _) = build(&args.common, args.top, args.format)?;
            out.write_all(text.as_bytes())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check(common) => {
            let (text, passed) = build(&common, 10, Format::Table)?;
            if passed {
                writeln!(out, "devpulse check passed")?;
                Ok(ExitCode::SUCCESS)
            } else {
                out.write_all(text.as_bytes())?;
                Ok(ExitCode::FAILURE)
            }
        }
        Command::Init { repo, force } => {
            let path = repo.join(config::FILE_NAME);
            if path.exists() && !force {
                anyhow::bail!(
                    "{} already exists (use --force to overwrite)",
                    path.display()
                );
            }
            std::fs::write(&path, Config::default().to_toml())
                .with_context(|| format!("writing {}", path.display()))?;
            writeln!(out, "wrote {}", path.display())?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn build(args: &CommonArgs, top: usize, format: Format) -> Result<(String, bool)> {
    let (cfg, _) = Config::load(&args.repo, args.config.as_deref())?;
    let git = GitCli::open(&args.repo)?;
    let commits = git.commits(&Query {
        since: args.since.clone(),
        paths: args.paths.clone(),
        revision: None,
    })?;
    let analysis = analysis::analyze(
        &commits,
        &Options {
            top,
            exclude: cfg.exclude_set()?,
            weeks: 12,
        },
    );
    let health = analysis::health_eval(&analysis, &cfg.check, Utc::now());
    let name = std::fs::canonicalize(&args.repo)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| args.repo.display().to_string());
    let text = report::render(
        &Report {
            repository: &name,
            health: &health,
            analysis: &analysis,
        },
        format,
    );
    Ok((text, health.passed()))
}
