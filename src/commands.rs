//! Discover how to build, test, lint and run the project. Explicit developer
//! interfaces (Makefile, justfile, package scripts) win over ecosystem
//! defaults, and CI steps show what has to pass before a change lands.

use std::path::Path;

use crate::index::RepoIndex;
use crate::manifest::{Ecosystem, Manifest};
use crate::model::CommandHint;
use crate::paths;

const MAX_COMMANDS: usize = 12;
const MAX_CI_COMMANDS: usize = 6;
const MAX_DETAIL: usize = 90;

/// Order in which kinds are rendered; also the only kinds we report.
const KINDS: &[&str] = &[
    "setup",
    "build",
    "test",
    "lint",
    "format",
    "typecheck",
    "check",
    "run",
    "dev",
];

pub fn collect(index: &RepoIndex, manifests: &[Manifest]) -> Vec<CommandHint> {
    let mut commands = Vec::new();

    collect_task_runners(index, &mut commands);
    collect_package_scripts(index, manifests, &mut commands);
    collect_ecosystem_defaults(index, manifests, &mut commands);

    commands.sort_by_key(|hint| KINDS.iter().position(|kind| *kind == hint.kind));
    let mut selected: Vec<CommandHint> = Vec::new();
    for hint in commands {
        let per_kind = selected
            .iter()
            .filter(|known| known.kind == hint.kind)
            .count();
        if per_kind >= 2 || selected.iter().any(|known| known.command == hint.command) {
            continue;
        }
        selected.push(hint);
        if selected.len() >= MAX_COMMANDS {
            break;
        }
    }

    // CI steps that only repeat a command (or recipe) listed above add nothing.
    for hint in collect_ci(index) {
        let duplicate = selected.iter().any(|known| {
            known.command == hint.command
                || known
                    .source
                    .split_once(": ")
                    .is_some_and(|(_, detail)| detail == hint.command)
        });
        if !duplicate {
            selected.push(hint);
        }
    }
    selected
}

fn classify(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let base = lower
        .split([':', '-', '_', '.', '/'])
        .next()
        .unwrap_or(&lower);
    let kind = match lower.as_str() {
        "type-check" | "typecheck" | "check-types" | "types" | "tsc" | "check:types" => "typecheck",
        "fmt" | "format" | "prettier" | "fmt-check" | "format:check" | "check-format" => "format",
        "e2e" | "unit" | "integration" | "coverage" => "test",
        "precommit" | "pre-commit" | "ci" | "verify" | "validate" | "all" => "check",
        "serve" | "start" | "run" => "run",
        "watch" | "develop" => "dev",
        "install" | "setup" | "bootstrap" | "deps" | "init" => "setup",
        _ => match base {
            "build" | "compile" | "bundle" => "build",
            "test" | "tests" => "test",
            "lint" | "clippy" | "vet" | "eslint" => "lint",
            "fmt" | "format" => "format",
            "typecheck" => "typecheck",
            "check" => "check",
            "dev" => "dev",
            "start" | "serve" => "run",
            _ => return None,
        },
    };
    Some(kind)
}

fn push(commands: &mut Vec<CommandHint>, kind: &str, command: String, source: String) {
    commands.push(CommandHint {
        kind: kind.to_string(),
        command,
        source,
    });
}

fn collect_task_runners(index: &RepoIndex, commands: &mut Vec<CommandHint>) {
    for name in ["Makefile", "GNUmakefile", "makefile"] {
        if let Some(text) = index.read(name) {
            for (target, recipe) in make_targets(&text) {
                if let Some(kind) = classify(&target) {
                    push(
                        commands,
                        kind,
                        format!("make {target}"),
                        source_with_detail(name, &recipe),
                    );
                }
            }
            break;
        }
    }

    for name in ["justfile", "Justfile", ".justfile"] {
        if let Some(text) = index.read(name) {
            for (recipe, body) in just_recipes(&text) {
                if let Some(kind) = classify(&recipe) {
                    push(
                        commands,
                        kind,
                        format!("just {recipe}"),
                        source_with_detail(name, &body),
                    );
                }
            }
            break;
        }
    }

    for name in ["Taskfile.yml", "Taskfile.yaml"] {
        if let Some(text) = index.read(name) {
            for task in taskfile_tasks(&text) {
                if let Some(kind) = classify(&task) {
                    push(commands, kind, format!("task {task}"), name.to_string());
                }
            }
            break;
        }
    }
}

