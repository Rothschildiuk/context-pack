//! Assemble the briefing from every stage, fit it into the byte budget, and
//! spend whatever budget is left on excerpts.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::commands;
use crate::excerpt;
use crate::git;
use crate::index::RepoIndex;
use crate::layout;
use crate::manifest::{self, Ecosystem, Manifest};
use crate::memory;
use crate::model::{
    AppConfig, Brief, DependencyList, LanguageShare, RepoSummary, Stats, Workspace, WorkspaceGroup,
    SCHEMA_VERSION,
};
use crate::paths::{self, PathRole};
use crate::render_markdown;
use crate::select;

const MAX_MEMORY_BYTES: usize = 2000;
const MIN_EXCERPT_BUDGET: usize = 300;

pub fn build(config: &AppConfig) -> Brief {
    let started = Instant::now();
    let index = RepoIndex::build(config);
    let manifests = manifest::collect(&index);
    let git = if config.no_git {
        git::GitResult {
            info: None,
            notes: Vec::new(),
        }
    } else {
        git::collect(&config.cwd)
    };
    let git_info = git.info.as_ref();

    let selection = select::select(
        &index,
        &manifests,
        git_info,
        config.max_files,
        config.changed_only,
    );
    let (layout, layout_notes) = if config.no_layout {
        (Vec::new(), Vec::new())
    } else {
        layout::build(&index)
    };

    let mut memory = memory::inspect(&config.cwd, git_info);
    if let Some(memory) = memory.as_mut() {
        if memory.notes.len() > MAX_MEMORY_BYTES {
            memory.notes = truncate_at_line(&memory.notes, MAX_MEMORY_BYTES);
            memory.truncated = true;
        }
    }

    let mut notes = Vec::new();
    if index.files.is_empty() {
        notes.push(format!("no files found under {}", config.cwd.display()));
    }
    if index.truncated {
        notes.push(
            "file index truncated: repository is very large, some directories were not scanned"
                .to_string(),
        );
    }
    if selection.instructions.is_empty() {
        notes.push("no agent instruction files (AGENTS.md, CLAUDE.md, ...) found".to_string());
    }
    if config.changed_only {
        notes.push("changed-only mode: key files limited to active work".to_string());
    }
    notes.extend(git.notes.iter().cloned());
    notes.extend(layout_notes);

    let mut brief = Brief {
        schema_version: SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION"),
        repo: repo_summary(config, &index, &manifests),
        instructions: selection.instructions,
        commands: commands::collect(&index, &manifests),
        entry_points: selection.entry_points,
        key_files: selection.key_files,
        workspace: workspace(&index, &manifests),
        layout,
        docs: selection.docs,
        config: selection.config,
        git: git.info,
        memory,
        excerpts: Vec::new(),
        notes,
        stats: Stats {
            files_indexed: index.files.len(),
            elapsed_ms: 0,
            generated_from_commit: None,
        },
    };
    brief.stats.generated_from_commit = brief.git.as_ref().and_then(|info| info.head.clone());

    fit_to_budget(&mut brief, config.max_bytes);
    if config.excerpts {
        add_excerpts(&mut brief, &index, config.max_bytes);
    }
    brief.stats.elapsed_ms = started.elapsed().as_millis();
    brief
}

// ---------------------------------------------------------------------------
// Repo summary

