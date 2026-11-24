//! Core value types shared by the git reader, the analysis and the renderers.

use serde::Serialize;

/// One non-merge commit and the files it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub author: Author,
    /// Seconds since the Unix epoch (author date).
    pub timestamp: i64,
    pub subject: String,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Line counts for one file in one commit. Binary files have no line counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub added: Option<u64>,
    pub deleted: Option<u64>,
}

impl FileChange {
    /// Lines touched; binary changes count as one so they still register.
    pub fn churn(&self) -> u64 {
        match (self.added, self.deleted) {
            (Some(a), Some(d)) => a + d,
            _ => 1,
        }
    }
}
