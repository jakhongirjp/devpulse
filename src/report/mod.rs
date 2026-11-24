//! Output renderers. Each takes the same data; none of them computes anything.

mod markdown;
mod table;

use serde::Serialize;

use crate::analysis::{Analysis, HealthReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Table,
    Json,
    Markdown,
}

#[derive(Serialize)]
pub struct Report<'a> {
    pub repository: &'a str,
    pub health: &'a HealthReport,
    #[serde(flatten)]
    pub analysis: &'a Analysis,
}

pub fn render(report: &Report<'_>, format: Format) -> String {
    match format {
        Format::Json => {
            serde_json::to_string_pretty(report).expect("report is serializable") + "\n"
        }
        Format::Table => table::render(report),
        Format::Markdown => markdown::render(report),
    }
}

pub(crate) fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

pub(crate) fn date(d: Option<chrono::DateTime<chrono::Utc>>) -> String {
    d.map_or_else(|| "-".into(), |d| d.format("%Y-%m-%d").to_string())
}
