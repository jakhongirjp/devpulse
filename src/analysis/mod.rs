//! Metrics computed from commit history. Everything here is pure: it takes
//! commits and returns numbers, which keeps it fast to test and easy to trust.

mod health;

pub use health::{HealthReport, Thresholds, Violation, evaluate as health_eval};

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Datelike, Utc};
use globset::GlobSet;
use serde::Serialize;

use crate::model::{Author, Commit};

#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub summary: Summary,
    pub authors: Vec<AuthorStats>,
    pub hotspots: Vec<Hotspot>,
    pub bus_factor: BusFactor,
    pub weekly_activity: Vec<WeekActivity>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Summary {
    pub commits: usize,
    pub authors: usize,
    pub files_touched: usize,
    pub lines_added: u64,
    pub lines_deleted: u64,
    pub first_commit: Option<DateTime<Utc>>,
    pub last_commit: Option<DateTime<Utc>>,
    pub active_days: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorStats {
    pub author: Author,
    pub commits: usize,
    pub lines_added: u64,
    pub lines_deleted: u64,
    /// Share of all commits, 0.0–1.0.
    pub commit_share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hotspot {
    pub path: String,
    pub commits: usize,
    pub churn: u64,
    pub authors: usize,
    /// Change frequency weighted by size: commits × ln(1 + churn).
    pub score: f64,
    pub primary_owner: Author,
    /// Share of this file's churn written by the primary owner, 0.0–1.0.
    pub ownership: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BusFactor {
    /// Smallest number of people who are primary owners of more than half of the files.
    pub value: usize,
    pub key_people: Vec<Author>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WeekActivity {
    /// ISO week label, e.g. "2026-W07".
    pub week: String,
    pub commits: usize,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub top: usize,
    pub exclude: GlobSet,
    pub weeks: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            top: 10,
            exclude: GlobSet::empty(),
            weeks: 12,
        }
    }
}

#[derive(Default)]
struct FileAcc {
    commits: usize,
    churn: u64,
    by_author: HashMap<Author, u64>,
}

/// Computes every metric in one pass over the commits.
pub fn analyze(commits: &[Commit], opts: &Options) -> Analysis {
    let mut summary = Summary::default();
    let mut authors: BTreeMap<Author, AuthorStats> = BTreeMap::new();
    let mut files: HashMap<&str, FileAcc> = HashMap::new();
    let mut days = BTreeSet::new();
    let mut weeks: BTreeMap<(i32, u32), usize> = BTreeMap::new();

    for c in commits {
        let changes: Vec<_> = c
            .changes
            .iter()
            .filter(|f| !opts.exclude.is_match(&f.path))
            .collect();
        let when = DateTime::from_timestamp(c.timestamp, 0).unwrap_or_default();
        summary.commits += 1;
        summary.first_commit = Some(summary.first_commit.map_or(when, |f| f.min(when)));
        summary.last_commit = Some(summary.last_commit.map_or(when, |l| l.max(when)));
        days.insert(when.date_naive());
        let iso = when.iso_week();
        *weeks.entry((iso.year(), iso.week())).or_default() += 1;

        let a = authors
            .entry(c.author.clone())
            .or_insert_with(|| AuthorStats {
                author: c.author.clone(),
                commits: 0,
                lines_added: 0,
                lines_deleted: 0,
                commit_share: 0.0,
            });
        a.commits += 1;
        for f in changes {
            let added = f.added.unwrap_or(0);
            let deleted = f.deleted.unwrap_or(0);
            a.lines_added += added;
            a.lines_deleted += deleted;
            summary.lines_added += added;
            summary.lines_deleted += deleted;
            let acc = files.entry(f.path.as_str()).or_default();
            acc.commits += 1;
            acc.churn += f.churn();
            *acc.by_author.entry(c.author.clone()).or_default() += f.churn();
        }
    }

    summary.authors = authors.len();
    summary.files_touched = files.len();
    summary.active_days = days.len();

    let mut authors: Vec<_> = authors.into_values().collect();
    for a in &mut authors {
        a.commit_share = ratio(a.commits as u64, summary.commits as u64);
    }
    authors.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then_with(|| a.author.cmp(&b.author))
    });

    let mut hotspots: Vec<Hotspot> = files
        .into_iter()
        .map(|(path, acc)| {
            let (owner, owned) = primary_owner(&acc.by_author);
            Hotspot {
                path: path.to_string(),
                commits: acc.commits,
                churn: acc.churn,
                authors: acc.by_author.len(),
                score: acc.commits as f64 * (1.0 + acc.churn as f64).ln(),
                ownership: ratio(owned, acc.churn),
                primary_owner: owner,
            }
        })
        .collect();
    hotspots.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    let bus_factor = bus_factor(&hotspots);
    hotspots.truncate(opts.top);

    let weekly_activity = weeks
        .into_iter()
        .rev()
        .take(opts.weeks)
        .rev()
        .map(|((y, w), commits)| WeekActivity {
            week: format!("{y}-W{w:02}"),
            commits,
        })
        .collect();

    Analysis {
        summary,
        authors,
        hotspots,
        bus_factor,
        weekly_activity,
    }
}

