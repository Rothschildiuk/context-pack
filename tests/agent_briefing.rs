//! End-to-end behaviour of the CLI on small synthetic repositories. Each test
//! encodes a failure mode seen on real projects.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// What the briefing surfaces

#[test]
fn instruction_files_for_every_major_agent_are_listed_first() {
    let repo = TempRepo::new("instructions");
    repo.write("AGENTS.md", "# Rules\nRun make test before committing.\n");
    repo.write("CLAUDE.md", "See AGENTS.md\n");
    repo.write(".cursor/rules/style.mdc", "Prefer small functions.\n");
    repo.write(".github/copilot-instructions.md", "Use tabs.\n");
    repo.write("services/api/AGENTS.md", "API-specific rules.\n");
    repo.write("tests/fixtures/sample/AGENTS.md", "fixture, not guidance\n");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");

    let output = repo.run(&["--no-git"]);
    let section = section(&output, "## Agent instructions");
    assert_order(section, &["`AGENTS.md`", "`CLAUDE.md`"]);
    assert!(section.contains("`.cursor/rules/style.mdc` — Cursor rule"));
    assert!(section.contains("`.github/copilot-instructions.md` — GitHub Copilot instructions"));
    assert!(section
        .contains("`services/api/AGENTS.md` — agent instructions (applies to `services/api/`)"));
    assert!(!section.contains("fixtures"), "{section}");
    assert_order(
        &output,
        &["## Agent instructions", "## Commands", "## Entry points"],
    );
}

#[test]
fn commands_come_from_task_runners_scripts_and_ci() {
    let repo = TempRepo::new("commands");
    repo.write("package.json", r#"{"name":"web","scripts":{"test":"vitest run","lint":"eslint .","pretest":"echo hi","release":"np"}}"#);
    repo.write("pnpm-lock.yaml", "lockfileVersion: 9\n");
    repo.write(
        "Makefile",
        ".PHONY: build\nbuild:\n\t@echo building\n\tpnpm tsc -b\n",
    );
    repo.write(
        ".github/workflows/ci.yml",
        "jobs:\n  test:\n    steps:\n      - uses: actions/checkout@v4\n      - run: pnpm install --frozen-lockfile\n      - run: |\n          echo start\n          pnpm test\n        shell: bash\n      - run: pnpm run e2e\n",
    );
    repo.write(
        ".github/workflows/release.yml",
        "jobs:\n  a:\n    steps:\n      - run: pnpm publish\n",
    );

    let output = repo.run(&["--no-git"]);
    let section = section(&output, "## Commands");
    assert!(
        section.contains("- build: `make build` — Makefile: pnpm tsc -b"),
        "{section}"
    );
    assert!(
        section.contains("- test: `pnpm test` — package.json: vitest run"),
        "{section}"
    );
    assert!(
        section.contains("- lint: `pnpm run lint` — package.json: eslint ."),
        "{section}"
    );
    assert!(section.contains("- ci: `pnpm install --frozen-lockfile` — .github/workflows/ci.yml"));
    assert!(section.contains("- ci: `pnpm run e2e`"));
    assert!(
        !section.contains("pretest")
            && !section.contains("release")
            && !section.contains("publish")
    );
    assert!(!section.contains("shell: bash") && !section.contains("echo"));
}

#[test]
fn ecosystem_defaults_fill_in_when_no_scripts_exist() {
    let repo = TempRepo::new("defaults");
    repo.write(
        "pyproject.toml",
        "[project]\nname = \"svc\"\n[tool.ruff]\nline-length = 100\n[tool.pytest.ini_options]\n",
    );
    repo.write("uv.lock", "version = 1\n");
    repo.write("svc/__init__.py", "");

    let output = repo.run(&["--no-git"]);
    assert!(
        output.contains("- test: `uv run pytest` — python default"),
        "{output}"
    );
    assert!(output.contains("- lint: `uv run ruff check .`"));
    assert!(output.contains("- Stack: python (uv)"));
}

#[test]
fn declared_entry_points_beat_file_name_conventions() {
    let repo = TempRepo::new("entry");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"rg\"\ndescription = \"Search tool\"\n[[bin]]\nname = \"rg\"\npath = \"crates/core/main.rs\"\n[workspace]\nmembers = [\"crates/*\"]\n",
    );
    repo.write("crates/core/main.rs", "mod search;\nfn main() {}\n");
    repo.write("crates/core/search.rs", &"pub fn search() {}\n".repeat(40));
    repo.write("crates/util/Cargo.toml", "[package]\nname = \"util\"\n");
    repo.write("crates/util/src/lib.rs", "pub fn helper() {}\n");
    repo.write("examples/demo/main.rs", "fn main() {}\n");

    let output = repo.run(&["--no-git"]);
    let entries = section(&output, "## Entry points");
    assert!(
        entries
            .starts_with("## Entry points\n- `crates/core/main.rs` — cargo bin `rg` (Cargo.toml)"),
        "{entries}"
    );
    assert!(!entries.contains("examples/"), "{entries}");
    assert!(
        section(&output, "## Key files")
            .contains("`crates/core/search.rs` — used by the main entry point"),
        "{output}"
    );
    assert!(output.contains("> Search tool"));
}

