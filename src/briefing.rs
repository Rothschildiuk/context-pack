use std::cmp::Reverse;

use crate::model::{
    AgentBriefing, AppConfig, BriefingItem, GitResult, ImportantFile, LargeCodeFile, RepoInfo,
    SignalCategory, WalkResult,
};

pub fn build(
    config: &AppConfig,
    repo: &RepoInfo,
    files: &[ImportantFile],
    large_code_files: &[LargeCodeFile],
    docker_summary: &[String],
    dependency_summary: &[String],
    git: &GitResult,
    walk: &WalkResult,
    budget: usize,
) -> AgentBriefing {
    let mut briefing = AgentBriefing {
        repo_summary: build_repo_summary(repo, files),
        active_work: build_active_work(git),
        read_these_first: build_read_these_first(files),
        likely_entry_points: build_likely_entry_points(files),
        docker_summary: docker_summary.to_vec(),
        dependency_summary: dependency_summary.to_vec(),
        large_code_files: build_large_code_files(large_code_files),
        caveats: build_caveats(config, files, git, walk),
    };

    apply_budget(&mut briefing, budget);
    briefing
}

fn build_repo_summary(repo: &RepoInfo, files: &[ImportantFile]) -> Vec<String> {
    let mut bullets = Vec::new();
    bullets.push(describe_repo_shape(repo, files));

    if !repo.primary_languages.is_empty() {
        bullets.push(format!(
            "Primary languages: {}.",
            repo.primary_languages.join(", ")
        ));
    }

    let mut guidance = Vec::new();
    if has_file(files, "AGENTS.md") {
        guidance.push("AGENTS.md");
    }
    if has_root_file(files, "README.md") || has_root_file(files, "README") {
        guidance.push("README");
    }
    if !guidance.is_empty() {
        bullets.push(format!(
            "Guidance files available: {}.",
            guidance.join(", ")
        ));
    }

    bullets.truncate(3);
    bullets
}

fn build_active_work(git: &GitResult) -> Vec<String> {
    if git.changes.is_empty() {
        return vec![git.summary.trim().trim_end_matches('.').to_string()];
    }

    let mut changes = git.changes.clone();
    changes.sort_by_key(|change| {
        (
            Reverse(change_priority(&change.path)),
            change.path.components().count(),
            change.path.clone(),
        )
    });

    changes
        .into_iter()
        .take(4)
        .map(|change| format!("{} `{}`", change.kind, change.path.display()))
        .collect()
}

fn build_read_these_first(files: &[ImportantFile]) -> Vec<BriefingItem> {
    let mut ordered = files
        .iter()
        .filter(|file| {
            matches!(
                file.category,
                SignalCategory::Instructions
                    | SignalCategory::Overview
                    | SignalCategory::Manifest
                    | SignalCategory::SupportingDoc
                    | SignalCategory::ChangedSource
                    | SignalCategory::IncludedSource
                    | SignalCategory::EntryPoint
                    | SignalCategory::Build
            )
        })
        .collect::<Vec<_>>();

    ordered.sort_by_key(|file| {
        (
            category_rank_for_file(file),
            Reverse(file.score),
            file.path.components().count(),
            file.path.clone(),
        )
    });

    ordered
        .into_iter()
        .take(5)
        .map(|file| BriefingItem {
            path: file.path.clone(),
            reason: file.reason.clone(),
        })
        .collect()
}

fn build_likely_entry_points(files: &[ImportantFile]) -> Vec<BriefingItem> {
    let mut ordered = files
        .iter()
        .filter(|file| {
            matches!(
                file.category,
                SignalCategory::EntryPoint | SignalCategory::Build
            ) || is_ranked_entrypoint(file.file_name().unwrap_or_default())
        })
        .collect::<Vec<_>>();

    ordered.sort_by_key(|file| {
        (
            entrypoint_rank(file.file_name().unwrap_or_default()),
            Reverse(file.score),
            file.path.clone(),
        )
    });

    ordered
        .into_iter()
        .take(4)
        .map(|file| BriefingItem {
            path: file.path.clone(),
            reason: file.reason.clone(),
        })
        .collect()
}

fn is_ranked_entrypoint(file_name: &str) -> bool {
    entrypoint_rank(file_name) < 16
}

fn build_caveats(
    config: &AppConfig,
    files: &[ImportantFile],
    git: &GitResult,
    walk: &WalkResult,
) -> Vec<String> {
    let mut caveats = Vec::new();
    let has_readme_on_disk = has_repo_file(config, "README.md") || has_repo_file(config, "README");

    if !has_file(files, "AGENTS.md") {
        caveats.push("No AGENTS.md found.".to_string());
    }
    if !has_readme_on_disk {
        caveats.push("No README found.".to_string());
    } else if !has_file(files, "README.md") && !has_file(files, "README") {
        caveats.push("README was omitted as low-signal or placeholder-heavy.".to_string());
    }
    if config.no_git {
        caveats.push("Git collection disabled.".to_string());
    } else if !git.available {
        caveats.push("Git context unavailable.".to_string());
    }
    if config.no_tree {
        caveats.push("Tree output disabled.".to_string());
    }

    for note in walk.notes.iter().filter(|note| {
        note.contains("omitted") || note.contains("truncated") || note.contains("missing")
    }) {
        caveats.push(note.clone());
    }

    caveats.truncate(4);
    caveats
}

fn build_large_code_files(files: &[LargeCodeFile]) -> Vec<LargeCodeFile> {
    files.iter().take(3).cloned().collect()
}

