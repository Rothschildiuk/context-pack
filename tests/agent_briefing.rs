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
fn repo_memory_files_are_selected_as_high_signal_guidance() {
    let temp = TempDir::new("briefing-repo-memory");
    write_file(
        temp.path(),
        "REPO_MEMORY.md",
        "# Repo Memory\n\nThe worker queue usually fails around the ingestion step.\n",
    );
    write_file(
        temp.path(),
        ".context-pack/memory.md",
        "# Learned Notes\n\nThe admin sync path bypasses normal retries.\n",
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

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("Guidance files available: repo memory, README."));
    assert!(output.contains("`REPO_MEMORY.md`: learned repo memory"));
    assert!(output.contains("`.context-pack/memory.md`: learned repo memory"));
    assert_contains_heading(&output, "REPO_MEMORY.md");
    assert_contains_heading(&output, ".context-pack/memory.md");
}

#[test]
fn clio_style_agent_guidance_files_are_prioritized() {
    let temp = TempDir::new("briefing-clio-guidance");
    write_file(temp.path(), "AGENTS.md", "# Agent Rules\n\nStart here.\n");
    write_file(
        temp.path(),
        ".clio/instructions.md",
        "# CLIO Instructions\n\nUse the task model and memory flow.\n",
    );
    write_file(
        temp.path(),
        "llms.txt",
        "Key files for AI agents:\n- AGENTS.md\n- .clio/instructions.md\n",
    );
    write_file(
        temp.path(),
        "docs/ARCHITECTURE.md",
        "# Architecture\n\nRuntime flow.\n",
    );
    write_file(
        temp.path(),
        "docs/MEMORY.md",
        "# Memory\n\nMemory model and recall notes.\n",
    );
    write_file(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");
    write_file(temp.path(), "src/index.js", "console.log('demo');\n");

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("Guidance files available: AGENTS.md, .clio instructions, llms.txt."));
    assert!(output.contains("`AGENTS.md`: agent instructions"));
    assert!(output.contains("`.clio/instructions.md`: tool-specific agent instructions"));
    assert!(output.contains("`llms.txt`: AI-facing repo summary"));
    assert_before(&output, "`.clio/instructions.md`", "`package.json`");
    assert_before(&output, "`llms.txt`", "`package.json`");
}

#[test]
fn init_memory_creates_template_file() {
    let temp = TempDir::new("briefing-init-memory");
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

    let output = run_pack(temp.path(), &["--init-memory"]);
    let memory_path = temp.path().join(".context-pack/memory.md");

    assert!(output.contains("Created"));
    assert!(memory_path.is_file());

    let content = fs::read_to_string(memory_path).expect("memory file should be readable");
    assert!(content.contains("# Learned Repo Memory"));
    assert!(content.contains("- purpose: Likely a Rust CLI or developer tooling project."));
    assert!(content.contains("## Read First"));
    assert!(content.contains("## Entry Points"));
    assert!(content.contains("## Hotspots"));
    assert!(content.contains("## Known Pitfalls"));
    assert!(content.contains("`README.md`: project overview"));
    assert!(content.contains("`src/main.rs`: entrypoint-like source file"));
}

#[test]
fn init_memory_hotspots_prioritize_code_over_manifests() {
    let temp = TempDir::new("briefing-init-memory-hotspots");
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
        "fn main() {\n    println!(\"demo\");\n}\n",
    );
    write_file(
        temp.path(),
        "src/engine.rs",
        &repeat_lines("pub fn important_step() {}\n", 30),
    );

    let _ = run_pack(temp.path(), &["--init-memory"]);
    let content = fs::read_to_string(temp.path().join(".context-pack/memory.md"))
        .expect("memory file should be readable");
    let hotspots = section_body(&content, "## Hotspots", "## Known Pitfalls");

    assert_before(&hotspots, "`src/main.rs`", "`Cargo.toml`");
    assert!(hotspots.contains("`src/engine.rs`: large production source file"));
}

