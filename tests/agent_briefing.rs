use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn briefing_is_first_and_prefers_guidance_files() {
    let temp = TempDir::new("briefing-guidance");
    write_file(
        temp.path(),
        "AGENTS.md",
        "# Agent Rules\n\nRead this first.\n",
    );
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert_before(&output, "## Agent Briefing", "## Repo");
    assert_before(&output, "`AGENTS.md`", "`README.md`");
    assert_before(&output, "`README.md`", "`Cargo.toml`");
    assert_contains_heading(&output, "AGENTS.md");
    assert_contains_heading(&output, "README.md");
    assert_contains_heading(&output, "Cargo.toml");
}

#[test]
fn changed_source_is_reflected_in_active_work_and_read_order() {
    let temp = TempDir::new("briefing-changed-source");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(
        temp.path(),
        "src/main.rs",
        "fn main() {\n    println!(\"v1\");\n}\n",
    );

    git(temp.path(), &["init"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test User"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "init"]);

    write_file(
        temp.path(),
        "src/main.rs",
        "fn main() {\n    println!(\"v2\");\n}\n",
    );

    let output = run_pack(temp.path(), &[]);

    assert!(output.contains("- modified `src/main.rs`"));
    assert!(output.contains("`src/main.rs`: changed source file, active work, likely entry point"));
}

#[test]
fn non_git_repo_reports_git_unavailable() {
    let temp = TempDir::new("briefing-non-git");
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &[]);

    assert!(output.contains("Git context unavailable."));
    assert!(output.contains("- Git context unavailable."));
}

#[test]
fn repo_without_readme_falls_back_to_manifest_and_entrypoint() {
    let temp = TempDir::new("briefing-no-readme");
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("- No README found."));
    assert!(output.contains("`Cargo.toml`: project manifest"));
    assert!(output.contains("`src/main.rs`: entrypoint-like source file"));
}

#[test]
fn noisy_files_and_lockfiles_are_filtered_from_important_files() {
    let temp = TempDir::new("briefing-noise");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "Cargo.lock", "noise\n");
    write_file(temp.path(), "package-lock.json", "{ }\n");
    write_file(temp.path(), ".secret", "hidden\n");
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(!output.contains("### Cargo.lock"));
    assert!(!output.contains("### package-lock.json"));
    assert!(!output.contains("### .secret"));
}

#[test]
fn tight_budget_keeps_briefing_and_trims_tree() {
    let temp = TempDir::new("briefing-tight-budget");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    for index in 0..20 {
        write_file(
            temp.path(),
            &format!("src/file_{index}.rs"),
            "pub fn demo() {}\n",
        );
    }
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git", "--max-bytes", "700"]);

    assert!(output.contains("## Agent Briefing"));
    assert!(output.contains("tree summary truncated to budget"));
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

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("failed to run git");

    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_file(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directory");
    }
    fs::write(path, content).expect("failed to write file");
}

fn assert_before(output: &str, left: &str, right: &str) {
    let left_index = output
        .find(left)
        .unwrap_or_else(|| panic!("missing {left}"));
    let right_index = output
        .find(right)
        .unwrap_or_else(|| panic!("missing {right}"));
    assert!(
        left_index < right_index,
        "{left} should appear before {right}"
    );
}

fn assert_contains_heading(output: &str, path: &str) {
    assert!(output.contains(&format!("### {path}")));
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("context-pack-{prefix}-{nonce}"));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