fn collect_package_scripts(
    index: &RepoIndex,
    manifests: &[Manifest],
    commands: &mut Vec<CommandHint>,
) {
    let runner = node_package_manager(index);
    let Some(depth) = manifests
        .iter()
        .filter(|manifest| !manifest.scripts.is_empty())
        .map(|manifest| paths::depth(&manifest.path))
        .min()
    else {
        return;
    };

    // Scripts of the shallowest manifests only; nested packages are usually
    // driven from the root in a workspace.
    for manifest in manifests
        .iter()
        .filter(|manifest| !manifest.scripts.is_empty() && paths::depth(&manifest.path) == depth)
        .take(2)
    {
        let prefix = if manifest.dir().as_os_str().is_empty() {
            String::new()
        } else {
            format!("cd {} && ", paths::display(manifest.dir()))
        };
        for (name, body) in &manifest.scripts {
            if is_lifecycle_hook(name, &manifest.scripts) {
                continue;
            }
            let Some(kind) = classify(name) else {
                continue;
            };
            let command = match manifest.ecosystem {
                Ecosystem::Deno => format!("{prefix}deno task {name}"),
                Ecosystem::Php => format!("{prefix}composer run {name}"),
                _ if name == "test" || name == "start" => format!("{prefix}{runner} {name}"),
                _ => format!("{prefix}{runner} run {name}"),
            };
            let source = source_with_detail(&paths::display(&manifest.path), body);
            push(commands, kind, command, source);
        }
    }
}

/// `pretest`/`postbuild` run implicitly around another script.
fn is_lifecycle_hook(name: &str, scripts: &[(String, String)]) -> bool {
    ["pre", "post"].iter().any(|prefix| {
        name.strip_prefix(prefix)
            .is_some_and(|rest| scripts.iter().any(|(other, _)| other == rest))
    })
}

fn node_package_manager(index: &RepoIndex) -> &'static str {
    if index.contains("pnpm-lock.yaml") || index.contains("pnpm-workspace.yaml") {
        "pnpm"
    } else if index.contains("bun.lockb") || index.contains("bun.lock") {
        "bun"
    } else if index.contains("yarn.lock") {
        "yarn"
    } else {
        "npm"
    }
}

fn collect_ecosystem_defaults(
    index: &RepoIndex,
    manifests: &[Manifest],
    commands: &mut Vec<CommandHint>,
) {
    let Some(min_depth) = manifests
        .iter()
        .map(|manifest| paths::depth(&manifest.path))
        .min()
    else {
        return;
    };
    let mut seen = Vec::new();
    for manifest in manifests
        .iter()
        .filter(|manifest| paths::depth(&manifest.path) == min_depth)
    {
        if seen.contains(&manifest.ecosystem) {
            continue;
        }
        seen.push(manifest.ecosystem);
        let source = format!("{} default", manifest.ecosystem.label());
        let prefix = if manifest.dir().as_os_str().is_empty() {
            String::new()
        } else {
            format!("cd {} && ", paths::display(manifest.dir()))
        };
        for (kind, command) in ecosystem_defaults(index, manifest) {
            if commands.iter().any(|known| known.kind == kind) {
                continue;
            }
            push(commands, kind, format!("{prefix}{command}"), source.clone());
        }
    }

    if index.contains(".pre-commit-config.yaml")
        && !commands.iter().any(|known| known.kind == "check")
    {
        push(
            commands,
            "check",
            "pre-commit run --all-files".to_string(),
            ".pre-commit-config.yaml".to_string(),
        );
    }
}

