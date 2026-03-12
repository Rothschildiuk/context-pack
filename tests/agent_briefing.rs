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

#[test]
fn low_signal_git_noise_is_filtered_from_active_work() {
    let temp = TempDir::new("briefing-git-noise");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    git(temp.path(), &["init"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test User"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "init"]);

    write_file(temp.path(), ".idea/workspace.xml", "<xml />\n");

    let output = run_pack(temp.path(), &[]);

    assert!(output.contains("No high-signal changes detected."));
    assert!(!output.contains(".idea/workspace.xml"));
}

#[test]
fn fallback_detection_finds_c_and_coq_projects() {
    let temp = TempDir::new("briefing-c-coq");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(temp.path(), "C/Makefile", "all:\n\tcc main.c\n");
    write_file(temp.path(), "C/main.c", "int main(void) { return 0; }\n");
    write_file(
        temp.path(),
        "Coq/demo.v",
        "Theorem demo : True.\nProof. exact I. Qed.\n",
    );

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("project types: c, coq"));
    assert!(output.contains("primary languages: c, coq"));
    assert!(output
        .contains("Likely a low-level language or formal methods project with C and Coq code."));
}

#[test]
fn large_code_files_prioritize_production_code_over_tests() {
    let temp = TempDir::new("briefing-large-code");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "src/engine.py",
        &repeat_lines("def important_step():\n    return 1\n", 30),
    );
    write_file(
        temp.path(),
        "tests/test_engine.py",
        &repeat_lines("def test_step():\n    assert True\n", 60),
    );

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("### Large Code Files"));
    assert!(output.contains("`src/engine.py`"));
    assert!(!output.contains("`tests/test_engine.py`"));
}

#[test]
fn json_output_is_structured_and_not_a_stub() {
    let temp = TempDir::new("briefing-json");
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

    let output = run_pack(temp.path(), &["--no-git", "--format", "json"]);

    assert!(output.starts_with("{\n"));
    assert!(output.contains("\"briefing\": {"));
    assert!(output.contains("\"repo\": {"));
    assert!(output.contains("\"git\": {"));
    assert!(output.contains("\"important_files\": ["));
    assert!(output.contains("\"path\": \"README.md\""));
    assert!(!output.contains("\"not_implemented\""));
}

#[test]
fn javascript_repo_detects_javascript_and_surfaces_entrypoint() {
    let temp = TempDir::new("briefing-javascript");
    write_file(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");
    write_file(temp.path(), "src/index.js", "console.log('demo');\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("project types: node"));
    assert!(output.contains("primary languages: javascript"));
    assert!(output.contains("### src/index.js"));
    assert!(output.contains("`src/index.js`: entrypoint-like source file"));
}

#[test]
fn changed_only_hides_unchanged_large_code_files() {
    let temp = TempDir::new("briefing-changed-only-large-files");
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
        "src/lib.rs",
        &repeat_lines("pub fn helper() {}\n", 80),
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

    let output = run_pack(temp.path(), &["--changed-only"]);

    assert!(output.contains("- modified `src/main.rs`"));
    assert!(!output.contains("`src/lib.rs` (80 LOC)"));
}

#[test]
fn truncated_makefile_excerpt_keeps_recipe_lines() {
    let temp = TempDir::new("briefing-makefile-excerpt");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "Makefile",
        concat!(
            ".PHONY: help setup build test run lint fmt doctor release ci clean\n",
            "help:\n\t@echo help\n",
            "setup:\n\t@echo setup\n",
            "build:\n\t@echo build\n",
            "test:\n\t@echo test\n",
            "run:\n\t@echo run\n",
            "lint:\n\t@echo lint\n",
            "fmt:\n\t@echo fmt\n",
            "doctor:\n\t@echo doctor\n",
            "release:\n\t@echo release\n",
            "ci:\n\t@echo ci\n",
            "clean:\n\t@echo clean\n"
        ),
    );

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("### Makefile"));
    assert!(output.contains("\t@echo help"));
    assert!(output.contains("\t@echo setup"));
}

#[test]
fn mixed_java_and_node_repo_is_detected_as_monolith() {
    let temp = TempDir::new("briefing-java-node");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Monolith\n\nReal project overview.\n",
    );
    write_file(
        temp.path(),
        "src/dblayer/pom.xml",
        "<project><modelVersion>4.0.0</modelVersion></project>\n",
    );
    write_file(
        temp.path(),
        "src/editorprovider/build.gradle",
        "plugins { id 'java' }\nrepositories { mavenCentral() }\n",
    );
    write_file(
        temp.path(),
        "src/fonto/package.json",
        "{\n  \"name\": \"fonto-app\"\n}\n",
    );
    write_file(temp.path(), "src/fonto/index.js", "console.log('ui');\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("Likely a mixed Java and Node monolith"));
    assert!(
        output.contains("project types: java, node")
            || output.contains("project types: node, java")
    );
    assert!(
        output.contains("primary languages: java, javascript")
            || output.contains("primary languages: javascript, java")
    );
}

#[test]
fn placeholder_readme_is_deprioritized_below_real_manifests() {
    let temp = TempDir::new("briefing-placeholder-readme");
    write_file(
        temp.path(),
        "README.md",
        concat!(
            "# <Title>\n\n",
            "## About\n\n<Header>\n\n",
            "## Contacts\n\n- **<Role>**: <Team>\n\n",
            "## Resources\n\n- **<Repository>**: [<Repository>](<URL>)\n\n",
            "## Usage\n\n<Usage>\n\n",
            "## Tests\n\n<Tests>\n"
        ),
    );
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("placeholder-heavy template"));
    assert_before(&output, "`Cargo.toml`", "`README.md`");
}

#[test]
fn vendor_like_directories_are_filtered_from_large_code_files() {
    let temp = TempDir::new("briefing-vendor-suppression");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "src/configureSxModule.js",
        &repeat_lines("export const configure = () => true;\n", 120),
    );
    write_file(
        temp.path(),
        "src/platform/fontoxml-vendors/src/react-dom.js",
        &repeat_lines("export const vendor = () => true;\n", 400),
    );

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("`src/configureSxModule.js`"));
    assert!(!output.contains("react-dom.js"));
}

#[test]
fn dockerfiles_and_compose_files_are_selected_as_build_signals() {
    let temp = TempDir::new("briefing-docker-signals");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "docker-compose.yaml",
        "services:\n  app:\n    build: .\n",
    );
    write_file(
        temp.path(),
        "Dockerfile.App",
        "FROM alpine:3.20\nRUN echo demo\n",
    );

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("### docker-compose.yaml"));
    assert!(output.contains("### Dockerfile.App"));
    assert!(output.contains("`docker-compose.yaml`: build or orchestration entrypoint"));
    assert!(output.contains("`Dockerfile.App`: build or orchestration entrypoint"));
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

fn repeat_lines(line: &str, times: usize) -> String {
    std::iter::repeat_n(line, times).collect::<String>()
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