fn apply_budget(briefing: &mut AgentBriefing, budget: usize) {
    while estimated_size(briefing) > budget {
        if briefing.likely_entry_points.len() > 2 {
            briefing.likely_entry_points.pop();
            continue;
        }
        if briefing.docker_summary.len() > 1 {
            briefing.docker_summary.pop();
            continue;
        }
        if briefing.dependency_summary.len() > 1 {
            briefing.dependency_summary.pop();
            continue;
        }
        if briefing.large_code_files.len() > 2 {
            briefing.large_code_files.pop();
            continue;
        }
        if briefing.caveats.len() > 2 {
            briefing.caveats.pop();
            continue;
        }
        if briefing.active_work.len() > 2 {
            briefing.active_work.pop();
            continue;
        }
        if briefing.read_these_first.len() > 3 {
            briefing.read_these_first.pop();
            continue;
        }
        break;
    }
}

fn estimated_size(briefing: &AgentBriefing) -> usize {
    let mut size = 0usize;
    size += briefing
        .repo_summary
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size += briefing
        .active_work
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size += briefing
        .read_these_first
        .iter()
        .map(|item| item.path.display().to_string().len() + item.reason.len())
        .sum::<usize>();
    size += briefing
        .likely_entry_points
        .iter()
        .map(|item| item.path.display().to_string().len() + item.reason.len())
        .sum::<usize>();
    size += briefing
        .docker_summary
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size += briefing
        .dependency_summary
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size += briefing
        .large_code_files
        .iter()
        .map(|item| item.path.display().to_string().len() + item.reason.len() + 8)
        .sum::<usize>();
    size += briefing
        .caveats
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size
}

fn describe_repo_shape(repo: &RepoInfo, files: &[ImportantFile]) -> String {
    if repo.project_types.iter().any(|item| item == "java")
        && repo.project_types.iter().any(|item| item == "node")
    {
        return "Likely a mixed Java and Node monolith with service orchestration.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "rust") {
        if has_file(files, "main.rs") || has_file(files, "Makefile") {
            return "Likely a Rust CLI or developer tooling project.".to_string();
        }
        return "Likely a Rust project with Cargo-based entry points.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "python") {
        return "Likely a Python project with manifest-driven setup.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "java") {
        return "Likely a Java or JVM project with Maven/Gradle build entry points.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "node") {
        return "Likely a Node or TypeScript project with manifest-driven setup.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "go") {
        return "Likely a Go project with module-based entry points.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "c")
        && repo.project_types.iter().any(|item| item == "coq")
    {
        return "Likely a low-level language or formal methods project with C and Coq code."
            .to_string();
    }
    if repo.project_types.iter().any(|item| item == "c") {
        return "Likely a C project with Makefile-driven build entry points.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "coq") {
        return "Likely a Coq project with proof-oriented source files.".to_string();
    }

    "Repository type is inferred heuristically from selected files.".to_string()
}

fn category_rank(category: SignalCategory) -> usize {
    match category {
        SignalCategory::Instructions => 0,
        SignalCategory::Overview => 1,
        SignalCategory::Manifest => 2,
        SignalCategory::ChangedSource => 3,
        SignalCategory::IncludedSource => 4,
        SignalCategory::EntryPoint => 5,
        SignalCategory::Build => 6,
        SignalCategory::Config => 7,
        SignalCategory::SupportingDoc => 8,
    }
}

fn category_rank_for_file(file: &ImportantFile) -> usize {
    if is_high_signal_guide(file) {
        return 1;
    }

    if matches!(file.category, SignalCategory::Overview)
        && file.reason.contains("placeholder-heavy template")
    {
        return 3;
    }

    category_rank(file.category)
}

fn entrypoint_rank(file_name: &str) -> usize {
    if file_name == "docker-compose.yml" || file_name == "docker-compose.yaml" {
        return 11;
    }
    if file_name == "compose.yml" || file_name == "compose.yaml" {
        return 12;
    }
    if file_name.starts_with("Dockerfile") {
        return 13;
    }

    match file_name {
        "main.rs" => 0,
        "lib.rs" => 1,
        "main.go" => 2,
        "app.py" => 3,
        "index.js" => 4,
        "index.ts" => 5,
        "main.js" => 6,
        "main.ts" => 7,
        "app.js" => 8,
        "server.js" => 9,
        "server.ts" => 10,
        "App.tsx" => 14,
        "Makefile" => 15,
        "Justfile" => 16,
        "Taskfile.yml" => 17,
        "Taskfile.yaml" => 18,
        _ => 19,
    }
}

fn change_priority(path: &std::path::Path) -> usize {
    match path.extension().and_then(|value| value.to_str()) {
        Some("rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt") => 3,
        Some("md") => 2,
        Some("toml" | "json" | "yml" | "yaml") => 1,
        _ => 0,
    }
}

fn has_file(files: &[ImportantFile], name: &str) -> bool {
    files.iter().any(|file| file.file_name() == Some(name))
}

fn has_root_file(files: &[ImportantFile], name: &str) -> bool {
    files
        .iter()
        .any(|file| file.file_name() == Some(name) && file.path.components().count() == 1)
}

fn has_repo_file(config: &AppConfig, name: &str) -> bool {
    config.cwd.join(name).exists()
}

fn is_high_signal_guide(file: &ImportantFile) -> bool {
    if !matches!(file.category, SignalCategory::SupportingDoc) {
        return false;
    }

    matches!(
        file.file_name(),
        Some("ARCHITECTURE.md")
            | Some("DATA_SOURCES.md")
            | Some("DESIGN.md")
            | Some("OPERATIONS.md")
            | Some("RUNBOOK.md")
            | Some("SERIES_GUIDE.md")
            | Some("TROUBLESHOOTING.md")
    ) || file
        .file_name()
        .map(|name| name.ends_with("_GUIDE.md") || name.ends_with("_OVERVIEW.md"))
        .unwrap_or(false)
}
