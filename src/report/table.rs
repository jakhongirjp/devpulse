use std::fmt::Write;

use super::{Report, date, pct};

/// Plain-text report for terminals. Column widths adapt to the content.
pub fn render(r: &Report<'_>) -> String {
    let a = r.analysis;
    let s = &a.summary;
    let mut out = String::new();
    let _ = writeln!(out, "devpulse · {}", r.repository);
    let _ = writeln!(
        out,
        "health score {}/100  ·  bus factor {}\n",
        r.health.score, a.bus_factor.value
    );
    let _ = writeln!(
        out,
        "{} · {} · {} · +{} −{} · {} → {} ({} active days)\n",
        plural(s.commits, "commit"),
        plural(s.authors, "author"),
        plural(s.files_touched, "file"),
        s.lines_added,
        s.lines_deleted,
        date(s.first_commit),
        date(s.last_commit),
        s.active_days
    );

    out.push_str("Hotspots\n");
    let rows: Vec<Vec<String>> = a
        .hotspots
        .iter()
        .map(|h| {
            vec![
                h.path.clone(),
                h.commits.to_string(),
                h.churn.to_string(),
                h.primary_owner.name.clone(),
                pct(h.ownership),
            ]
        })
        .collect();
    out.push_str(&table(
        &["file", "commits", "churn", "owner", "owned"],
        &rows,
    ));

    out.push_str("\nAuthors\n");
    let rows: Vec<Vec<String>> = a
        .authors
        .iter()
        .map(|x| {
            vec![
                x.author.name.clone(),
                x.commits.to_string(),
                format!("+{} −{}", x.lines_added, x.lines_deleted),
                pct(x.commit_share),
            ]
        })
        .collect();
    out.push_str(&table(&["author", "commits", "lines", "share"], &rows));

    if !r.health.violations.is_empty() {
        out.push_str("\nWarnings\n");
        for v in &r.health.violations {
            let _ = writeln!(out, "  ! {}", v.message);
        }
    }
    out
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = (0..headers.len())
        .map(|i| {
            rows.iter()
                .map(|r| r[i].chars().count())
                .chain([headers[i].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: Vec<&str>| {
        let parts: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c:<w$}"))
            .collect();
        format!("  {}\n", parts.join("  ").trim_end())
    };
    let mut out = line(headers.to_vec());
    for r in rows {
        out.push_str(&line(r.iter().map(String::as_str).collect()));
    }
    if rows.is_empty() {
        out.push_str("  (none)\n");
    }
    out
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}