fn ecosystem_defaults(index: &RepoIndex, manifest: &Manifest) -> Vec<(&'static str, String)> {
    let dir = manifest.dir();
    let has = |name: &str| index.contains(dir.join(name));
    match manifest.ecosystem {
        Ecosystem::Cargo => {
            let workspace = if manifest.members.is_empty() {
                ""
            } else {
                " --workspace"
            };
            vec![
                ("build", "cargo build".to_string()),
                ("test", format!("cargo test{workspace}")),
                (
                    "lint",
                    format!("cargo clippy{workspace} --all-targets -- -D warnings"),
                ),
                ("format", "cargo fmt --all --check".to_string()),
            ]
        }
        Ecosystem::Go => {
            let mut defaults = vec![
                ("build", "go build ./...".to_string()),
                ("test", "go test ./...".to_string()),
            ];
            if has(".golangci.yml") || has(".golangci.yaml") {
                defaults.push(("lint", "golangci-lint run".to_string()));
            } else {
                defaults.push(("lint", "go vet ./...".to_string()));
            }
            defaults
        }
        Ecosystem::Python => python_defaults(index, manifest),
        Ecosystem::Maven => {
            let mvn = if has("mvnw") { "./mvnw" } else { "mvn" };
            let mut defaults = vec![
                ("build", format!("{mvn} -q package -DskipTests")),
                ("test", format!("{mvn} test")),
            ];
            if manifest
                .dependencies
                .iter()
                .any(|dep| dep.starts_with("spring-boot"))
            {
                defaults.push(("run", format!("{mvn} spring-boot:run")));
            }
            defaults
        }
        Ecosystem::Gradle => {
            let gradle = if has("gradlew") {
                "./gradlew"
            } else {
                "gradle"
            };
            vec![
                ("build", format!("{gradle} build")),
                ("test", format!("{gradle} test")),
            ]
        }
        Ecosystem::Dotnet => vec![
            ("build", "dotnet build".to_string()),
            ("test", "dotnet test".to_string()),
        ],
        Ecosystem::Swift => vec![
            ("build", "swift build".to_string()),
            ("test", "swift test".to_string()),
        ],
        Ecosystem::Dart => {
            let tool = if manifest.has_dependency("flutter")
                || index
                    .read(&manifest.path)
                    .is_some_and(|text| text.contains("flutter:"))
            {
                "flutter"
            } else {
                "dart"
            };
            vec![
                ("test", format!("{tool} test")),
                ("lint", format!("{tool} analyze")),
            ]
        }
        Ecosystem::Elixir => vec![
            ("test", "mix test".to_string()),
            ("format", "mix format --check-formatted".to_string()),
        ],
        Ecosystem::Ruby => {
            if manifest.has_dependency("rspec") || manifest.has_dependency("rspec-rails") {
                vec![("test", "bundle exec rspec".to_string())]
            } else if manifest.has_dependency("rake") || has("Rakefile") {
                vec![("test", "bundle exec rake test".to_string())]
            } else {
                Vec::new()
            }
        }
        Ecosystem::Php => {
            if manifest
                .dev_dependencies
                .iter()
                .any(|dep| dep.ends_with("/phpunit"))
            {
                vec![("test", "vendor/bin/phpunit".to_string())]
            } else {
                Vec::new()
            }
        }
        Ecosystem::CMake => vec![
            (
                "build",
                "cmake -S . -B build && cmake --build build".to_string(),
            ),
            ("test", "ctest --test-dir build".to_string()),
        ],
        Ecosystem::Npm | Ecosystem::Deno => Vec::new(),
    }
}

fn python_defaults(index: &RepoIndex, manifest: &Manifest) -> Vec<(&'static str, String)> {
    let dir = manifest.dir();
    let has = |name: &str| index.contains(dir.join(name));
    let runner = if has("uv.lock") {
        "uv run "
    } else if has("poetry.lock") || manifest.has_tool("poetry") {
        "poetry run "
    } else if has("pdm.lock") {
        "pdm run "
    } else if has("Pipfile.lock") {
        "pipenv run "
    } else {
        ""
    };

    let mut defaults = Vec::new();
    if has("tox.ini") {
        defaults.push(("test", "tox".to_string()));
    }
    let uses_pytest = manifest.has_tool("pytest")
        || manifest.has_dependency("pytest")
        || has("pytest.ini")
        || has("conftest.py")
        || index.files.iter().any(|entry| {
            let name = paths::file_name(&entry.path);
            name.starts_with("test_") && name.ends_with(".py")
        });
    if uses_pytest {
        defaults.push(("test", format!("{runner}pytest")));
    }
    if manifest.has_tool("ruff") || has("ruff.toml") || has(".ruff.toml") {
        defaults.push(("lint", format!("{runner}ruff check .")));
        defaults.push(("format", format!("{runner}ruff format --check .")));
    } else if manifest.has_tool("black") {
        defaults.push(("format", format!("{runner}black --check .")));
    }
    if manifest.has_tool("mypy") || has("mypy.ini") {
        defaults.push(("typecheck", format!("{runner}mypy .")));
    } else if manifest.has_tool("pyright") || has("pyrightconfig.json") {
        defaults.push(("typecheck", format!("{runner}pyright")));
    }
    if has("manage.py") {
        defaults.push(("run", format!("{runner}python manage.py runserver")));
    }
    defaults
}

