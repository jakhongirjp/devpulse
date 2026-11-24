//! End-to-end tests: build a real git repository, run the binary against it.

use std::path::Path;
use std::process::Command as Process;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str], author: (&str, &str)) {
    let status = Process::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", author.0)
        .env("GIT_AUTHOR_EMAIL", author.1)
        .env("GIT_COMMITTER_NAME", author.0)
        .env("GIT_COMMITTER_EMAIL", author.1)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

fn write_and_commit(dir: &Path, file: &str, body: &str, author: (&str, &str)) {
    let path = dir.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
    git(dir, &["add", "-A"], author);
    git(
        dir,
        &["commit", "-q", "-m", &format!("update {file}")],
        author,
    );
}

const ADA: (&str, &str) = ("Ada", "ada@example.com");
const BOB: (&str, &str) = ("Bob", "bob@example.com");

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"], ADA);
    for i in 0..5 {
        write_and_commit(dir.path(), "src/engine.rs", &"x\n".repeat(10 + i * 10), ADA);
    }
    write_and_commit(dir.path(), "src/api.rs", "fn api() {}\n", BOB);
    write_and_commit(dir.path(), "Cargo.lock", &"dep\n".repeat(5000), BOB);
    // A rename must keep history under the new name.
    git(dir.path(), &["mv", "src/api.rs", "src/http.rs"], BOB);
    git(dir.path(), &["commit", "-q", "-m", "rename"], BOB);
    dir
}

fn devpulse() -> Command {
    Command::cargo_bin("devpulse").unwrap()
}

#[test]
fn json_report_has_expected_metrics() {
    let repo = fixture();
    let out = devpulse()
        .args(["report", "--format", "json", "--repo"])
        .arg(repo.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    assert_eq!(v["summary"]["commits"], 8);
    assert_eq!(v["summary"]["authors"], 2);
    assert_eq!(v["hotspots"][0]["path"], "src/engine.rs");
    assert_eq!(v["hotspots"][0]["primary_owner"]["name"], "Ada");
    // Lockfiles are excluded by the default config.
    let paths: Vec<_> = v["hotspots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["path"].as_str().unwrap())
        .collect();
    assert!(!paths.contains(&"Cargo.lock"), "{paths:?}");
    assert!(
        paths.contains(&"src/http.rs"),
        "rename not followed: {paths:?}"
    );
    assert!(v["health"]["score"].as_u64().unwrap() <= 100);
}

#[test]
fn table_and_markdown_render() {
    let repo = fixture();
    devpulse()
        .args(["report", "--repo"])
        .arg(repo.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Hotspots").and(predicate::str::contains("src/engine.rs")),
        );
    devpulse()
        .args(["report", "--format", "markdown", "--repo"])
        .arg(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("| `src/engine.rs` |"));
}

#[test]
fn check_fails_on_thresholds_and_passes_when_relaxed() {
    let repo = fixture();
    std::fs::write(
        repo.path().join(".devpulse.toml"),
        "[check]\nmin_bus_factor = 5\n",
    )
    .unwrap();
    devpulse()
        .args(["check", "--repo"])
        .arg(repo.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("bus factor"));

    std::fs::write(
        repo.path().join(".devpulse.toml"),
        "[check]\nmin_score = 0\nmin_bus_factor = 1\nmax_hotspot_share = 1.0\n",
    )
    .unwrap();
    devpulse()
        .args(["check", "--repo"])
        .arg(repo.path())
        .assert()
        .success();
}

#[test]
fn init_writes_config_once() {
    let repo = fixture();
    devpulse()
        .args(["init", "--repo"])
        .arg(repo.path())
        .assert()
        .success();
    assert!(repo.path().join(".devpulse.toml").exists());
    devpulse()
        .args(["init", "--repo"])
        .arg(repo.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn path_filter_and_errors() {
    let repo = fixture();
    let out = devpulse()
        .args([
            "report",
            "--format",
            "json",
            "--path",
            "src/http.rs",
            "--repo",
        ])
        .arg(repo.path())
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["summary"]["files_touched"], 1);

    let not_git = TempDir::new().unwrap();
    devpulse()
        .args(["report", "--repo"])
        .arg(not_git.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a git repository"));
}

#[test]
fn empty_repository_is_not_an_error() {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"], ADA);
    devpulse()
        .args(["report", "--format", "json", "--repo"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"commits\": 0"));
}