fn repo_summary(config: &AppConfig, index: &RepoIndex, manifests: &[Manifest]) -> RepoSummary {
    let primary = primary_manifests(manifests);
    let dir_name = config
        .cwd
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repository")
        .to_string();
    let name = primary
        .iter()
        .filter(|manifest| manifest.dir().as_os_str().is_empty())
        .filter_map(|manifest| manifest.name.clone())
        .find(|name| !is_placeholder_name(name))
        .unwrap_or(dir_name);

    // Monorepo roots rarely describe themselves; the package named like the
    // repo usually does (`packages/astro` in the astro repo).
    let namesake = manifests.iter().find(|manifest| {
        manifest.description.is_some()
            && !paths::role(&manifest.path).is_secondary()
            && manifest
                .name
                .as_deref()
                .is_some_and(|value| value.rsplit('/').next() == Some(name.as_str()))
    });
    let description = primary
        .iter()
        .find_map(|manifest| manifest.description.clone())
        .or_else(|| namesake.and_then(|manifest| manifest.description.clone()))
        .filter(|value| !value.is_empty())
        .or_else(|| readme_description(index))
        .map(|value| truncate_sentence(&value, 280));

    RepoSummary {
        name,
        path: config.cwd.display().to_string(),
        description,
        languages: languages(index),
        stack: stack(index, &primary),
        dependencies: primary
            .iter()
            .filter(|manifest| {
                !manifest.dependencies.is_empty() || !manifest.dev_dependencies.is_empty()
            })
            .take(3)
            .map(|manifest| DependencyList {
                manifest: paths::display(&manifest.path),
                runtime: manifest.dependencies.iter().take(15).cloned().collect(),
                dev: manifest.dev_dependencies.iter().take(10).cloned().collect(),
            })
            .collect(),
    }
}

/// Monorepo roots are often named `root` or `monorepo`; the directory says more.
fn is_placeholder_name(name: &str) -> bool {
    matches!(
        name.trim_start_matches('@').to_ascii_lowercase().as_str(),
        "root" | "monorepo" | "workspace" | "workspaces" | "project" | "app" | "main" | "repo"
    )
}

/// Manifests that describe the project itself: the shallowest non-example ones.
fn primary_manifests(manifests: &[Manifest]) -> Vec<&Manifest> {
    let candidates = manifests
        .iter()
        .filter(|manifest| !paths::role(&manifest.path).is_secondary())
        .collect::<Vec<_>>();
    let Some(depth) = candidates
        .iter()
        .map(|manifest| paths::depth(&manifest.path))
        .min()
    else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter(|manifest| paths::depth(&manifest.path) == depth)
        .collect()
}

fn languages(index: &RepoIndex) -> Vec<LanguageShare> {
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    let mut fallback = std::collections::BTreeMap::<&str, usize>::new();
    for entry in &index.files {
        let Some(language) = paths::language(&entry.path) else {
            continue;
        };
        *fallback.entry(language).or_default() += 1;
        if paths::role(&entry.path) == PathRole::Source && paths::is_primary_language(language) {
            *counts.entry(language).or_default() += 1;
        }
    }
    let counts = if counts.is_empty() { fallback } else { counts };
    let total = counts.values().sum::<usize>().max(1);
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    ranked
        .into_iter()
        .filter(|(_, count)| count * 100 >= total * 3)
        .take(5)
        .map(|(name, files)| LanguageShare {
            name: name.to_string(),
            files,
        })
        .collect()
}

fn stack(index: &RepoIndex, primary: &[&Manifest]) -> Vec<String> {
    let mut stack = Vec::new();
    let mut push = |value: String| {
        if !stack.contains(&value) {
            stack.push(value);
        }
    };
    for manifest in primary {
        match manifest.ecosystem {
            Ecosystem::Npm => {
                let manager = if index.contains("pnpm-lock.yaml") {
                    "pnpm"
                } else if index.contains("yarn.lock") {
                    "yarn"
                } else if index.contains("bun.lockb") || index.contains("bun.lock") {
                    "bun"
                } else {
                    "npm"
                };
                push(manager.to_string());
            }
            Ecosystem::Python => {
                let dir = manifest.dir();
                let tool = if index.contains(dir.join("uv.lock")) {
                    "python (uv)"
                } else if index.contains(dir.join("poetry.lock")) || manifest.has_tool("poetry") {
                    "python (poetry)"
                } else {
                    "python"
                };
                push(tool.to_string());
            }
            other => push(other.label().to_string()),
        }
        for (dependency, label) in FRAMEWORKS {
            if manifest.has_dependency(dependency)
                || manifest
                    .dependencies
                    .iter()
                    .any(|dep| dep.ends_with(&format!("/{dependency}")))
            {
                push(label.to_string());
            }
        }
    }
    for (file, label) in [
        ("turbo.json", "turborepo"),
        ("nx.json", "nx"),
        ("Dockerfile", "docker"),
        ("flake.nix", "nix"),
    ] {
        if index.contains(file) {
            push(label.to_string());
        }
    }
    stack.truncate(8);
    stack
}