#[test]
fn package_code_is_found_even_when_example_trees_sort_first() {
    // fastapi-style: thousands of example files alphabetically before the package.
    let repo = TempRepo::new("crowded");
    repo.write("pyproject.toml", "[project]\nname = \"fastlib\"\ndescription = \"Fast library\"\n[project.scripts]\nfastlib = \"fastlib.cli:main\"\n");
    for index in 0..300 {
        repo.write(&format!("docs_src/tutorial{index:03}/main.py"), "app = 1\n");
        repo.write(&format!("docs/en/page{index:03}.md"), "# Page\n");
    }
    repo.write("fastlib/__init__.py", "from .routing import Router\n");
    repo.write("fastlib/cli.py", "def main():\n    pass\n");
    repo.write(
        "fastlib/routing.py",
        &"def route():\n    return 1\n".repeat(400),
    );
    repo.write(
        "fastlib/applications.py",
        &"class App:\n    pass\n".repeat(300),
    );

    let output = repo.run(&["--no-git"]);
    assert!(
        section(&output, "## Entry points")
            .contains("`fastlib/cli.py` — console script `fastlib` (pyproject.toml)"),
        "{output}"
    );
    let key_files = section(&output, "## Key files");
    assert_order(
        key_files,
        &["`fastlib/routing.py`", "`fastlib/applications.py`"],
    );
    assert!(!key_files.contains("docs_src"), "{key_files}");
    assert!(
        section(&output, "## Layout").contains("- `docs_src/` — 300 file(s), python [examples]"),
        "{output}"
    );
}

#[test]
fn maven_package_paths_are_not_mistaken_for_examples() {
    let repo = TempRepo::new("maven");
    repo.write("pom.xml", "<project><parent><artifactId>spring-boot-starter-parent</artifactId></parent><artifactId>petclinic</artifactId><dependencies><dependency><artifactId>spring-boot-starter-web</artifactId></dependency></dependencies></project>");
    repo.write("mvnw", "#!/bin/sh\n");
    let base = "src/main/java/org/springframework/samples/petclinic";
    repo.write(
        &format!("{base}/PetClinicApplication.java"),
        "public class PetClinicApplication { public static void main(String[] a) {} }\n",
    );
    repo.write(
        &format!("{base}/owner/OwnerController.java"),
        &"public void handle() {}\n".repeat(30),
    );
    repo.write(
        "src/test/java/org/example/OwnerTest.java",
        "class OwnerTest {}\n",
    );

    let output = repo.run(&["--no-git"]);
    assert!(output.contains("- Languages: java (2)"), "{output}");
    assert!(output.contains("PetClinicApplication.java` — application main class (pom.xml)"));
    assert!(
        section(&output, "## Key files").contains("owner/OwnerController.java"),
        "{output}"
    );
    assert!(output.contains("- run: `./mvnw spring-boot:run` — maven default"));
}

