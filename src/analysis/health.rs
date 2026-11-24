//! Turns metrics into a 0–100 health score and pass/fail checks for CI gates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Analysis;

/// Limits enforced by `devpulse check`. Loaded from `.devpulse.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Thresholds {
    pub min_score: u8,
    pub min_bus_factor: usize,
    /// Fail if the top hotspot owns more than this share of all churn.
    pub max_hotspot_share: f64,
    pub max_days_since_commit: i64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_score: 60,
            min_bus_factor: 2,
            max_hotspot_share: 0.25,
            max_days_since_commit: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub score: u8,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Violation {
    pub rule: &'static str,
    pub message: String,
}

impl HealthReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Score components (weights sum to 100):
/// - 40: knowledge spread (bus factor, full marks at 4+)
/// - 30: churn concentration (share of churn in the top hotspot)
/// - 30: recency (days since the last commit, zero after 180 days)
pub fn evaluate(a: &Analysis, t: &Thresholds, now: DateTime<Utc>) -> HealthReport {
    let total_churn = a.summary.lines_added + a.summary.lines_deleted;
    let top_share = match (a.hotspots.first(), total_churn) {
        (Some(h), total) if total > 0 => h.churn as f64 / total as f64,
        _ => 0.0,
    };
    let idle_days = a.summary.last_commit.map(|l| (now - l).num_days().max(0));

    let spread = (a.bus_factor.value.min(4) as f64 / 4.0) * 40.0;
    let concentration = (1.0 - top_share).clamp(0.0, 1.0) * 30.0;
    let recency = idle_days.map_or(0.0, |d| (1.0 - d as f64 / 180.0).clamp(0.0, 1.0) * 30.0);
    let score = (spread + concentration + recency).round() as u8;

    let mut violations = Vec::new();
    if score < t.min_score {
        violations.push(Violation {
            rule: "min_score",
            message: format!("health score {score} is below {}", t.min_score),
        });
    }
    if a.bus_factor.value < t.min_bus_factor {
        violations.push(Violation {
            rule: "min_bus_factor",
            message: format!(
                "bus factor {} is below {}",
                a.bus_factor.value, t.min_bus_factor
            ),
        });
    }
    if top_share > t.max_hotspot_share {
        let path = a.hotspots.first().map_or("", |h| h.path.as_str());
        violations.push(Violation {
            rule: "max_hotspot_share",
            message: format!(
                "{path} holds {:.0}% of all churn (limit {:.0}%)",
                top_share * 100.0,
                t.max_hotspot_share * 100.0
            ),
        });
    }
    match idle_days {
        Some(d) if d > t.max_days_since_commit => violations.push(Violation {
            rule: "max_days_since_commit",
            message: format!(
                "last commit was {d} days ago (limit {})",
                t.max_days_since_commit
            ),
        }),
        None => violations.push(Violation {
            rule: "max_days_since_commit",
            message: "no commits in range".into(),
        }),
        _ => {}
    }
    HealthReport { score, violations }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::tests::commit;
    use crate::analysis::{Options, analyze};
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap()
    }

    #[test]
    fn healthy_repository_passes() {
        let mut commits = Vec::new();
        for (i, who) in ["Ada", "Bob", "Cy", "Dee"].iter().enumerate() {
            for f in 0..5 {
                let path = format!("{i}-{f}.rs");
                commits.push(commit(who, 9, &[(path.as_str(), 10)]));
            }
        }
        let a = analyze(&commits, &Options::default());
        let r = evaluate(&a, &Thresholds::default(), now());
        assert!(r.passed(), "{:?}", r.violations);
        // spread 30 (bus factor 3) + concentration 28.5 + recency ~29.8
        assert_eq!(r.score, 88);
    }

    #[test]
    fn single_owner_stale_repo_fails_every_rule() {
        let commits = vec![commit("Ada", -200, &[("god.rs", 1000)])];
        let a = analyze(&commits, &Options::default());
        let r = evaluate(&a, &Thresholds::default(), now());
        let rules: Vec<_> = r.violations.iter().map(|v| v.rule).collect();
        assert_eq!(
            rules,
            [
                "min_score",
                "min_bus_factor",
                "max_hotspot_share",
                "max_days_since_commit"
            ]
        );
        assert_eq!(r.score, 10); // spread 10, concentration 0, recency 0
    }

    #[test]
    fn empty_history_is_reported() {
        let a = analyze(&[], &Options::default());
        let r = evaluate(
            &a,
            &Thresholds {
                min_score: 0,
                min_bus_factor: 0,
                ..Default::default()
            },
            now(),
        );
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.violations[0].rule, "max_days_since_commit");
    }
}