#[test]
fn init_memory_does_not_overwrite_existing_file() {
    let temp = TempDir::new("briefing-init-memory-existing");
    write_file(
        temp.path(),
        ".context-pack/memory.md",
        "# Existing Memory\n\nKeep this.\n",
    );

    let stderr = run_pack_failure(temp.path(), &["--init-memory"]);
    let content = fs::read_to_string(temp.path().join(".context-pack/memory.md"))
        .expect("existing memory should remain readable");

    assert!(stderr.contains("memory file already exists"));
    assert!(stderr.contains("--refresh-memory"));
    assert!(content.contains("# Existing Memory"));
}

#[test]
fn refresh_memory_overwrites_existing_file_with_new_draft() {
    let temp = TempDir::new("briefing-refresh-memory");
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
        ".context-pack/memory.md",
        "# Existing Memory\n\nOld notes.\n",
    );

    let output = run_pack(temp.path(), &["--refresh-memory"]);
    let content = fs::read_to_string(temp.path().join(".context-pack/memory.md"))
        .expect("refreshed memory should be readable");

    assert!(output.contains("Updated"));
    assert!(content.contains("# Learned Repo Memory"));
    assert!(content.contains("## Read First"));
    assert!(!content.contains("Old notes."));
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

    assert!(output.contains("- M `src/main.rs` (modified, +1 -1)"));
    assert!(output.contains("`src/main.rs`: changed source file, active work, likely entry point"));
}

#[test]
fn max_files_flag_allows_more_than_ten_selected_files() {
    let temp = TempDir::new("briefing-max-files");
    write_file(temp.path(), "AGENTS.md", "# Agent Rules\n\nStart here.\n");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "REPO_MEMORY.md",
        "# Repo Memory\n\nOperational notes.\n",
    );
    write_file(temp.path(), "llms.txt", "Key files for AI agents.\n");
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "Makefile", ".PHONY: run\nrun:\n\tcargo run\n");
    write_file(
        temp.path(),
        "CONTRIBUTING.md",
        "# Contributing\n\nHow to contribute.\n",
    );
    write_file(temp.path(), "OPERATIONS.md", "# Operations\n\nRunbook.\n");
    write_file(
        temp.path(),
        "RUNBOOK.md",
        "# Runbook\n\nOperational checklist.\n",
    );
    write_file(
        temp.path(),
        "docs/ARCHITECTURE.md",
        "# Architecture\n\nSystem overview.\n",
    );
    write_file(
        temp.path(),
        "docs/MEMORY.md",
        "# Memory\n\nContext model.\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(
        temp.path(),
        &[
            "--no-git",
            "--no-tree",
            "--max-files",
            "12",
            "--max-bytes",
            "8000",
        ],
    );

    assert!(output.contains("- selected files: 12"));
    assert_contains_heading(&output, "src/main.rs");
    assert_contains_heading(&output, "docs/MEMORY.md");
}