fn collect_ci(index: &RepoIndex) -> Vec<CommandHint> {
    let mut hints: Vec<CommandHint> = Vec::new();
    let workflows = index
        .files
        .iter()
        .filter(|entry| {
            let path = paths::display(&entry.path);
            path.starts_with(".github/workflows/")
                && (path.ends_with(".yml") || path.ends_with(".yaml"))
        })
        .map(|entry| entry.path.clone())
        .chain(
            [
                ".gitlab-ci.yml",
                ".circleci/config.yml",
                "azure-pipelines.yml",
            ]
            .iter()
            .map(Path::new)
            .filter(|path| index.contains(path))
            .map(Path::to_path_buf),
        )
        .collect::<Vec<_>>();

    // Only workflows that gate changes matter; release and housekeeping jobs are noise.
    let mut workflows = workflows
        .into_iter()
        .filter_map(|path| workflow_kind(&path).map(|gate| (gate, path)))
        .collect::<Vec<_>>();
    let has_gate = workflows.iter().any(|(gate, _)| *gate);
    workflows.retain(|(gate, _)| *gate || !has_gate);
    // `ci.yml` / `test.yml` are the main gate; `test-redistribute.yml` is a side job.
    workflows.sort_by_key(|(_, path)| {
        let stem = paths::file_name(path)
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let exact = matches!(
            stem.as_str(),
            "ci" | "test" | "tests" | "check" | "checks" | "build" | "lint" | "main" | "pr"
        );
        (!exact, path.clone())
    });

    for (_, workflow) in workflows {
        let Some(text) = index.read(&workflow) else {
            continue;
        };
        for command in ci_run_commands(&text).into_iter().take(4) {
            if hints.len() >= MAX_CI_COMMANDS {
                return hints;
            }
            if hints.iter().any(|known| known.command == command) {
                continue;
            }
            hints.push(CommandHint {
                kind: "ci".to_string(),
                command,
                source: paths::display(&workflow),
            });
        }
    }
    hints
}

/// `Some(true)` for PR gates, `Some(false)` for neutral workflows, `None` for noise.
fn workflow_kind(path: &Path) -> Option<bool> {
    let name = paths::file_name(path).to_ascii_lowercase();
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
    let tokens = stem
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .collect::<Vec<_>>();
    let noise = [
        "release",
        "publish",
        "deploy",
        "stale",
        "label",
        "labeler",
        "docs",
        "codeql",
        "bump",
        "dependabot",
        "renovate",
        "sync",
        "triage",
        "greet",
        "greetings",
        "welcome",
        "lock",
        "backport",
        "cla",
        "preview",
        "nightly",
        "benchmark",
        "bench",
        "issue",
        "issues",
        "changeset",
        "changesets",
        "cleanup",
        "cron",
        "notify",
        "pages",
        "snapshot",
    ];
    if tokens.iter().any(|token| noise.contains(token)) {
        return None;
    }
    let gate = [
        "ci",
        "test",
        "tests",
        "check",
        "checks",
        "build",
        "lint",
        "main",
        "pr",
        "pull",
        "validate",
        "verify",
        "rust",
        "go",
        "python",
        "node",
        "java",
        "unit",
        "integration",
        "e2e",
    ];
    Some(tokens.iter().any(|token| gate.contains(token)) || path.starts_with(".gitlab-ci.yml"))
}

