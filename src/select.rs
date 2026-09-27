//! Decide which files an agent should know about: instructions, entry points,
//! key source files, supporting docs, and configuration.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::index::RepoIndex;
use crate::manifest::Manifest;
use crate::model::{FileRef, GitInfo};
use crate::paths::{self, PathRole};

pub struct Selection {
    pub instructions: Vec<FileRef>,
    pub entry_points: Vec<FileRef>,
    pub key_files: Vec<FileRef>,
    pub docs: Vec<FileRef>,
    pub config: Vec<FileRef>,
}

pub fn select(
    index: &RepoIndex,
    manifests: &[Manifest],
    git: Option<&GitInfo>,
    max_files: usize,
    changed_only: bool,
) -> Selection {
    let instructions = instructions(index);
    let entry_points = entry_points(index, manifests);
    let taken = entry_points
        .iter()
        .map(|file| PathBuf::from(&file.path))
        .collect::<HashSet<_>>();
    let key_files = key_files(
        index,
        manifests,
        git,
        &entry_points,
        &taken,
        max_files,
        changed_only,
    );

    Selection {
        instructions,
        entry_points,
        key_files,
        docs: docs(index),
        config: config(index),
    }
}

// ---------------------------------------------------------------------------
// Agent instructions

/// Known agent instruction files: (matcher, reason). Order is priority.
fn instruction_reason(path: &Path) -> Option<&'static str> {
    let display = paths::display(path);
    let name = paths::file_name(path);
    let reason = match display.as_str() {
        ".github/copilot-instructions.md" => "GitHub Copilot instructions",
        ".cursorrules" => "Cursor rules",
        ".windsurfrules" => "Windsurf rules",
        ".clinerules" => "Cline rules",
        "CONVENTIONS.md" => "coding conventions (aider)",
        "REPO_MEMORY.md" => "repo memory",
        _ if display.starts_with(".cursor/rules/") => "Cursor rule",
        _ if display.starts_with(".windsurf/rules/") => "Windsurf rule",
        _ if display.starts_with(".clinerules/") => "Cline rule",
        _ if display.starts_with(".github/instructions/") && name.ends_with(".instructions.md") => {
            "GitHub Copilot path instructions"
        }
        _ if name == "SKILL.md"
            && (display.starts_with(".claude/skills/")
                || display.starts_with(".agents/skills/")
                || display.starts_with("skills/")) =>
        {
            "agent skill"
        }
        _ => match name {
            "AGENTS.md" | "AGENT.md" => "agent instructions",
            "CLAUDE.md" | "CLAUDE.local.md" => "Claude Code instructions",
            "GEMINI.md" => "Gemini CLI instructions",
            _ => return None,
        },
    };
    Some(reason)
}

fn instructions(index: &RepoIndex) -> Vec<FileRef> {
    let mut found = index
        .files
        .iter()
        .filter(|entry| entry.size > 0)
        .filter(|entry| {
            !matches!(
                paths::role(&entry.path),
                PathRole::Tests | PathRole::Fixtures | PathRole::Vendor | PathRole::Examples
            )
        })
        .filter_map(|entry| {
            let reason = instruction_reason(&entry.path)?;
            if reason == "agent skill" {
                let skill = entry
                    .path
                    .parent()
                    .map(paths::file_name)
                    .unwrap_or_default();
                return Some((entry.path.clone(), format!("agent skill `{skill}`")));
            }
            let scope = entry
                .path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .filter(|parent| {
                    !paths::file_name(parent).starts_with('.')
                        && !paths::display(parent).starts_with('.')
                })
                .map(|parent| format!(" (applies to `{}/`)", paths::display(parent)))
                .unwrap_or_default();
            Some((entry.path.clone(), format!("{reason}{scope}")))
        })
        .collect::<Vec<_>>();

    found.sort_by_key(|(path, _)| (instruction_rank(path), paths::depth(path), path.clone()));
    let mut skills = 0;
    found.retain(|(_, reason)| {
        if reason.starts_with("agent skill") {
            skills += 1;
            skills <= 5
        } else {
            true
        }
    });
    found
        .into_iter()
        .take(12)
        .map(|(path, reason)| FileRef {
            lines: line_count(index, &path),
            path: paths::display(&path),
            reason,
        })
        .collect()
}