/// Author with the most churn on a file; ties break alphabetically for stable output.
fn primary_owner(by_author: &HashMap<Author, u64>) -> (Author, u64) {
    by_author
        .iter()
        .max_by(|(a, x), (b, y)| x.cmp(y).then_with(|| b.cmp(a)))
        .map(|(a, n)| (a.clone(), *n))
        .expect("a file in the map has at least one author")
}

/// Classic "truck factor" heuristic over primary file ownership. Must run on
/// the full, untruncated file list.
fn bus_factor(files: &[Hotspot]) -> BusFactor {
    let mut owned: HashMap<&Author, usize> = HashMap::new();
    for f in files {
        *owned.entry(&f.primary_owner).or_default() += 1;
    }
    let mut ranked: Vec<_> = owned.into_iter().collect();
    ranked.sort_by(|(a, x), (b, y)| y.cmp(x).then_with(|| a.cmp(b)));

    let mut covered = 0;
    let mut key_people = Vec::new();
    for (author, n) in ranked {
        if covered * 2 > files.len() {
            break;
        }
        covered += n;
        key_people.push(author.clone());
    }
    BusFactor {
        value: key_people.len(),
        key_people,
    }
}

fn ratio(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::FileChange;

    pub fn author(name: &str) -> Author {
        Author {
            name: name.into(),
            email: format!("{}@example.com", name.to_lowercase()),
        }
    }

    pub fn commit(who: &str, day: i64, files: &[(&str, u64)]) -> Commit {
        Commit {
            hash: format!("{who}{day}"),
            author: author(who),
            timestamp: 1_767_225_600 + day * 86_400, // 2026-01-01
            subject: "change".into(),
            changes: files
                .iter()
                .map(|(p, n)| FileChange {
                    path: (*p).into(),
                    added: Some(*n),
                    deleted: Some(0),
                })
                .collect(),
        }
    }

    #[test]
    fn empty_history() {
        let a = analyze(&[], &Options::default());
        assert_eq!(a.summary.commits, 0);
        assert_eq!(a.bus_factor.value, 0);
        assert!(a.hotspots.is_empty());
    }

    #[test]
    fn summary_and_authors() {
        let commits = vec![
            commit("Ada", 0, &[("a.rs", 10)]),
            commit("Ada", 0, &[("b.rs", 5)]),
            commit("Bob", 8, &[("a.rs", 1)]),
        ];
        let a = analyze(&commits, &Options::default());
        assert_eq!(a.summary.commits, 3);
        assert_eq!(a.summary.files_touched, 2);
        assert_eq!(a.summary.lines_added, 16);
        assert_eq!(a.summary.active_days, 2);
        assert_eq!(a.authors[0].author.name, "Ada");
        assert!((a.authors[0].commit_share - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(a.weekly_activity.len(), 2);
    }

    #[test]
    fn hotspots_rank_frequency_and_size() {
        let commits = vec![
            commit("Ada", 0, &[("hot.rs", 50)]),
            commit("Ada", 1, &[("hot.rs", 50)]),
            commit("Bob", 2, &[("hot.rs", 10), ("cold.rs", 500)]),
        ];
        let a = analyze(&commits, &Options::default());
        assert_eq!(a.hotspots[0].path, "hot.rs");
        assert_eq!(a.hotspots[0].primary_owner.name, "Ada");
        assert!((a.hotspots[0].ownership - 100.0 / 110.0).abs() < 1e-9);
        assert_eq!(a.hotspots[0].authors, 2);
    }

    #[test]
    fn bus_factor_counts_people_owning_majority_of_files() {
        // Ada owns 3 files, Bob 1, Cy 1: Ada alone owns more than half.
        let commits = vec![
            commit("Ada", 0, &[("a", 1), ("b", 1), ("c", 1)]),
            commit("Bob", 0, &[("d", 1)]),
            commit("Cy", 0, &[("e", 1)]),
        ];
        let a = analyze(&commits, &Options::default());
        assert_eq!(a.bus_factor.value, 1);
        assert_eq!(a.bus_factor.key_people[0].name, "Ada");

        // Evenly spread: two of three people are needed.
        let commits = vec![
            commit("Ada", 0, &[("a", 1), ("b", 1)]),
            commit("Bob", 0, &[("c", 1), ("d", 1)]),
            commit("Cy", 0, &[("e", 1), ("f", 1)]),
        ];
        assert_eq!(analyze(&commits, &Options::default()).bus_factor.value, 2);
    }

    #[test]
    fn excludes_and_truncates() {
        let mut set = globset::GlobSetBuilder::new();
        set.add(globset::Glob::new("**/*.lock").unwrap());
        let opts = Options {
            top: 1,
            exclude: set.build().unwrap(),
            weeks: 12,
        };
        let commits = vec![commit(
            "Ada",
            0,
            &[("Cargo.lock", 9000), ("src/a.rs", 1), ("src/b.rs", 2)],
        )];
        let a = analyze(&commits, &opts);
        assert_eq!(a.summary.files_touched, 2);
        assert_eq!(a.hotspots.len(), 1);
        assert_eq!(a.summary.lines_added, 3);
    }
}