#[test]
fn git_changes_include_status_codes_and_diff_hints_in_json() {
    let temp = TempDir::new("briefing-git-json-hints");
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
    write_file(temp.path(), "src/helper.rs", "pub fn helper() {}\n");

    let output = run_pack(temp.path(), &["--format", "json", "--no-tree"]);

    assert!(output.contains("\"status\": \"M\""));
    assert!(output.contains("\"kind\": \"modified\""));
    assert!(output.contains("\"hint\": \"+1 -1\""));
    assert!(output.contains("\"status\": \"??\""));
    assert!(output.contains("\"kind\": \"untracked\""));
    assert!(output.contains("\"hint\": \"new file\""));
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
fn nested_readme_does_not_displace_repo_guides() {
    let temp = TempDir::new("briefing-repo-guides");
    write_file(
        temp.path(),
        "AGENTS.md",
        "# Agent Rules\n\nRead this first.\n",
    );
    write_file(
        temp.path(),
        "ARCHITECTURE.md",
        "# Architecture\n\nRuntime flow.\n",
    );
    write_file(
        temp.path(),
        "DATA_SOURCES.md",
        "# Data Sources\n\nSnapshot policy.\n",
    );
    write_file(
        temp.path(),
        "SERIES_GUIDE.md",
        "# Series Guide\n\nSeries families.\n",
    );
    write_file(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");
    write_file(temp.path(), "requirements.txt", "streamlit==1.0.0\n");
    write_file(temp.path(), "app.py", "print('demo')\n");
    write_file(
        temp.path(),
        "data/snapshots/README.md",
        "# Snapshots\n\nNested module notes.\n",
    );

    let output = run_pack(temp.path(), &["--no-git"]);

    assert!(output.contains("- Guidance files available: AGENTS.md."));
    assert!(output.contains("### ARCHITECTURE.md"));
    assert!(output.contains("### DATA_SOURCES.md"));
    assert_before(&output, "`ARCHITECTURE.md`", "`package.json`");
    assert!(!output.contains("`data/snapshots/README.md`"));
}

#[test]
fn nested_supporting_docs_are_selected_and_ranked() {
    let temp = TempDir::new("briefing-nested-guides");
    write_file(
        temp.path(),
        "docs/ARCHITECTURE.md",
        "# Architecture\n\nRuntime flow.\n",
    );
    write_file(
        temp.path(),
        "docs/DATA_SOURCES.md",
        "# Data Sources\n\nSnapshot policy.\n",
    );
    write_file(
        temp.path(),
        "docs/SERIES_GUIDE.md",
        "# Series Guide\n\nSeries families.\n",
    );
    write_file(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");
    write_file(
        temp.path(),
        "pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    write_file(temp.path(), "app.py", "print('demo')\n");

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("### docs/ARCHITECTURE.md"));
    assert!(output.contains("### docs/DATA_SOURCES.md"));
    assert_before(&output, "`docs/ARCHITECTURE.md`", "`package.json`");
}

#[test]
fn explicit_include_forces_docs_and_code_in_changed_only_mode() {
    let temp = TempDir::new("briefing-explicit-include");
    write_file(
        temp.path(),
        "docs/ARCHITECTURE.md",
        "# Architecture\n\nRuntime flow.\n",
    );
    write_file(
        temp.path(),
        "docs/DATA_SOURCES.md",
        "# Data Sources\n\nSnapshot policy.\n",
    );
    write_file(
        temp.path(),
        "docs/SERIES_GUIDE.md",
        "# Series Guide\n\nSeries families.\n",
    );
    write_file(temp.path(), "package.json", "{\n  \"name\": \"demo\"\n}\n");
    write_file(
        temp.path(),
        "pyproject.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    write_file(temp.path(), "app.py", "print('v1')\n");
    write_file(
        temp.path(),
        "services/price_service.py",
        "def fetch_price():\n    return 42\n",
    );
    write_file(
        temp.path(),
        "core/series_registry.py",
        "SERIES_REGISTRY = {\"demo\": 1}\n",
    );

    git(temp.path(), &["init"]);
    git(temp.path(), &["config", "user.email", "test@example.com"]);
    git(temp.path(), &["config", "user.name", "Test User"]);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "init"]);

    write_file(temp.path(), "app.py", "print('v2')\n");

    let output = run_pack(
        temp.path(),
        &[
            "--changed-only",
            "--include",
            "docs/*.md",
            "--include",
            "services/*.py",
            "--include",
            "core/*.py",
            "--no-tree",
        ],
    );

    assert!(output.contains("changed-only fast path used"));
    assert!(output.contains("### docs/ARCHITECTURE.md"));
    assert!(output.contains("### docs/DATA_SOURCES.md"));
    assert!(output.contains("### docs/SERIES_GUIDE.md"));
    assert!(output.contains("### services/price_service.py"));
    assert!(output.contains("### core/series_registry.py"));
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
    write_file(
        temp.path(),
        ".vscode/settings.json",
        "{\n  \"editor.tabSize\": 4\n}\n",
    );

    let output = run_pack(temp.path(), &[]);

    assert!(output.contains("No high-signal changes detected."));
    assert!(!output.contains(".idea/workspace.xml"));
    assert!(!output.contains(".vscode/settings.json"));
}

#[test]
fn shared_ide_configs_are_selected_without_local_workspace_noise() {
    let temp = TempDir::new("briefing-ide-configs");
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
    write_file(
        temp.path(),
        ".editorconfig",
        "root = true\n\n[*]\nindent_style = space\nindent_size = 4\n",
    );
    write_file(
        temp.path(),
        ".vscode/tasks.json",
        "{\n  \"version\": \"2.0.0\",\n  \"tasks\": [{\"label\": \"test\"}]\n}\n",
    );
    write_file(
        temp.path(),
        ".vscode/launch.json",
        "{\n  \"version\": \"0.2.0\",\n  \"configurations\": [{\"name\": \"Debug\"}]\n}\n",
    );
    write_file(
        temp.path(),
        ".idea/runConfigurations/Demo.xml",
        "<component name=\"ProjectRunConfigurationManager\">\n  <configuration name=\"Demo\" />\n</component>\n",
    );
    write_file(temp.path(), ".idea/workspace.xml", "<xml />\n");

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert_contains_heading(&output, ".editorconfig");
    assert_contains_heading(&output, ".vscode/tasks.json");
    assert_contains_heading(&output, ".vscode/launch.json");
    assert_contains_heading(&output, ".idea/runConfigurations/Demo.xml");
    assert!(output.contains("shared editor config"));
    assert!(output.contains("shared VS Code task config"));
    assert!(output.contains("shared VS Code launch config"));
    assert!(output.contains("shared IntelliJ run config"));
    assert!(!output.contains(".idea/workspace.xml"));
}

#[test]
fn git_branch_context_reports_current_default_and_upstream() {
    let remote = TempDir::new("briefing-git-remote");
    bare_git(remote.path(), &["init", "--bare"]);

    let temp = TempDir::new("briefing-git-branches");
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
    git(temp.path(), &["branch", "-M", "main"]);
    git(
        temp.path(),
        &[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("utf-8 path"),
        ],
    );
    git(temp.path(), &["push", "-u", "origin", "main"]);

    bare_git(remote.path(), &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(temp.path(), &["fetch", "origin"]);
    git(temp.path(), &["remote", "set-head", "origin", "-a"]);

    git(temp.path(), &["checkout", "-b", "feature/git-context"]);
    git(
        temp.path(),
        &["push", "-u", "origin", "feature/git-context"],
    );

    let output = run_pack(temp.path(), &["--no-tree"]);

    assert!(output.contains("- current branch: `feature/git-context`"));
    assert!(output.contains("- upstream branch: `origin/feature/git-context`"));
    assert!(output.contains("- default branch: `main`"));
    assert!(output.contains("- primary development branch likely `main`"));
    assert!(output.contains("`feature/git-context`"));
    assert!(output.contains("`main`"));
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
    assert!(output.contains("\"approx tokens: "));
    assert!(output.contains("\"path\": \"README.md\""));
    assert!(!output.contains("\"not_implemented\""));
}

#[test]
fn markdown_notes_include_approx_token_estimate() {
    let temp = TempDir::new("briefing-token-estimate");
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

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("## Notes"));
    assert!(output.contains("- approx tokens: "));
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
fn no_language_aware_flag_disables_language_boosts() {
    let temp = TempDir::new("briefing-no-language-aware");
    write_file(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(temp.path(), "src/main.rs", "fn main() {}\n");

    let output = run_pack(temp.path(), &["--no-git", "--no-language-aware"]);

    assert!(!output.contains("language-aware boost"));
    assert!(output.contains("- language-aware scoring disabled"));
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

    assert!(output.contains("- M `src/main.rs` (modified, +1 -1)"));
    assert!(!output.contains("`src/lib.rs` (80 LOC)"));
}

#[test]
fn changed_only_uses_fast_path_instead_of_full_repo_scan() {
    let temp = TempDir::new("briefing-changed-only-fast-path");
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
    for index in 0..120 {
        write_file(
            temp.path(),
            &format!("src/file_{index}.rs"),
            "pub fn stable() {}\n",
        );
    }
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

    let output = run_pack(temp.path(), &["--changed-only", "--no-tree"]);

    assert!(output.contains("changed-only fast path used"));
    assert!(selection_scanned_count(&output) <= 6);
}

#[test]
fn explicitly_included_source_excerpt_surfaces_structure() {
    let temp = TempDir::new("briefing-structured-excerpt");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );

    let mut source = repeat_lines("SERIES_NAME = 'demo'\n", 40);
    source.push_str(
        "\nclass PriceService:\n    def refresh_prices(self):\n        return fetch_prices()\n",
    );
    source.push_str("\ndef fetch_prices():\n    return [42]\n");
    source.push_str("\nif __name__ == \"__main__\":\n    print(fetch_prices())\n");
    write_file(temp.path(), "src/price_service.py", &source);

    let output = run_pack(
        temp.path(),
        &[
            "--no-git",
            "--no-tree",
            "--max-bytes",
            "900",
            "--include",
            "src/*.py",
        ],
    );

    assert!(output.contains("### src/price_service.py"));
    assert!(output.contains("class PriceService:"));
    assert!(output.contains("def fetch_prices():"));
    assert!(output.contains("if __name__ == \"__main__\":"));
}

#[test]
fn explicitly_included_env_file_is_omitted_as_sensitive() {
    let temp = TempDir::new("briefing-sensitive-env");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        ".env",
        "OPENAI_API_KEY=sk-live-secret\nDATABASE_URL=postgres://user:pass@localhost/db\n",
    );

    let output = run_pack(temp.path(), &["--no-git", "--no-tree", "--include", ".env"]);

    assert!(output.contains("### .env"));
    assert!(output.contains("- redacted: true"));
    assert!(output.contains("sensitive file type"));
    assert!(output.contains("[content omitted: sensitive file type]"));
    assert!(!output.contains("sk-live-secret"));
    assert!(!output.contains("postgres://user:pass@localhost/db"));
}

#[test]
fn env_example_values_are_redacted_in_excerpt() {
    let temp = TempDir::new("briefing-sensitive-env-example");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        ".env.example",
        "OPENAI_API_KEY=sk-example\nFEATURE_FLAG=true\nDB_PASSWORD=secret-pass\n",
    );

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("### .env.example"));
    assert!(output.contains("- redacted: true"));
    assert!(output.contains("OPENAI_API_KEY=[REDACTED]"));
    assert!(output.contains("DB_PASSWORD=[REDACTED]"));
    assert!(output.contains("FEATURE_FLAG=true"));
    assert!(!output.contains("sk-example"));
    assert!(!output.contains("secret-pass"));
}