/// Root instructions first, then scoped ones, then skills.
fn instruction_rank(path: &Path) -> usize {
    let name = paths::file_name(path);
    let root = paths::depth(path) == 0;
    match name {
        "AGENTS.md" if root => 0,
        "CLAUDE.md" if root => 1,
        "SKILL.md" => 4,
        _ if root || paths::display(path).starts_with('.') => 2,
        _ => 3,
    }
}

// ---------------------------------------------------------------------------
// Entry points

fn entry_points(index: &RepoIndex, manifests: &[Manifest]) -> Vec<FileRef> {
    let mut candidates: Vec<(PathBuf, String, usize)> = Vec::new();
    let mut push = |path: PathBuf, reason: String, score: usize| {
        if let Some(existing) = candidates.iter_mut().find(|(known, _, _)| *known == path) {
            existing.2 = existing.2.max(score);
            return;
        }
        candidates.push((path, reason, score));
    };

    for manifest in manifests {
        let role = paths::role(&manifest.path);
        // In a monorepo the entry of the biggest package is usually the one that matters.
        let package_files = index.files_under(manifest.dir()).max(1) as f64;
        let size_bonus = (package_files.log2() * 20.0).min(220.0) as usize;
        for (path, reason) in &manifest.entry_points {
            let manifest_label = paths::display(&manifest.path);
            push(
                path.clone(),
                format!("{reason} ({manifest_label})"),
                1000 + role_bonus(role) + size_bonus,
            );
        }
    }

    for entry in &index.files {
        let Some(reason) = conventional_entry_reason(&entry.path) else {
            continue;
        };
        push(
            entry.path.clone(),
            reason.to_string(),
            600 + role_bonus(paths::role(&entry.path)),
        );
    }

    for dockerfile in index.files.iter().filter(|entry| {
        paths::file_name(&entry.path) == "Dockerfile" && paths::depth(&entry.path) <= 1
    }) {
        if let Some(target) = docker_entry(index, &dockerfile.path) {
            push(
                target,
                format!(
                    "container entrypoint ({})",
                    paths::display(&dockerfile.path)
                ),
                900,
            );
        }
    }

    candidates.sort_by_key(|(path, _, score)| {
        (
            Reverse(score.saturating_sub(paths::depth(path) * 40)),
            path.clone(),
        )
    });
    let has_production = candidates
        .iter()
        .any(|(path, _, _)| !paths::role(path).is_secondary());
    candidates
        .into_iter()
        .filter(|(path, _, _)| !has_production || !paths::role(path).is_secondary())
        .take(6)
        .map(|(path, reason, _)| FileRef {
            lines: line_count(index, &path),
            path: paths::display(&path),
            reason,
        })
        .collect()
}

fn role_bonus(role: PathRole) -> usize {
    match role {
        PathRole::Source => 300,
        PathRole::Scripts | PathRole::Ci | PathRole::Hidden => 100,
        _ => 0,
    }
}

fn conventional_entry_reason(path: &Path) -> Option<&'static str> {
    let name = paths::file_name(path);
    let parent = path.parent().map(paths::file_name).unwrap_or_default();
    let depth = paths::depth(path);
    let reason = match name {
        "__main__.py" => "python -m entry",
        "manage.py" if depth <= 1 => "Django management entry",
        "wsgi.py" | "asgi.py" if depth <= 1 => "WSGI/ASGI application",
        "main.py" | "app.py" | "server.py" | "cli.py" if depth <= 2 => "python application module",
        "main.go" if depth <= 2 => "go main package",
        "main.rs" if parent == "src" => "rust binary root",
        "lib.rs" if parent == "src" && depth <= 2 => "rust library root",
        "Program.cs" => ".NET program entry",
        "main.swift" => "swift executable",
        "main.dart" if parent == "lib" => "dart main",
        "index.ts" | "index.tsx" | "index.js" | "main.ts" | "main.tsx" | "main.js"
        | "server.ts" | "server.js" | "app.ts" | "app.js"
            if parent == "src" || depth == 0 =>
        {
            "module entry"
        }
        "layout.tsx" | "layout.jsx" | "layout.js" if parent == "app" => "Next.js app root layout",
        "_app.tsx" | "_app.jsx" | "_app.js" if parent == "pages" => "Next.js pages root",
        "App.tsx" | "App.jsx" | "App.vue" | "App.svelte" if parent == "src" => "UI root component",
        _ => return None,
    };
    Some(reason)
}

