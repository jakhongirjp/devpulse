use std::fmt::Write;

use super::{Report, date, pct};

/// GitHub-flavoured Markdown, suitable for a PR comment or job summary.
pub fn render(r: &Report<'_>) -> String {
    let a = r.analysis;
    let s = &a.summary;
    let status = if r.health.passed() {
        "✅ passing"
    } else {
        "⚠️ needs attention"
    };
    let mut out = String::new();
    let _ = writeln!(out, "## devpulse report: `{}`\n", r.repository);
    let _ = writeln!(
        out,
        "| Health | Bus factor | Commits | Authors | Files | Period |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    let _ = writeln!(
        out,
        "| **{}/100** {} | {} | {} | {} | {} | {} → {} |\n",
        r.health.score,
        status,
        a.bus_factor.value,
        s.commits,
        s.authors,
        s.files_touched,
        date(s.first_commit),
        date(s.last_commit)
    );
    out.push_str(
        "### Hotspots\n\n| File | Commits | Churn | Owner | Owned |\n|---|---:|---:|---|---:|\n",
    );
    for h in &a.hotspots {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} |",
            h.path,
            h.commits,
            h.churn,
            h.primary_owner.name,
            pct(h.ownership)
        );
    }
    if !r.health.violations.is_empty() {
        out.push_str("\n### Warnings\n\n");
        for v in &r.health.violations {
            let _ = writeln!(out, "- {}", v.message);
        }
    }
    out
}
