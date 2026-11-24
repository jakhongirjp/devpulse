//! Reading history from git.
//!
//! We shell out to the `git` binary and parse a stable, machine-oriented log
//! format instead of linking libgit2. See docs/adr/0001-shell-out-to-git.md.

mod parser;

pub use parser::{ParseError, parse_log};

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::Commit;

/// Field and record separators that cannot appear in commit metadata.
pub(crate) const RECORD_SEP: char = '\x1e';
pub(crate) const FIELD_SEP: char = '\x1f';

/// Source of commits. A trait so analysis can be tested without a repository.
pub trait History {
    fn commits(&self, query: &Query) -> Result<Vec<Commit>, GitError>;
}

/// Which part of history to read.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Passed to `git log --since`, e.g. "6 months ago" or "2026-01-01".
    pub since: Option<String>,
    /// Restrict to these pathspecs.
    pub paths: Vec<String>,
    /// Revision range, defaults to HEAD.
    pub revision: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("failed to run git: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("{path} is not a git repository")]
    NotARepository { path: PathBuf },
    #[error("git log failed: {stderr}")]
    Failed { stderr: String },
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Reads history by running `git log` in a working directory.
pub struct GitCli {
    repo: PathBuf,
}

impl GitCli {
    pub fn open(repo: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo = repo.as_ref().to_path_buf();
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["rev-parse", "--git-dir"])
            .output()?;
        if !status.status.success() {
            return Err(GitError::NotARepository { path: repo });
        }
        Ok(Self { repo })
    }
}

impl GitCli {
    fn has_head(&self) -> Result<bool, GitError> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(["rev-parse", "--verify", "--quiet", "HEAD"])
            .output()?;
        Ok(out.status.success())
    }
}

impl History for GitCli {
    fn commits(&self, query: &Query) -> Result<Vec<Commit>, GitError> {
        // A freshly initialised repository has no HEAD yet: zero commits, not an error.
        if query.revision.is_none() && !self.has_head()? {
            return Ok(Vec::new());
        }
        let format = format!(
            "--format={RECORD_SEP}%H{FIELD_SEP}%aN{FIELD_SEP}%aE{FIELD_SEP}%at{FIELD_SEP}%s"
        );
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(&self.repo)
            // Predictable output regardless of user configuration.
            .args([
                "-c",
                "core.quotepath=off",
                "log",
                "--no-merges",
                "--numstat",
                "-M",
                "--no-color",
            ])
            .arg(format);
        if let Some(since) = &query.since {
            cmd.arg(format!("--since={since}"));
        }
        cmd.arg(query.revision.as_deref().unwrap_or("HEAD"));
        cmd.arg("--");
        cmd.args(&query.paths);

        let out = cmd.output()?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(GitError::Failed { stderr });
        }
        Ok(parse_log(&String::from_utf8_lossy(&out.stdout))?)
    }
}