/// Resolve a Dockerfile `CMD`/`ENTRYPOINT` argument to a repo file when possible.
fn docker_entry(index: &RepoIndex, dockerfile: &Path) -> Option<PathBuf> {
    let text = index.read(dockerfile)?;
    let dir = dockerfile.parent().unwrap_or_else(|| Path::new(""));
    let line = text
        .lines()
        .rev()
        .find(|line| line.starts_with("CMD") || line.starts_with("ENTRYPOINT"))?;
    line.split(|ch: char| ch.is_whitespace() || "[],\"'".contains(ch))
        .filter(|token| token.contains('.') && !token.starts_with('-'))
        .map(|token| dir.join(token.trim_start_matches("./")))
        .find(|candidate| index.contains(candidate))
}

// ---------------------------------------------------------------------------
// Key source files

#[allow(clippy::too_many_arguments)]
fn key_files(
    index: &RepoIndex,
    manifests: &[Manifest],
    git: Option<&GitInfo>,
    entry_points: &[FileRef],
    taken: &HashSet<PathBuf>,
    max_files: usize,
    changed_only: bool,
) -> Vec<FileRef> {
    let mut selected: Vec<FileRef> = Vec::new();
    let mut seen = taken.clone();

    for path in &index.forced {
        if seen.insert(path.clone()) {
            selected.push(FileRef {
                path: paths::display(path),
                reason: "explicitly included".to_string(),
                lines: line_count(index, path),
            });
        }
    }

    // Changed files are listed even when they are also entry points: the
    // agent needs to know the entry point itself is being modified.
    // Biggest production changes first; a one-line script tweak is not the headline.
    let mut active = git.map(GitInfo::active_paths).unwrap_or_default();
    let weights = git.map(change_weights).unwrap_or_default();
    active.sort_by_key(|path| {
        let weight = weights
            .get(path.as_path())
            .copied()
            .flatten()
            .or_else(|| line_count(index, path))
            .unwrap_or(0);
        (
            paths::role(path).is_secondary(),
            Reverse(weight),
            path.clone(),
        )
    });
    for path in &active {
        if selected.len() >= max_files {
            break;
        }
        let already_listed = selected.iter().any(|file| Path::new(&file.path) == path);
        if !paths::is_source(path) || !index.contains(path) || already_listed {
            continue;
        }
        seen.insert(path.clone());
        selected.push(FileRef {
            path: paths::display(path),
            reason: "changed in active work".to_string(),
            lines: line_count(index, path),
        });
    }
    if changed_only {
        return selected;
    }

    let churn = git.map(|info| churn_map(info)).unwrap_or_default();
    let history_depth = git.map(|info| info.history_depth).unwrap_or(0);
    let referenced = referenced_from_entry_points(index, entry_points);
    let package_dirs = main_package_dirs(index, manifests);

    let mut scored = index
        .files
        .iter()
        .filter(|entry| paths::is_source(&entry.path) && !seen.contains(&entry.path))
        .filter(|entry| paths::role(&entry.path) == PathRole::Source)
        .filter(|entry| entry.size >= 200)
        .map(|entry| {
            let path = &entry.path;
            let mut score = size_score(entry.size);
            let mut reasons = Vec::new();

            let changes = churn.get(path.as_path()).copied().unwrap_or(0);
            if history_depth >= 10 && changes >= 2 {
                score += (changes * 400 / history_depth.max(1)).min(300) + 40;
                reasons.push(format!(
                    "changed in {changes} of the last {history_depth} commits"
                ));
            }
            match referenced.get(path) {
                Some(0) => {
                    score += 100;
                    reasons.push("used by the main entry point".to_string());
                }
                Some(_) => {
                    score += 40;
                    reasons.push("used by an entry point".to_string());
                }
                None => {}
            }
            if package_dirs.iter().any(|dir| path.starts_with(dir)) {
                score += 60;
            } else if !package_dirs.is_empty() {
                score = score.saturating_sub(80);
            }
            score = score.saturating_sub(paths::depth(path).saturating_sub(2) * 15);
            if paths::language(path).is_some_and(|language| !paths::is_primary_language(language)) {
                score /= 3;
            }
            (score, reasons, entry)
        })
        .collect::<Vec<_>>();

    scored.sort_by_key(|(score, _, entry)| (Reverse(*score), entry.path.clone()));

    // Avoid a key-file list that is five siblings from one directory.
    let mut per_dir: HashMap<PathBuf, usize> = HashMap::new();
    for (_, mut reasons, entry) in scored {
        if selected.len() >= max_files {
            break;
        }
        let dir = entry
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let count = per_dir.entry(dir).or_default();
        if *count >= 3 {
            continue;
        }
        *count += 1;

        let lines = line_count(index, &entry.path);
        if let Some(lines) = lines.filter(|lines| *lines >= 300) {
            reasons.insert(0, format!("large module ({lines} lines)"));
        }
        if reasons.is_empty() {
            reasons.push("core source file".to_string());
        }
        selected.push(FileRef {
            path: paths::display(&entry.path),
            reason: reasons.join(", "),
            lines,
        });
    }
    selected
}