/// Pull single commands out of `run:` steps (GitHub) and `script:` lists (GitLab).
fn ci_run_commands(text: &str) -> Vec<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut commands = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start().trim_start_matches("- ").trim_start();
        // Column of the key itself, so sibling keys after `- run: |` end the block.
        let indent = line.len() - trimmed.len();
        index += 1;

        if let Some(rest) = trimmed.strip_prefix("run:") {
            let rest = rest.trim();
            if rest.starts_with('>') {
                // Folded scalar: all lines form a single command.
                let mut parts = Vec::new();
                while index < lines.len() {
                    let next = lines[index];
                    let next_indent = next.len() - next.trim_start().len();
                    if !next.trim().is_empty() && next_indent <= indent {
                        break;
                    }
                    index += 1;
                    if !next.trim().is_empty() {
                        parts.push(next.trim());
                    }
                }
                let command = parts.join(" ");
                if keep_ci_command(&command) {
                    commands.push(command);
                }
            } else if rest.is_empty() || rest.starts_with('|') {
                let mut taken = 0;
                let mut continued = false;
                while index < lines.len() {
                    let next = lines[index];
                    let next_indent = next.len() - next.trim_start().len();
                    if !next.trim().is_empty() && next_indent <= indent {
                        break;
                    }
                    index += 1;
                    // Arguments of a `\`-continued command are not commands.
                    let was_continued = continued;
                    continued = next.trim_end().ends_with('\\');
                    if !was_continued && taken < 3 && keep_ci_command(next.trim()) {
                        commands.push(next.trim().to_string());
                        taken += 1;
                    }
                }
            } else if keep_ci_command(rest) {
                commands.push(rest.trim_matches('"').to_string());
            }
        } else if trimmed.starts_with("script:") {
            while index < lines.len() {
                let next = lines[index].trim_start();
                let Some(item) = next.strip_prefix("- ") else {
                    break;
                };
                index += 1;
                if keep_ci_command(item.trim()) {
                    commands.push(item.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    commands
}

fn keep_ci_command(command: &str) -> bool {
    if command.is_empty()
        || command.starts_with(['#', '-', '"', '\'', '|', '&', ')'])
        || command.contains("${{")
    {
        return false;
    }
    let first = command.split_whitespace().next().unwrap_or_default();
    // `VAR=value`, YAML keys like `shell: bash`, and continuation fragments.
    if first.contains('=') || first.ends_with(':') {
        return false;
    }
    let second = command.split_whitespace().nth(1).unwrap_or_default();
    if matches!(first, "npm" | "pnpm" | "yarn") && matches!(second, "config" | "cache") {
        return false;
    }
    if matches!(
        first,
        "echo"
            | "cd"
            | "export"
            | "set"
            | "if"
            | "then"
            | "else"
            | "fi"
            | "done"
            | "for"
            | "sudo"
            | "apt"
            | "apt-get"
            | "brew"
            | "choco"
            | "curl"
            | "wget"
            | "git"
            | "mkdir"
            | "rm"
            | "cp"
            | "mv"
            | "ls"
            | "cat"
            | "chmod"
            | "source"
            | "."
            | "rustup"
            | "}"
            | "{"
            | "exit"
            | "python3"
            | "python"
            | "pip"
            | "pip3"
            | "tar"
            | "unzip"
            | "zip"
            | "gh"
            | "docker"
            | "sleep"
            | "test"
            | "["
    ) {
        // `python -m pytest` style invocations are real checks.
        return (first == "python" || first == "python3") && command.contains(" -m ");
    }
    let balanced = command.matches('"').count().is_multiple_of(2)
        && command.matches('\'').count().is_multiple_of(2);
    let installs_one_off = [
        "pip install",
        "pip uninstall",
        "install -g",
        "playwright install",
        "go install",
        "cargo install",
        "pipx install",
        "npm i -g",
    ]
    .iter()
    .any(|needle| command.contains(needle));
    balanced && !installs_one_off && command.len() <= 160 && !command.ends_with('\\')
}

fn source_with_detail(source: &str, detail: &str) -> String {
    let detail = detail.trim().trim_start_matches('@').trim();
    if detail.is_empty() {
        return source.to_string();
    }
    format!("{source}: {}", truncate(detail, MAX_DETAIL))
}

fn truncate(text: &str, max: usize) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= max {
        return single_line;
    }
    let cut = single_line.chars().take(max).collect::<String>();
    format!("{}…", cut.trim_end())
}

/// Makefile targets with the first recipe line of each.
fn make_targets(text: &str) -> Vec<(String, String)> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut targets = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('\t')
            || line.starts_with(' ')
            || line.starts_with('#')
            || line.starts_with('.')
        {
            continue;
        }
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        if line[head.len()..].starts_with(":=")
            || head.contains('=')
            || head.contains('$')
            || head.contains('%')
        {
            continue;
        }
        let recipe = lines[index + 1..]
            .iter()
            .take_while(|next| next.starts_with('\t'))
            .map(|next| next.trim().trim_start_matches(['@', '-']))
            .find(|next| {
                !next.is_empty()
                    && !next.starts_with('#')
                    && !next.starts_with("$(info")
                    && !next.starts_with("$(warning")
                    && !next.starts_with("echo ")
                    && !next.starts_with("printf ")
            })
            .unwrap_or_default()
            .to_string();
        for target in head.split_whitespace() {
            targets.push((target.to_string(), recipe.clone()));
        }
    }
    targets
}

