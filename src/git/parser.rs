use super::{FIELD_SEP, RECORD_SEP};
use crate::model::{Author, Commit, FileChange};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("commit header has {0} fields, expected 5")]
    Header(usize),
    #[error("invalid timestamp {0:?}")]
    Timestamp(String),
    #[error("invalid numstat line {0:?}")]
    Numstat(String),
}

/// Parses the output of `git log --numstat` with our record/field separators.
pub fn parse_log(raw: &str) -> Result<Vec<Commit>, ParseError> {
    raw.split(RECORD_SEP)
        .filter(|r| !r.trim().is_empty())
        .map(parse_record)
        .collect()
}

fn parse_record(record: &str) -> Result<Commit, ParseError> {
    let mut lines = record.lines();
    let header = lines.next().unwrap_or_default();
    let fields: Vec<&str> = header.split(FIELD_SEP).collect();
    let [hash, name, email, ts, subject] = fields[..] else {
        return Err(ParseError::Header(fields.len()));
    };
    let timestamp = ts
        .parse()
        .map_err(|_| ParseError::Timestamp(ts.to_string()))?;
    let changes = lines
        .filter(|l| !l.trim().is_empty())
        .map(parse_numstat)
        .collect::<Result<_, _>>()?;
    Ok(Commit {
        hash: hash.to_string(),
        author: Author {
            name: name.to_string(),
            email: email.to_lowercase(),
        },
        timestamp,
        subject: subject.to_string(),
        changes,
    })
}

fn parse_numstat(line: &str) -> Result<FileChange, ParseError> {
    let mut parts = line.splitn(3, '\t');
    let (Some(a), Some(d), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(ParseError::Numstat(line.to_string()));
    };
    let count = |s: &str| -> Result<Option<u64>, ParseError> {
        match s {
            "-" => Ok(None),
            n => n
                .parse()
                .map(Some)
                .map_err(|_| ParseError::Numstat(line.to_string())),
        }
    };
    Ok(FileChange {
        path: resolve_rename(path),
        added: count(a)?,
        deleted: count(d)?,
    })
}

/// Turns git's rename notation into the destination path:
/// `old => new` and `src/{old => new}/lib.rs` both resolve to the new name.
pub(crate) fn resolve_rename(path: &str) -> String {
    if let (Some(open), Some(close)) = (path.find('{'), path.rfind('}'))
        && let Some(arrow) = path[open..close].find(" => ")
    {
        let prefix = &path[..open];
        let new = &path[open + arrow + 4..close];
        let suffix = &path[close + 1..];
        // `{ => dir}/file` or `dir/{old => }/file` produce doubled slashes.
        return format!("{prefix}{new}{suffix}").replace("//", "/");
    }
    match path.split_once(" => ") {
        Some((_, new)) => new.to_string(),
        None => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(header: &str, body: &str) -> String {
        format!(
            "{RECORD_SEP}{}\n\n{body}\n",
            header.replace('|', &FIELD_SEP.to_string())
        )
    }

    #[test]
    fn parses_commits_and_numstat() {
        let raw = record(
            "abc|Ada Lovelace|ADA@Example.com|1700000000|Add engine",
            "10\t2\tsrc/engine.rs\n-\t-\tlogo.png",
        ) + &record(
            "def|Grace|grace@example.com|1700000100|Fix|pipe",
            "1\t1\tREADME.md",
        );
        // A '|' in the subject would be a 6th field: the second record is malformed on purpose.
        assert_eq!(parse_log(&raw), Err(ParseError::Header(6)));

        let raw = record(
            "abc|Ada Lovelace|ADA@Example.com|1700000000|Add engine",
            "10\t2\tsrc/engine.rs\n-\t-\tlogo.png",
        );
        let commits = parse_log(&raw).unwrap();
        assert_eq!(commits.len(), 1);
        let c = &commits[0];
        assert_eq!(c.author.email, "ada@example.com");
        assert_eq!(c.timestamp, 1_700_000_000);
        assert_eq!(
            c.changes[0],
            FileChange {
                path: "src/engine.rs".into(),
                added: Some(10),
                deleted: Some(2)
            }
        );
        assert_eq!(c.changes[1].added, None);
        assert_eq!(c.changes[1].churn(), 1);
    }

    #[test]
    fn commit_without_file_changes() {
        let commits = parse_log(&record("abc|A|a@x|1|empty", "")).unwrap();
        assert!(commits[0].changes.is_empty());
    }

    #[test]
    fn rejects_bad_numstat_and_timestamp() {
        assert!(matches!(
            parse_log(&record("a|b|c|nope|s", "")),
            Err(ParseError::Timestamp(_))
        ));
        assert!(matches!(
            parse_log(&record("a|b|c|1|s", "x\t1\tf")),
            Err(ParseError::Numstat(_))
        ));
    }

    #[test]
    fn resolves_renames() {
        assert_eq!(resolve_rename("old.rs => new.rs"), "new.rs");
        assert_eq!(resolve_rename("src/{a => b}/lib.rs"), "src/b/lib.rs");
        assert_eq!(
            resolve_rename("src/{ => nested}/lib.rs"),
            "src/nested/lib.rs"
        );
        assert_eq!(resolve_rename("src/{old => }/lib.rs"), "src/lib.rs");
        assert_eq!(resolve_rename("plain/path.rs"), "plain/path.rs");
    }
}
