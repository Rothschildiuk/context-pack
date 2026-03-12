use std::cmp::Reverse;

use crate::model::{
    AgentBriefing, AppConfig, BriefingItem, GitResult, ImportantFile, RepoInfo, SignalCategory,
    WalkResult,
};

pub fn build(
    config: &AppConfig,
    repo: &RepoInfo,
    files: &[ImportantFile],
    git: &GitResult,
    walk: &WalkResult,
    budget: usize,
) -> AgentBriefing {
    let mut briefing = AgentBriefing {
        repo_summary: build_repo_summary(repo, files),
        active_work: build_active_work(git),
        read_these_first: build_read_these_first(files),
        likely_entry_points: build_likely_entry_points(files),
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
    if has_file(files, "README.md") || has_file(files, "README") {
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
                    | SignalCategory::ChangedSource
                    | SignalCategory::EntryPoint
                    | SignalCategory::Build
            )
        })
        .collect::<Vec<_>>();

    ordered.sort_by_key(|file| {
        (
            category_rank(file.category),
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
    entrypoint_rank(file_name) < 8
}

fn build_caveats(
    config: &AppConfig,
    files: &[ImportantFile],
    git: &GitResult,
    walk: &WalkResult,
) -> Vec<String> {
    let mut caveats = Vec::new();

    if !has_file(files, "AGENTS.md") {
        caveats.push("No AGENTS.md found.".to_string());
    }
    if !has_file(files, "README.md") && !has_file(files, "README") {
        caveats.push("No README found.".to_string());
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

fn apply_budget(briefing: &mut AgentBriefing, budget: usize) {
    while estimated_size(briefing) > budget {
        if briefing.likely_entry_points.len() > 2 {
            briefing.likely_entry_points.pop();
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
        .caveats
        .iter()
        .map(|item| item.len())
        .sum::<usize>();
    size
}

fn describe_repo_shape(repo: &RepoInfo, files: &[ImportantFile]) -> String {
    if repo.project_types.iter().any(|item| item == "rust") && has_file(files, "Cargo.toml") {
        if has_file(files, "main.rs") || has_file(files, "Makefile") {
            return "Likely a Rust CLI or developer tooling project.".to_string();
        }
        return "Likely a Rust project with Cargo-based entry points.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "python") {
        return "Likely a Python project with manifest-driven setup.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "node") {
        return "Likely a Node or TypeScript project with manifest-driven setup.".to_string();
    }
    if repo.project_types.iter().any(|item| item == "go") {
        return "Likely a Go project with module-based entry points.".to_string();
    }

    "Repository type is inferred heuristically from selected files.".to_string()
}

fn category_rank(category: SignalCategory) -> usize {
    match category {
        SignalCategory::Instructions => 0,
        SignalCategory::Overview => 1,
        SignalCategory::Manifest => 2,
        SignalCategory::ChangedSource => 3,
        SignalCategory::EntryPoint => 4,
        SignalCategory::Build => 5,
        SignalCategory::Config => 6,
        SignalCategory::SupportingDoc => 7,
    }
}

fn entrypoint_rank(file_name: &str) -> usize {
    match file_name {
        "main.rs" => 0,
        "lib.rs" => 1,
        "main.go" => 2,
        "app.py" => 3,
        "index.ts" => 4,
        "main.ts" => 5,
        "App.tsx" => 6,
        "Makefile" => 7,
        _ => 8,
    }
}

fn change_priority(path: &std::path::Path) -> usize {
    match path.extension().and_then(|value| value.to_str()) {
        Some("rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx") => 3,
        Some("md") => 2,
        Some("toml" | "json" | "yml" | "yaml") => 1,
        _ => 0,
    }
}

fn has_file(files: &[ImportantFile], name: &str) -> bool {
    files.iter().any(|file| file.file_name() == Some(name))
}