#[test]
fn monorepo_workspace_ignores_test_fixture_packages() {
    let repo = TempRepo::new("monorepo");
    repo.write(
        "package.json",
        r#"{"name":"root","private":true,"workspaces":["packages/*"]}"#,
    );
    repo.write(
        "packages/core/package.json",
        r#"{"name":"@acme/core","description":"Acme core runtime","bin":{"acme":"bin/acme.js"}}"#,
    );
    repo.write("packages/core/bin/acme.js", "require('../src/index.js')\n");
    repo.write(
        "packages/core/src/index.js",
        &"export function run() {}\n".repeat(20),
    );
    repo.write("packages/ui/package.json", r#"{"name":"@acme/ui"}"#);
    repo.write("packages/ui/src/index.js", "export const Button = 1\n");
    for index in 0..5 {
        repo.write(
            &format!("packages/core/test/fixtures/case{index}/package.json"),
            r#"{"name":"fixture"}"#,
        );
    }
    repo.write("examples/blog/package.json", r#"{"name":"blog-example"}"#);

    let output = repo.run(&["--no-git"]);
    let dir_name = repo
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(
        output.starts_with(&format!("# {dir_name} — context pack")),
        "placeholder root name should fall back to the directory: {output}"
    );
    let workspace = section(&output, "## Workspace");
    assert!(
        workspace.starts_with("## Workspace (3 packages)"),
        "{workspace}"
    );
    assert!(
        workspace.contains("- `packages/*` — 2 package(s): `core (@acme/core)`, `ui (@acme/ui)`"),
        "{workspace}"
    );
    assert!(workspace.contains("- `examples/*` — 1 package(s) [examples]"));
    assert!(!workspace.contains("fixtures"));
    assert!(section(&output, "## Entry points")
        .contains("`packages/core/bin/acme.js` — npm bin `acme`"));
}

#[test]
fn nested_gitignore_rules_are_respected() {
    let repo = TempRepo::new("ignore");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write("web/.gitignore", "generated/\n");
    repo.write("web/generated/huge.js", &"var x = 1;\n".repeat(5000));
    repo.write("web/src/app.js", "export default 1\n");

    let json = repo.json(&["--no-git"]);
    let dump = json.to_string();
    assert!(!dump.contains("generated/huge.js"), "{dump}");
    let web = json["layout"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == "web/")
        .unwrap();
    assert_eq!(web["files"], 2, "web/.gitignore and web/src/app.js only");
}

#[test]
fn include_forces_files_and_exclude_hides_them() {
    let repo = TempRepo::new("include");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write("src/tiny.rs", "pub fn x() {}\n");
    repo.write("legacy/old.rs", &"fn old() {}\n".repeat(200));

    let output = repo.run(&[
        "--no-git",
        "--include",
        "src/tiny.rs",
        "--exclude",
        "legacy",
    ]);
    assert!(
        output.contains("- `src/tiny.rs` — explicitly included"),
        "{output}"
    );
    assert!(!output.contains("legacy"), "{output}");
}

// ---------------------------------------------------------------------------
// Git

#[test]
fn active_work_shows_branch_commits_and_uncommitted_changes() {
    let repo = TempRepo::new("git");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write("src/lib.rs", "pub fn a() {}\n");
    repo.git(&["init", "-q", "-b", "main"]);
    repo.commit("initial");
    repo.git(&["checkout", "-q", "-b", "feature/search"]);
    repo.write("src/search.rs", "pub fn search() {}\n");
    repo.commit("add search");
    repo.write("src/lib.rs", "pub fn a() {}\npub fn b() {}\n");
    repo.write("notes.txt", "scratch\n");
    repo.write(".context-pack/PROJECT_CONTEXT.md", "own artifact\n");

    let output = repo.run(&[]);
    assert!(
        output.contains(
            "- Git: branch `feature/search`, 1 commit(s) ahead of `main`, 2 uncommitted change(s)"
        ),
        "{output}"
    );
    let active = section(&output, "## Active work");
    assert!(
        active.contains(
            "- Branch changes vs `main` (1 commit(s), 1 file(s)):\n  - A `src/search.rs` (+1 -0)"
        ),
        "{active}"
    );
    assert!(active.contains("  - M `src/lib.rs` (+1 -0)"), "{active}");
    assert!(active.contains("  - ?? `notes.txt`"), "{active}");
    assert!(
        !active.contains(".context-pack"),
        "own artifacts are not active work"
    );
    assert!(active.contains("add search"));
    let key_files = section(&output, "## Key files");
    assert!(
        key_files.contains("`src/lib.rs` — changed in active work"),
        "{key_files}"
    );
    assert!(
        key_files.contains("`src/search.rs` — changed in active work"),
        "{key_files}"
    );
}

#[test]
fn cwd_inside_a_repository_reports_paths_relative_to_it() {
    let repo = TempRepo::new("subdir");
    repo.write("service/package.json", r#"{"name":"service"}"#);
    repo.write("service/index.js", "console.log(1)\n");
    repo.write("other/file.txt", "x\n");
    repo.git(&["init", "-q", "-b", "main"]);
    repo.commit("initial");
    repo.write("service/index.js", "console.log(2)\n");
    repo.write("other/file.txt", "y\n");

    let output = run(&repo.path().join("service"), &[]);
    let active = section(&output, "## Active work");
    assert!(active.contains("  - M `index.js`"), "{active}");
    assert!(!active.contains("other/"), "{active}");
}

#[test]
fn directories_without_git_still_get_a_briefing() {
    let repo = TempRepo::new("nogit");
    repo.write("main.go", "package main\nfunc main() {}\n");
    repo.write("go.mod", "module example.com/tool\n");

    let output = repo.run(&[]);
    assert!(
        output.contains("- `main.go` — go main package (go.mod)"),
        "{output}"
    );
    assert!(output.contains("- not a git repository (or git is unavailable)"));
    assert!(output.contains("- test: `go test ./...` — go default"));
}

// ---------------------------------------------------------------------------
// Memory and context artifacts

#[test]
fn memory_notes_survive_refresh_and_appear_in_the_briefing() {
    let repo = TempRepo::new("memory");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");

    assert!(repo.run(&["memory", "init"]).starts_with("Created "));
    assert!(repo
        .run_failure(&["memory", "init"])
        .contains("already exists"));
    repo.run(&[
        "memory",
        "add",
        "Integration",
        "tests",
        "need",
        "Docker",
        "running.",
    ]);
    let memory_path = repo.path().join(".context-pack/memory.md");
    let before = fs::read_to_string(&memory_path).unwrap();
    repo.run(&["memory", "refresh"]);
    let after = fs::read_to_string(&memory_path).unwrap();

    assert!(
        after.contains("## Notes\n- Integration tests need Docker running.\n"),
        "{after}"
    );
    assert_eq!(
        metadata(&before, "created_at_unix"),
        metadata(&after, "created_at_unix")
    );
    let output = repo.run(&["--no-git"]);
    assert!(
        section(&output, "## Repo memory").contains("- Integration tests need Docker running."),
        "{output}"
    );
}

#[test]
fn stale_memory_is_flagged_when_commits_landed_since_review() {
    let repo = TempRepo::new("stale-memory");
    repo.write("README.md", "# Demo\n");
    repo.write(
        ".context-pack/memory.md",
        "# Memory\n\n## Memory Metadata\n- created_at_unix: 1000\n- created_at_utc: 1970-01-01T00:16:40Z\n- refreshed_at_unix: 1000\n- refreshed_at_utc: 1970-01-01T00:16:40Z\n\n## Notes\n- old fact\n",
    );
    repo.git(&["init", "-q", "-b", "main"]);
    repo.commit("initial");

    let output = repo.run(&[]);
    assert!(
        output.contains(
            "- Memory: `.context-pack/memory.md` (stale — last reviewed 1970-01-01T00:16:40Z"
        ),
        "{output}"
    );
}

#[test]
fn context_check_fails_once_head_moves() {
    let repo = TempRepo::new("context");
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.git(&["init", "-q", "-b", "main"]);
    repo.commit("initial");

    let refreshed = repo.run(&["context", "refresh"]);
    assert!(refreshed.contains("PROJECT_CONTEXT.md") && refreshed.contains("PROJECT_CONTEXT.json"));
    assert!(repo.path().join(".context-pack/memory.md").is_file());
    assert_eq!(
        repo.run(&["context", "check"]).trim(),
        "Context artifacts are present and match HEAD"
    );

    repo.write("src/lib.rs", "pub fn x() {}\n");
    repo.commit("more");
    assert!(repo
        .run_failure(&["context", "check"])
        .contains("run `context-pack context refresh`"));
}

// ---------------------------------------------------------------------------
// Output contract

#[test]
fn json_output_matches_schema_two() {
    let repo = TempRepo::new("json");
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\ndescription = \"A demo\"\n[dependencies]\nserde = \"1\"\n",
    );
    repo.write("src/main.rs", "fn main() {}\n");

    let json = repo.json(&["--no-git"]);
    assert_eq!(json["schema_version"], "2.0");
    assert_eq!(json["repo"]["name"], "demo");
    assert_eq!(json["repo"]["description"], "A demo");
    assert_eq!(json["repo"]["dependencies"][0]["runtime"][0], "serde");
    assert_eq!(json["entry_points"][0]["path"], "src/main.rs");
    assert_eq!(json["commands"][1]["command"], "cargo test");
    for key in [
        "instructions",
        "key_files",
        "layout",
        "docs",
        "config",
        "excerpts",
        "notes",
        "stats",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    assert!(json.get("git").is_none(), "git omitted with --no-git");
}

#[test]
fn markdown_respects_the_byte_budget() {
    let repo = TempRepo::new("budget");
    repo.write("AGENTS.md", &"- rule\n".repeat(200));
    repo.write(
        "package.json",
        r#"{"name":"big","scripts":{"build":"tsc","test":"vitest","lint":"eslint ."}}"#,
    );
    for index in 0..40 {
        repo.write(
            &format!("src/module{index:02}/index.ts"),
            &"export const value = 1;\n".repeat(50 + index),
        );
    }
    for budget in [1000usize, 1500, 3000, 6000] {
        let output = repo.run(&["--no-git", "--max-bytes", &budget.to_string()]);
        assert!(
            output.len() <= budget,
            "budget {budget}: {} bytes",
            output.len()
        );
        assert!(
            output.contains("## Commands"),
            "commands survive trimming at {budget}"
        );
    }
}

#[test]
fn secrets_are_redacted_in_excerpts_but_keywords_are_not() {
    let repo = TempRepo::new("secrets");
    repo.write(
        "CLAUDE.md",
        "Deploy key lives in vault.\napi_key: sk-live-123\nkeywords: cli, agents\n",
    );
    repo.write(".env", "TOKEN=abc\n");

    let output = repo.run(&["--no-git"]);
    let excerpts = section(&output, "## Excerpts");
    assert!(excerpts.contains("api_key: [REDACTED]"), "{excerpts}");
    assert!(excerpts.contains("keywords: cli, agents"), "{excerpts}");
    assert!(!output.contains("TOKEN=abc"));
}

#[test]
fn help_works_for_every_subcommand() {
    let repo = TempRepo::new("help");
    for args in [
        &["--help"][..],
        &["context", "--help"],
        &["memory", "-h"],
        &["help"],
    ] {
        assert!(repo.run(args).contains("Usage:"), "{args:?}");
    }
    assert!(repo
        .run_failure(&["--format", "viking"])
        .contains("invalid format"));
}

// ---------------------------------------------------------------------------
// Helpers

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(prefix: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cp-{prefix}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        Self {
            path: path.canonicalize().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn run(&self, args: &[&str]) -> String {
        run(&self.path, args)
    }

    fn json(&self, args: &[&str]) -> serde_json::Value {
        let mut args = args.to_vec();
        args.extend(["--format", "json"]);
        serde_json::from_str(&self.run(&args)).expect("valid JSON")
    }

    fn run_failure(&self, args: &[&str]) -> String {
        let output = command(&self.path, args).output().unwrap();
        assert!(!output.status.success(), "expected failure for {args:?}");
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn command(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_context-pack"));
    command.current_dir(cwd).args(args);
    command
}

fn run(cwd: &Path, args: &[&str]) -> String {
    let output = command(cwd, args).output().unwrap();
    assert!(
        output.status.success(),
        "context-pack {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The markdown section starting at `heading`, up to the next `## ` heading.
fn section<'a>(output: &'a str, heading: &str) -> &'a str {
    let start = output
        .find(heading)
        .unwrap_or_else(|| panic!("missing section {heading} in:\n{output}"));
    let rest = &output[start..];
    let end = rest[heading.len()..]
        .find("\n## ")
        .map(|offset| offset + heading.len() + 1)
        .unwrap_or(rest.len());
    &rest[..end]
}

fn assert_order(output: &str, needles: &[&str]) {
    let mut last = 0;
    for needle in needles {
        let position = output[last..]
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} missing or out of order in:\n{output}"));
        last += position + needle.len();
    }
}

fn metadata(content: &str, key: &str) -> String {
    content
        .lines()
        .find_map(|line| line.strip_prefix(&format!("- {key}: ")))
        .unwrap_or_default()
        .to_string()
}
