//! Golden-file tests for the markdown briefing. Refresh with
//! `UPDATE_EXPECT=1 cargo test --test markdown_snapshots` and review the diff.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn guided_rust_snapshot() {
    assert_snapshot("guided_rust", &["--no-git"], "guided_rust.md");
}

#[test]
fn guided_rust_tight_budget_snapshot() {
    assert_snapshot(
        "guided_rust",
        &["--no-git", "--max-bytes", "700"],
        "guided_rust_tight_budget.md",
    );
}

#[test]
fn no_readme_snapshot() {
    assert_snapshot("no_readme_rust", &["--no-git"], "no_readme_rust.md");
}

fn assert_snapshot(fixture: &str, args: &[&str], snapshot: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture);
    let output = Command::new(env!("CARGO_BIN_EXE_context-pack"))
        .arg("--cwd")
        .arg(&root)
        .args(args)
        .output()
        .expect("failed to run context-pack");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout)
        .expect("utf-8 output")
        .replace(env!("CARGO_PKG_VERSION"), "<VERSION>");

    let path = snapshot_path(snapshot);
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        fs::write(&path, &actual).expect("failed to write snapshot");
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "snapshot {snapshot} changed; rerun with UPDATE_EXPECT=1 and review the diff"
    );
}

fn snapshot_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(name)
}