#[test]
fn docker_compose_secrets_are_redacted_in_excerpt() {
    let temp = TempDir::new("briefing-sensitive-compose");
    write_file(
        temp.path(),
        "README.md",
        "# Demo Repo\n\nProject overview.\n",
    );
    write_file(
        temp.path(),
        "docker-compose.yml",
        concat!(
            "services:\n",
            "  app:\n",
            "    environment:\n",
            "      API_TOKEN: super-secret-token\n",
            "      LOG_LEVEL: debug\n",
            "      CLIENT_SECRET: \"top-secret\"\n"
        ),
    );

    let output = run_pack(temp.path(), &["--no-git", "--no-tree"]);

    assert!(output.contains("### docker-compose.yml"));
    assert!(output.contains("- redacted: true"));
    assert!(output.contains("API_TOKEN: [REDACTED]"));
    assert!(output.contains("CLIENT_SECRET: \"[REDACTED]\""));
    assert!(output.contains("LOG_LEVEL: debug"));
    assert!(!output.contains("super-secret-token"));
    assert!(!output.contains("top-secret"));
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

fn run_pack_failure(repo: &Path, args: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_context-pack"));
    command.arg("--cwd").arg(repo);
    command.args(args);

    let output = command.output().expect("failed to run context-pack");
    assert!(
        !output.status.success(),
        "context-pack unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
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

fn bare_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(repo)
        .args(args)
        .output()
        .expect("failed to run bare git");

    assert!(
        output.status.success(),
        "bare git {:?} failed: {}",
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

fn selection_scanned_count(output: &str) -> usize {
    output
        .lines()
        .find_map(|line| {
            line.strip_prefix("- files scanned for selection: ")
                .and_then(|value| value.parse::<usize>().ok())
        })
        .expect("missing selection scan count")
}

fn section_body<'a>(content: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = content
        .find(start)
        .unwrap_or_else(|| panic!("missing section {start}"));
    let tail = &content[start_index..];
    let end_index = tail.find(end).unwrap_or(tail.len());
    &tail[..end_index]
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