const FRAMEWORKS: &[(&str, &str)] = &[
    ("next", "next.js"),
    ("react", "react"),
    ("vue", "vue"),
    ("svelte", "svelte"),
    ("astro", "astro"),
    ("nuxt", "nuxt"),
    ("express", "express"),
    ("fastify", "fastify"),
    ("@nestjs/core", "nestjs"),
    ("hono", "hono"),
    ("electron", "electron"),
    ("react-native", "react-native"),
    ("vite", "vite"),
    ("django", "django"),
    ("fastapi", "fastapi"),
    ("flask", "flask"),
    ("torch", "pytorch"),
    ("tokio", "tokio"),
    ("axum", "axum"),
    ("actix-web", "actix-web"),
    ("clap", "clap"),
    ("tauri", "tauri"),
    ("gin", "gin"),
    ("cobra", "cobra"),
    ("spring-boot-starter-web", "spring-boot"),
    ("spring-boot-starter", "spring-boot"),
    ("rails", "rails"),
    ("framework", "laravel"),
];

fn readme_description(index: &RepoIndex) -> Option<String> {
    let path = ["README.md", "README", "README.rst", "readme.md"]
        .into_iter()
        .find(|name| index.contains(name))?;
    let text = index.read(path)?;
    let mut paragraph = Vec::new();
    let mut in_code = false;
    let mut in_html = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        // Only the introduction counts: stop at the first section heading.
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            break;
        }
        // Text glued to HTML (centered taglines, badges) is decoration, not a description.
        if trimmed.starts_with('<') {
            in_html = true;
            paragraph.clear();
            continue;
        }
        if in_html {
            in_html = !trimmed.is_empty();
            continue;
        }
        let noise = trimmed.starts_with('#')
            || trimmed.starts_with('<')
            || trimmed.starts_with("[![")
            || trimmed.starts_with("![")
            || trimmed.starts_with('>') && paragraph.is_empty()
            || trimmed.starts_with('|')
            || trimmed.starts_with("---")
            || trimmed.starts_with("===")
            || trimmed.starts_with("[!")
            || trimmed.chars().all(|ch| !ch.is_alphanumeric());
        if trimmed.is_empty() || noise {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed.to_string());
    }
    let text = strip_markdown(&paragraph.join(" "));
    (text.len() >= 20).then_some(text)
}

fn strip_markdown(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '[' => {
                let mut label = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }
                if chars.peek() == Some(&'(') {
                    for next in chars.by_ref() {
                        if next == ')' {
                            break;
                        }
                    }
                }
                output.push_str(&label);
            }
            '*' | '_' if chars.peek() == Some(&ch) => {
                chars.next();
            }
            _ => output.push(ch),
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_sentence(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut cut = max;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let head = &text[..cut];
    match head.rfind(". ") {
        Some(end) if end > max / 2 => head[..=end].to_string(),
        _ => format!("{}…", head.trim_end()),
    }
}

fn truncate_at_line(text: &str, max: usize) -> String {
    let mut output = String::new();
    for line in text.lines() {
        if output.len() + line.len() + 1 > max {
            break;
        }
        output.push_str(line);
        output.push('\n');
    }
    output.trim_end().to_string()
}

// ---------------------------------------------------------------------------
// Workspace

