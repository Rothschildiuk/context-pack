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

#[test]
fn c_coq_snapshot() {
    assert_snapshot("c_coq", &["--no-git"], "c_coq.md");
}

fn assert_snapshot(fixture_name: &str, args: &[&str], snapshot_name: &str) {
    let fixture_root = fixture_dir(fixture_name);
    let output = run_pack(&fixture_root, args);
    let normalized = normalize_output(&output, &fixture_root);
    let expected = fs::read_to_string(snapshot_dir().join(snapshot_name))
        .expect("failed to read snapshot file");
    let expected = normalize_snapshot(&expected);

    assert_eq!(
        normalized, expected,
        "snapshot mismatch for {snapshot_name}"
    );
}

fn run_pack(repo: &Path, args: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_context-pack"));
    command.arg("--cwd").arg(repo);
    command.args(args);

    let output = command.output().expect("failed to run context-pack");
    assert!(
        output.status.success(),
        "context-pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn normalize_output(output: &str, fixture_root: &Path) -> String {
    let path_normalized = output.replace(
        fixture_root
            .to_str()
            .expect("fixture path should be valid utf-8"),
        "<FIXTURE_ROOT>",
    );

    normalize_snapshot(&path_normalized)
}

fn normalize_snapshot(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with("- elapsed_ms: "))
        .map(|line| {
            if line.starts_with("- approx tokens: ") {
                "- approx tokens: <APPROX_TOKENS>"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}
