//! `.devpulse.toml` loading.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::analysis::Thresholds;

pub const FILE_NAME: &str = ".devpulse.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Glob patterns excluded from file metrics (lockfiles, generated code...).
    pub exclude: Vec<String>,
    pub check: Thresholds,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exclude: [
                "**/*.lock",
                "**/package-lock.json",
                "**/vendor/**",
                "**/dist/**",
                "**/*.min.js",
            ]
            .map(String::from)
            .to_vec(),
            check: Thresholds::default(),
        }
    }
}

impl Config {
    /// Loads `path` if given, else `<repo>/.devpulse.toml` if present, else defaults.
    pub fn load(repo: &Path, path: Option<&Path>) -> Result<(Self, Option<PathBuf>)> {
        let candidate = path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo.join(FILE_NAME));
        if !candidate.exists() {
            if path.is_some() {
                anyhow::bail!("config file {} not found", candidate.display());
            }
            return Ok((Self::default(), None));
        }
        let text = std::fs::read_to_string(&candidate)
            .with_context(|| format!("reading {}", candidate.display()))?;
        let cfg =
            toml::from_str(&text).with_context(|| format!("parsing {}", candidate.display()))?;
        Ok((cfg, Some(candidate)))
    }

    pub fn exclude_set(&self) -> Result<GlobSet> {
        let mut b = GlobSetBuilder::new();
        for pattern in &self.exclude {
            b.add(
                Glob::new(pattern)
                    .with_context(|| format!("invalid exclude pattern {pattern:?}"))?,
            );
        }
        Ok(b.build()?)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("config is always serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_unknown_keys() {
        let cfg = Config::default();
        let parsed: Config = toml::from_str(&cfg.to_toml()).unwrap();
        assert_eq!(parsed, cfg);
        assert!(toml::from_str::<Config>("colour = 'red'").is_err());
    }

    #[test]
    fn partial_files_keep_defaults() {
        let cfg: Config = toml::from_str("[check]\nmin_bus_factor = 3").unwrap();
        assert_eq!(cfg.check.min_bus_factor, 3);
        assert_eq!(cfg.check.min_score, Thresholds::default().min_score);
        assert!(!cfg.exclude.is_empty());
    }

    #[test]
    fn rejects_bad_globs() {
        let cfg = Config {
            exclude: vec!["a{".into()],
            ..Default::default()
        };
        assert!(cfg.exclude_set().is_err());
    }
}