fn workspace(index: &RepoIndex, manifests: &[Manifest]) -> Option<Workspace> {
    // Test fixtures often carry their own manifests; they are not packages.
    let members = manifests
        .iter()
        .filter(|manifest| {
            !matches!(
                paths::role(&manifest.path),
                PathRole::Tests | PathRole::Fixtures | PathRole::Vendor
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let groups = manifest::workspace_groups(&members);
    let packages = groups.values().map(Vec::len).sum::<usize>();
    if packages < 2 {
        return None;
    }

    let mut groups = groups
        .into_iter()
        .map(|(pattern, members)| {
            let role = paths::role(&PathBuf::from(pattern.trim_end_matches("/*")).join("x"));
            let mut members = members;
            // Larger packages first: they are more likely to be the core.
            members.sort_by_key(|manifest| std::cmp::Reverse(index.files_under(manifest.dir())));
            WorkspaceGroup {
                role: role.label().to_string(),
                count: members.len(),
                examples: members
                    .iter()
                    .take(5)
                    .map(|manifest| {
                        // Relative to the group pattern: `cloudflare (@astrojs/cloudflare)`.
                        let last = paths::file_name(manifest.dir()).to_string();
                        match &manifest.name {
                            Some(name) if *name != last => format!("{last} ({name})"),
                            _ => last,
                        }
                    })
                    .collect(),
                pattern,
            }
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        (left.role != "source")
            .cmp(&(right.role != "source"))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.pattern.cmp(&right.pattern))
    });
    groups.truncate(8);
    Some(Workspace { packages, groups })
}

// ---------------------------------------------------------------------------
// Budget

/// Trim the least important detail until the markdown rendering fits.
fn fit_to_budget(brief: &mut Brief, max_bytes: usize) {
    type Step = fn(&mut Brief) -> bool;
    let steps: &[Step] = &[
        |brief| shrink(&mut brief.repo.dependencies, 0),
        |brief| {
            let before = brief
                .layout
                .iter()
                .map(|entry| entry.children.len())
                .sum::<usize>();
            brief
                .layout
                .iter_mut()
                .for_each(|entry| entry.children.truncate(3));
            before
                != brief
                    .layout
                    .iter()
                    .map(|entry| entry.children.len())
                    .sum::<usize>()
        },
        |brief| {
            brief
                .git
                .as_mut()
                .is_some_and(|git| shrink(&mut git.recent_commits, 2))
        },
        |brief| shrink(&mut brief.docs, 3),
        |brief| shrink(&mut brief.config, 3),
        |brief| shrink(&mut brief.key_files, 6),
        |brief| {
            let changed = brief.layout.iter().any(|entry| !entry.children.is_empty());
            brief
                .layout
                .iter_mut()
                .for_each(|entry| entry.children.clear());
            changed
        },
        |brief| {
            brief.git.as_mut().is_some_and(|git| {
                shrink(&mut git.working_changes, 8) | shrink(&mut git.branch_changes, 8)
            })
        },
        |brief| shrink(&mut brief.layout, 8),
        |brief| {
            let before = brief.commands.len();
            let mut ci = 0;
            brief.commands.retain(|hint| {
                hint.kind != "ci" || {
                    ci += 1;
                    ci <= 2
                }
            });
            before != brief.commands.len()
        },
        |brief| {
            brief
                .workspace
                .as_mut()
                .is_some_and(|workspace| shrink(&mut workspace.groups, 3))
        },
        |brief| shrink(&mut brief.key_files, 4),
        |brief| {
            brief.memory.as_mut().is_some_and(|memory| {
                if memory.notes.len() <= 600 {
                    return false;
                }
                memory.notes = truncate_at_line(&memory.notes, 600);
                memory.truncated = true;
                true
            })
        },
        |brief| shrink(&mut brief.docs, 1),
        |brief| shrink(&mut brief.config, 0),
        |brief| shrink(&mut brief.layout, 4),
        |brief| shrink(&mut brief.instructions, 4),
        |brief| shrink(&mut brief.entry_points, 3),
        |brief| shrink(&mut brief.key_files, 2),
        |brief| {
            brief.git.as_mut().is_some_and(|git| {
                shrink(&mut git.working_changes, 3)
                    | shrink(&mut git.branch_changes, 3)
                    | shrink(&mut git.recent_commits, 0)
            })
        },
        |brief| shrink(&mut brief.commands, 6),
        |brief| shrink(&mut brief.layout, 0),
        |brief| {
            let before = brief.instructions.len();
            brief
                .instructions
                .retain(|file| !file.reason.starts_with("agent skill"));
            before != brief.instructions.len()
        },
        |brief| {
            brief.workspace.as_mut().is_some_and(|workspace| {
                let changed = shrink(&mut workspace.groups, 1);
                workspace.groups.iter_mut().fold(changed, |changed, group| {
                    shrink(&mut group.examples, 2) | changed
                })
            })
        },
        |brief| {
            // One command per kind.
            let before = brief.commands.len();
            let mut kinds = Vec::new();
            brief.commands.retain(|hint| {
                let first = !kinds.contains(&hint.kind);
                kinds.push(hint.kind.clone());
                first
            });
            before != brief.commands.len()
        },
        |brief| shrink(&mut brief.docs, 0),
        |brief| brief.workspace.take().is_some(),
        |brief| shrink(&mut brief.entry_points, 1),
        |brief| shrink(&mut brief.commands, 3),
    ];

    if render_markdown::render(brief).len() <= max_bytes {
        return;
    }
    // Add the note first so the final size check accounts for it.
    brief
        .notes
        .push(format!("output trimmed to fit --max-bytes {max_bytes}"));
    for step in steps {
        if render_markdown::render(brief).len() <= max_bytes {
            break;
        }
        step(brief);
    }
}

fn shrink<T>(items: &mut Vec<T>, keep: usize) -> bool {
    if items.len() > keep {
        items.truncate(keep);
        true
    } else {
        false
    }
}

fn add_excerpts(brief: &mut Brief, index: &RepoIndex, max_bytes: usize) {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for file in brief
        .instructions
        .iter()
        .filter(|file| !file.path.contains('/'))
        .take(2)
    {
        candidates.push(PathBuf::from(&file.path));
    }
    if brief.instructions.is_empty() {
        if let Some(readme) = brief
            .docs
            .iter()
            .find(|file| file.reason == "project overview")
        {
            candidates.push(PathBuf::from(&readme.path));
        }
    }
    for file in brief
        .key_files
        .iter()
        .filter(|file| file.reason.contains("changed"))
        .take(2)
    {
        candidates.push(PathBuf::from(&file.path));
    }
    for file in brief
        .entry_points
        .iter()
        .take(1)
        .chain(brief.key_files.iter().take(2))
    {
        candidates.push(PathBuf::from(&file.path));
    }
    candidates.dedup();

    let mut used = render_markdown::render(brief).len();
    for path in candidates {
        let remaining = max_bytes.saturating_sub(used);
        if remaining < MIN_EXCERPT_BUDGET {
            break;
        }
        if brief
            .excerpts
            .iter()
            .any(|excerpt| Path::new(&excerpt.path) == path)
        {
            continue;
        }
        // Leave room for the fenced block header, and never let one file eat everything.
        let budget = (remaining - 60)
            .min(max_bytes / 3)
            .max(MIN_EXCERPT_BUDGET - 60);
        let Some(excerpt) = excerpt::excerpt(index, &path, budget) else {
            continue;
        };
        brief.excerpts.push(excerpt);
        used = render_markdown::render(brief).len();
        if used > max_bytes {
            brief.excerpts.pop();
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_links_and_emphasis_are_stripped() {
        assert_eq!(
            strip_markdown("A **fast** [CLI](https://x) for __agents__"),
            "A fast CLI for agents"
        );
    }

    #[test]
    fn long_descriptions_end_on_a_sentence() {
        let text = "First sentence is here. Second sentence goes on and on and on.";
        assert_eq!(truncate_sentence(text, 40), "First sentence is here.");
    }
}