/// Lines added + deleted per changed path; `None` for untracked files.
fn change_weights(info: &GitInfo) -> HashMap<&Path, Option<usize>> {
    let mut weights: HashMap<&Path, Option<usize>> = HashMap::new();
    for change in info
        .working_changes
        .iter()
        .chain(info.branch_changes.iter())
    {
        let weight = change
            .added
            .zip(change.deleted)
            .map(|(added, deleted)| added + deleted);
        let entry = weights.entry(Path::new(&change.path)).or_insert(weight);
        *entry = match (*entry, weight) {
            (Some(left), Some(right)) => Some(left + right),
            (left, right) => left.or(right),
        };
    }
    weights
}

fn size_score(size: u64) -> usize {
    // Logarithmic: a 40 KB module beats a 4 KB one, but not by 10x.
    let kb = (size as f64 / 1024.0).max(0.25);
    ((kb.log2() + 3.0).max(0.0) * 40.0) as usize
}

fn churn_map(info: &GitInfo) -> HashMap<&Path, usize> {
    info.churn
        .iter()
        .map(|(path, count)| (path.as_path(), *count))
        .collect()
}

/// Directories that hold the project's own code: `src/` next to root
/// manifests, a package directory named after the project, or workspace
/// members that are not examples or fixtures.
fn main_package_dirs(index: &RepoIndex, manifests: &[Manifest]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Some(min_depth) = manifests
        .iter()
        .map(|manifest| paths::depth(&manifest.path))
        .min()
    else {
        return dirs;
    };
    for manifest in manifests {
        if paths::role(&manifest.path).is_secondary() {
            continue;
        }
        let dir = manifest.dir();
        for (entry, _) in &manifest.entry_points {
            if let Some(parent) = entry.parent() {
                // `crates/core/main.rs` declared by the root manifest makes `crates/core` core code.
                let parent = if paths::file_name(parent) == "src" {
                    parent.parent().unwrap_or(parent)
                } else {
                    parent
                };
                dirs.push(parent.to_path_buf());
            }
        }
        if paths::depth(&manifest.path) == min_depth {
            for candidate in ["src", "lib", "app", "internal", "pkg", "cmd"] {
                dirs.push(dir.join(candidate));
            }
            if let Some(name) = &manifest.name {
                let short = name.rsplit('/').next().unwrap_or(name);
                for variant in [short.to_string(), short.replace('-', "_")] {
                    dirs.push(dir.join(&variant));
                    dirs.push(dir.join("src").join(&variant));
                }
            }
        } else {
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.retain(|dir| !dir.as_os_str().is_empty() && index.files_under(dir) > 0);
    dirs.sort();
    dirs.dedup();
    dirs
}

/// Local modules referenced from entry points (`mod x;`, relative imports, ...),
/// mapped to the rank of the first entry point that references them.
fn referenced_from_entry_points(
    index: &RepoIndex,
    entry_points: &[FileRef],
) -> HashMap<PathBuf, usize> {
    let mut referenced = HashMap::new();
    for (rank, entry) in entry_points.iter().enumerate() {
        let path = PathBuf::from(&entry.path);
        let Some(text) = index.read(&path) else {
            continue;
        };
        for candidate in local_references(&path, &text) {
            if index.contains(&candidate) {
                referenced.entry(candidate).or_insert(rank);
            }
        }
    }
    referenced
}

fn local_references(path: &Path, text: &str) -> Vec<PathBuf> {
    let dir = path.parent().unwrap_or_else(|| Path::new(""));
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let mut references = Vec::new();

    for line in text.lines().map(str::trim).take(400) {
        match ext {
            "rs" => {
                let module = line
                    .strip_prefix("pub mod ")
                    .or_else(|| line.strip_prefix("pub(crate) mod "))
                    .or_else(|| line.strip_prefix("mod "))
                    .and_then(|rest| rest.strip_suffix(';'));
                if let Some(module) = module {
                    references.push(dir.join(format!("{module}.rs")));
                    references.push(dir.join(module).join("mod.rs"));
                }
            }
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
                let target = line
                    .split(" from ")
                    .nth(1)
                    .or_else(|| line.split("require(").nth(1))
                    .or_else(|| line.strip_prefix("import "))
                    .map(|rest| rest.trim_matches(|ch: char| "'\";) ".contains(ch)));
                if let Some(target) = target.filter(|target| target.starts_with('.')) {
                    let base = crate::manifest::normalize(&dir.join(target));
                    for suffix in ["ts", "tsx", "js", "jsx", "mjs"] {
                        references.push(base.with_extension(suffix));
                        references.push(base.join(format!("index.{suffix}")));
                    }
                    references.push(base);
                }
            }
            "py" => {
                if let Some(rest) = line.strip_prefix("from .") {
                    let module = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .trim_start_matches('.');
                    if !module.is_empty() {
                        let relative = module.replace('.', "/");
                        references.push(dir.join(format!("{relative}.py")));
                        references.push(dir.join(&relative).join("__init__.py"));
                    }
                }
            }
            _ => {}
        }
    }
    references
}

// ---------------------------------------------------------------------------
// Supporting docs and configuration

fn docs(index: &RepoIndex) -> Vec<FileRef> {
    // A handful of top-level docs pages are guidance; a docs site is content.
    let small_docs_tree = ["docs", "doc"]
        .iter()
        .map(|dir| index.files_under(Path::new(dir)))
        .max()
        .unwrap_or(0)
        <= 15;
    let mut found = index
        .files
        .iter()
        .filter_map(|entry| {
            let (reason, score) = doc_reason(&entry.path, small_docs_tree)?;
            Some((entry.path.clone(), reason, score))
        })
        .collect::<Vec<_>>();
    found.sort_by_key(|(path, _, score)| (Reverse(*score), paths::depth(path), path.clone()));
    // Translated or versioned doc trees repeat the same file names.
    let mut names = HashSet::new();
    found.retain(|(path, _, _)| names.insert(paths::file_name(path).to_ascii_lowercase()));
    found
        .into_iter()
        .take(6)
        .map(|(path, reason, _)| FileRef {
            lines: line_count(index, &path),
            path: paths::display(&path),
            reason: reason.to_string(),
        })
        .collect()
}

fn doc_reason(path: &Path, small_docs_tree: bool) -> Option<(&'static str, usize)> {
    let name = paths::file_name(path);
    let upper = name.to_ascii_uppercase();
    let depth = paths::depth(path);
    let role = paths::role(path);
    if matches!(
        role,
        PathRole::Examples | PathRole::Fixtures | PathRole::Vendor | PathRole::Tests
    ) || role == PathRole::Hidden
        || role == PathRole::Ci && !upper.starts_with("CONTRIBUTING")
    {
        return None;
    }
    // Inside a docs tree only the top level is guidance; deeper pages are content.
    if role == PathRole::Docs && depth > 1 {
        return None;
    }
    let (reason, score): (&str, usize) = match upper.as_str() {
        "README.MD" | "README" | "README.RST" | "README.TXT" if depth == 0 => {
            ("project overview", 100)
        }
        "LLMS.TXT" if depth == 0 => ("AI-facing summary", 95),
        "ARCHITECTURE.MD" | "DESIGN.MD" => ("architecture", 90),
        "CONTRIBUTING.MD" | "HACKING.MD" | "DEVELOPMENT.MD" | "DEVELOPING.MD" => {
            ("contributor workflow", 85)
        }
        "TESTING.MD" => ("testing guide", 80),
        "RUNBOOK.MD" | "OPERATIONS.MD" | "TROUBLESHOOTING.MD" => ("operations", 70),
        "SECURITY.MD" if depth == 0 => ("security policy", 30),
        _ if upper.ends_with("_GUIDE.MD") || upper.ends_with("-GUIDE.MD") => ("guide", 60),
        "README.MD" if depth <= 2 && role == PathRole::Source => ("module overview", 40),
        _ if role == PathRole::Docs
            && depth <= 1
            && upper.ends_with(".MD")
            && !upper.starts_with("CHANGELOG")
            && small_docs_tree =>
        {
            ("documentation", 20)
        }
        _ => return None,
    };
    // Guidance next to the code beats deep documentation trees.
    Some((reason, score.saturating_sub(depth.saturating_sub(1) * 10)))
}

fn config(index: &RepoIndex) -> Vec<FileRef> {
    let mut found = Vec::new();
    for entry in &index.files {
        if paths::depth(&entry.path) > 1
            || paths::role(&entry.path) != PathRole::Source && paths::depth(&entry.path) > 0
        {
            continue;
        }
        let name = paths::file_name(&entry.path);
        let reason = match name {
            ".env.example" | ".env.sample" | ".env.template" | "example.env" => {
                "environment variables template".to_string()
            }
            "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => {
                compose_services(index, &entry.path)
            }
            "Dockerfile" => "container build".to_string(),
            "turbo.json" => "Turborepo pipeline".to_string(),
            "nx.json" => "Nx workspace".to_string(),
            "pnpm-workspace.yaml" => "pnpm workspace".to_string(),
            "tsconfig.json" if paths::depth(&entry.path) == 0 => "TypeScript config".to_string(),
            "rust-toolchain.toml" | "rust-toolchain" => "pinned Rust toolchain".to_string(),
            ".tool-versions" | ".nvmrc" | ".python-version" | "mise.toml" => {
                "pinned tool versions".to_string()
            }
            "flake.nix" => "Nix dev environment".to_string(),
            "devcontainer.json" => "dev container".to_string(),
            _ => continue,
        };
        found.push(FileRef {
            path: paths::display(&entry.path),
            reason,
            lines: None,
        });
    }
    if let Some(entry) = index
        .files
        .iter()
        .find(|entry| paths::display(&entry.path) == ".devcontainer/devcontainer.json")
    {
        found.push(FileRef {
            path: paths::display(&entry.path),
            reason: "dev container".to_string(),
            lines: None,
        });
    }
    found.truncate(8);
    found
}