fn just_recipes(text: &str) -> Vec<(String, String)> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut recipes = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with(' ')
            || line.starts_with('\t')
            || line.starts_with('#')
            || line.starts_with('[')
        {
            continue;
        }
        let Some((head, _)) = line.split_once(':') else {
            continue;
        };
        if line.contains(":=")
            || head.starts_with("set ")
            || head.starts_with("export ")
            || head.starts_with("alias ")
        {
            continue;
        }
        let Some(name) = head.trim_start_matches('@').split_whitespace().next() else {
            continue;
        };
        let body = lines[index + 1..]
            .iter()
            .take_while(|next| next.starts_with(' ') || next.starts_with('\t'))
            .map(|next| next.trim())
            .find(|next| !next.is_empty())
            .unwrap_or_default()
            .to_string();
        recipes.push((name.to_string(), body));
    }
    recipes
}

fn taskfile_tasks(text: &str) -> Vec<String> {
    let mut tasks = Vec::new();
    let mut in_tasks = false;
    let mut task_indent = None;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_tasks = line.starts_with("tasks:");
            continue;
        }
        if !in_tasks {
            continue;
        }
        let indent_level = *task_indent.get_or_insert(indent);
        if indent == indent_level {
            if let Some(name) = line.trim().strip_suffix(':') {
                tasks.push(name.trim_matches('"').to_string());
            }
        }
    }
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_targets_capture_first_recipe_line() {
        let makefile = ".PHONY: test\nVAR := 1\ntest:\n\t@cargo test\nlint: fmt\n\tcargo clippy\n";
        let targets = make_targets(makefile);
        assert_eq!(targets[0], ("test".to_string(), "cargo test".to_string()));
        assert_eq!(targets[1].0, "lint");
    }

    #[test]
    fn ci_commands_skip_setup_noise() {
        let workflow = "jobs:\n  a:\n    steps:\n      - uses: actions/checkout@v4\n      - run: cargo test\n      - name: Lint\n        run: |\n          echo start\n          cargo clippy -- -D warnings\n      - run: sudo apt-get install x\n";
        assert_eq!(
            ci_run_commands(workflow),
            vec![
                "cargo test".to_string(),
                "cargo clippy -- -D warnings".to_string()
            ]
        );
    }

    #[test]
    fn ci_block_ends_at_sibling_key() {
        let workflow =
            "steps:\n  - run: |\n      pnpm build\n    shell: bash\n  - run: pnpm test\n";
        assert_eq!(
            ci_run_commands(workflow),
            vec!["pnpm build".to_string(), "pnpm test".to_string()]
        );
    }

    #[test]
    fn workflow_names_are_classified_by_token() {
        assert_eq!(
            workflow_kind(Path::new(".github/workflows/ci.yml")),
            Some(true)
        );
        assert_eq!(
            workflow_kind(Path::new(".github/workflows/pre-commit.yml")),
            Some(false)
        );
        assert_eq!(
            workflow_kind(Path::new(".github/workflows/bump-pre-commit-hooks.yml")),
            None
        );
        assert_eq!(
            workflow_kind(Path::new(".github/workflows/release.yml")),
            None
        );
    }

    #[test]
    fn script_names_map_to_kinds() {
        assert_eq!(classify("test:unit"), Some("test"));
        assert_eq!(classify("lint:fix"), Some("lint"));
        assert_eq!(classify("typecheck"), Some("typecheck"));
        assert_eq!(classify("release"), None);
    }

    #[test]
    fn taskfile_top_level_tasks_are_listed() {
        let taskfile = "version: '3'\ntasks:\n  build:\n    cmds:\n      - go build\n  test:\n    cmds:\n      - go test\n";
        assert_eq!(
            taskfile_tasks(taskfile),
            vec!["build".to_string(), "test".to_string()]
        );
    }
}