fn compose_services(index: &RepoIndex, path: &Path) -> String {
    let Some(text) = index.read(path) else {
        return "compose stack".to_string();
    };
    let mut services = Vec::new();
    let mut in_services = false;
    let mut service_indent = None;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_services = line.starts_with("services:");
            continue;
        }
        if in_services {
            let level = *service_indent.get_or_insert(indent);
            if indent == level {
                if let Some(name) = line.trim().strip_suffix(':') {
                    services.push(name.to_string());
                }
            }
        }
    }
    if services.is_empty() {
        "compose stack".to_string()
    } else {
        let shown = services
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = services.len().saturating_sub(8);
        if more > 0 {
            format!("compose services: {shown} (+{more})")
        } else {
            format!("compose services: {shown}")
        }
    }
}

pub fn line_count(index: &RepoIndex, path: &Path) -> Option<usize> {
    index.read(path).map(|text| text.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_files_cover_major_agents() {
        for path in [
            "AGENTS.md",
            "CLAUDE.md",
            "GEMINI.md",
            ".cursorrules",
            ".cursor/rules/style.mdc",
            ".github/copilot-instructions.md",
            "packages/api/AGENTS.md",
        ] {
            assert!(instruction_reason(Path::new(path)).is_some(), "{path}");
        }
        assert!(instruction_reason(Path::new("docs/agents.md")).is_none());
    }

    #[test]
    fn rust_mod_declarations_resolve_next_to_the_entry() {
        let references = local_references(Path::new("src/main.rs"), "mod cli;\npub mod select;\n");
        assert!(references.contains(&PathBuf::from("src/cli.rs")));
        assert!(references.contains(&PathBuf::from("src/select/mod.rs")));
    }

    #[test]
    fn relative_js_imports_are_normalized() {
        let references = local_references(
            Path::new("src/index.ts"),
            "import { a } from './core/app';\n",
        );
        assert!(references.contains(&PathBuf::from("src/core/app.ts")));
    }
}
