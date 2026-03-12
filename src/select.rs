use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, ImportantFile, SelectionResult, SignalCategory};

const EXCLUDED_FILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "poetry.lock",
    "Pipfile.lock",
    "composer.lock",
    "Gemfile.lock",
    "CONTEXT_PACK_PLAN.md",
];

pub fn is_relevant_change_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    if should_skip_file(path, file_name) {
        return false;
    }

    classify(file_name, path, true).is_some()
}

pub fn select_files(
    config: &AppConfig,
    changed_files: &[PathBuf],
    excerpt_budget: usize,
) -> SelectionResult {
    let matcher = IgnoreMatcher::load(&config.cwd, config);
    let mut candidates = Vec::new();
    let mut stats = SelectionStats::new(config.max_files.saturating_mul(200).max(400));
    collect_candidates(
        &config.cwd,
        Path::new(""),
        &matcher,
        changed_files,
        config,
        &mut candidates,
        &mut stats,
    );

    candidates.sort_by_key(|candidate| {
        (
            Reverse(candidate.score),
            candidate.depth,
            candidate.path.clone(),
        )
    });

    let shortlist_len = config.max_files.clamp(1, 8);
    let mut files = Vec::new();
    let mut remaining = excerpt_budget.max(320);
    let mut notes = Vec::new();

    for candidate in candidates.into_iter().take(shortlist_len) {
        if remaining < 120 {
            break;
        }

        let budget = per_file_budget(remaining, shortlist_len.saturating_sub(files.len()));
        let Some(file) = read_important_file(&config.cwd, &candidate, budget) else {
            continue;
        };

        remaining = remaining.saturating_sub(file.excerpt.len());
        files.push(file);
    }

    if files.is_empty() {
        notes.push("no important files selected".to_string());
    } else {
        notes.push(format!("selected files: {}", files.len()));
    }
    notes.extend(stats.render_notes());

    SelectionResult { files, notes }
}

#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    category: SignalCategory,
    score: usize,
    reason: String,
    depth: usize,
}

fn collect_candidates(
    absolute_dir: &Path,
    relative_dir: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    config: &AppConfig,
    candidates: &mut Vec<Candidate>,
    stats: &mut SelectionStats,
) {
    if stats.scan_limit_reached() {
        stats.scan_omissions += 1;
        return;
    }

    let Ok(entries) = fs::read_dir(absolute_dir) else {
        return;
    };

    let mut children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    children.sort();

    for child in children {
        let Ok(metadata) = fs::symlink_metadata(&child) else {
            continue;
        };

        let name = child
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let relative_path = relative_dir.join(&name);
        let is_dir = metadata.is_dir();

        if matcher.is_ignored(&relative_path, is_dir) {
            continue;
        }

        if is_dir {
            collect_candidates(
                &child,
                &relative_path,
                matcher,
                changed_files,
                config,
                candidates,
                stats,
            );
            continue;
        }

        if stats.scan_limit_reached() {
            stats.scan_omissions += 1;
            return;
        }

        stats.visited_files += 1;
        let Some(candidate) = score_candidate(
            &relative_path,
            metadata.len() as usize,
            changed_files,
            config.changed_only,
        ) else {
            continue;
        };

        candidates.push(candidate);
    }
}

fn score_candidate(
    path: &Path,
    byte_len: usize,
    changed_files: &[PathBuf],
    changed_only: bool,
) -> Option<Candidate> {
    let file_name = path.file_name()?.to_str()?;
    if should_skip_file(path, file_name) {
        return None;
    }

    let changed = changed_files.iter().any(|candidate| candidate == path);
    let depth = path.components().count().saturating_sub(1);
    let (category, mut score, mut reasons) = classify(file_name, path, changed)?;

    if depth == 0 {
        score += 40;
    } else if depth == 1 {
        score += 15;
    }

    if byte_len <= 8 * 1024 {
        score += 20;
    }

    if changed_only
        && !changed
        && !matches!(
            category,
            SignalCategory::Instructions | SignalCategory::Overview | SignalCategory::Manifest
        )
    {
        return None;
    }

    if score < 120 {
        return None;
    }

    Some(Candidate {
        path: path.to_path_buf(),
        category,
        score,
        reason: summarize_reasons(&mut reasons),
        depth,
    })
}

fn classify(
    file_name: &str,
    path: &Path,
    changed: bool,
) -> Option<(SignalCategory, usize, Vec<String>)> {
    let mut reasons = Vec::new();
    let category = if file_name == "AGENTS.md" {
        reasons.push("agent instructions".to_string());
        SignalCategory::Instructions
    } else if file_name == "README.md" || file_name == "README" {
        reasons.push("project overview".to_string());
        SignalCategory::Overview
    } else if is_manifest(file_name) {
        reasons.push("project manifest".to_string());
        SignalCategory::Manifest
    } else if file_name == "Makefile" || file_name == "docker-compose.yml" {
        reasons.push("build or orchestration entrypoint".to_string());
        SignalCategory::Build
    } else if file_name == ".env.example" {
        reasons.push("environment template".to_string());
        SignalCategory::Config
    } else if is_supporting_doc(file_name) {
        reasons.push("supporting documentation".to_string());
        SignalCategory::SupportingDoc
    } else if changed && is_source_file(path) {
        reasons.push("changed source file".to_string());
        SignalCategory::ChangedSource
    } else if is_entrypoint_file(file_name) {
        reasons.push("entrypoint-like source file".to_string());
        SignalCategory::EntryPoint
    } else {
        return None;
    };

    let mut score = match category {
        SignalCategory::Instructions => 1000,
        SignalCategory::Overview => 900,
        SignalCategory::Manifest => 820,
        SignalCategory::Build => 760,
        SignalCategory::ChangedSource => 740,
        SignalCategory::EntryPoint => 700,
        SignalCategory::Config => 660,
        SignalCategory::SupportingDoc => 520,
    };

    if changed {
        score += if is_source_file(path) { 90 } else { 35 };
        reasons.push("active work".to_string());
    }

    if is_entrypoint_file(file_name) && !matches!(category, SignalCategory::EntryPoint) {
        score += 30;
        reasons.push("likely entry point".to_string());
    }

    Some((category, score, reasons))
}

fn read_important_file(root: &Path, candidate: &Candidate, budget: usize) -> Option<ImportantFile> {
    let bytes = fs::read(root.join(&candidate.path)).ok()?;
    if bytes.contains(&0) {
        return None;
    }

    let text = String::from_utf8_lossy(&bytes);
    let (excerpt, truncated) = extract_excerpt(
        candidate.path.file_name()?.to_str()?,
        candidate.category,
        &text,
        budget,
    );

    Some(ImportantFile {
        path: candidate.path.clone(),
        reason: candidate.reason.clone(),
        category: candidate.category,
        score: candidate.score,
        excerpt,
        truncated,
    })
}

fn extract_excerpt(
    file_name: &str,
    category: SignalCategory,
    text: &str,
    budget: usize,
) -> (String, bool) {
    let cleaned = compact_text(text);
    let excerpt = match category {
        SignalCategory::Instructions | SignalCategory::Overview | SignalCategory::SupportingDoc => {
            excerpt_sections(&cleaned, budget)
        }
        SignalCategory::Manifest | SignalCategory::Config => excerpt_manifest(&cleaned, budget),
        SignalCategory::Build if file_name == "Makefile" => excerpt_makefile(&cleaned, budget),
        SignalCategory::Build => excerpt_leading_block(&cleaned, budget, 18),
        SignalCategory::ChangedSource | SignalCategory::EntryPoint => {
            excerpt_source(&cleaned, budget)
        }
    };

    let truncated = excerpt != cleaned;
    if truncated {
        (format!("{}\n... [truncated]", excerpt.trim_end()), true)
    } else {
        (excerpt, false)
    }
}

fn excerpt_sections(text: &str, budget: usize) -> String {
    excerpt_by_lines(text, budget, 22, |line, lines| {
        if lines.is_empty() {
            true
        } else {
            !line.starts_with("## ") || lines.len() < 14
        }
    })
}

fn excerpt_manifest(text: &str, budget: usize) -> String {
    excerpt_by_lines(text, budget, 20, |line, _| {
        let trimmed = line.trim_start();
        trimmed.starts_with('[')
            || trimmed.starts_with('{')
            || trimmed.starts_with('}')
            || trimmed.starts_with('"')
            || trimmed.contains('=')
            || trimmed.starts_with("name")
            || trimmed.starts_with("version")
            || trimmed.starts_with("package")
            || trimmed.starts_with("dependencies")
            || trimmed.starts_with("scripts")
    })
}

fn excerpt_makefile(text: &str, budget: usize) -> String {
    excerpt_by_lines(text, budget, 16, |line, lines| {
        let trimmed = line.trim();
        trimmed.starts_with(".PHONY")
            || (!trimmed.starts_with('\t')
                && trimmed.contains(':')
                && !trimmed.starts_with('#')
                && lines.len() < 10)
    })
}

fn excerpt_source(text: &str, budget: usize) -> String {
    excerpt_leading_block(text, budget, 18)
}

fn excerpt_leading_block(text: &str, budget: usize, max_lines: usize) -> String {
    excerpt_by_lines(text, budget, max_lines, |_, _| true)
}

fn excerpt_by_lines<F>(text: &str, budget: usize, max_lines: usize, include: F) -> String
where
    F: Fn(&str, &[String]) -> bool,
{
    if text.len() <= budget {
        return text.to_string();
    }

    let mut lines = Vec::new();
    let mut used = 0usize;

    for line in text.lines() {
        if !include(line, &lines) {
            continue;
        }

        let next = if lines.is_empty() {
            line.len()
        } else {
            line.len() + 1
        };
        if used + next > budget || lines.len() >= max_lines {
            break;
        }

        lines.push(line.to_string());
        used += next;
    }

    if lines.is_empty() {
        return excerpt_leading_block(text, budget, max_lines.min(8));
    }

    lines.join("\n")
}

fn per_file_budget(remaining: usize, remaining_slots: usize) -> usize {
    let slots = remaining_slots.max(1);
    let fair_share = remaining / slots;
    fair_share.clamp(180, 1200).min(remaining.max(1))
}

fn should_skip_file(path: &Path, file_name: &str) -> bool {
    if EXCLUDED_FILES.contains(&file_name) {
        return true;
    }

    if file_name.starts_with('.') && file_name != ".env.example" {
        return true;
    }

    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".min.js") || lower.ends_with(".min.css") {
        return true;
    }

    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        value == "target" || value == "dist" || value == "build"
    })
}

fn is_manifest(file_name: &str) -> bool {
    matches!(
        file_name,
        "package.json"
            | "pyproject.toml"
            | "Cargo.toml"
            | "go.mod"
            | "requirements.txt"
            | "pom.xml"
            | "build.gradle"
    )
}

fn is_supporting_doc(file_name: &str) -> bool {
    matches!(file_name, "ARCHITECTURE.md" | "CONTRIBUTING.md")
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx")
    )
}

fn is_entrypoint_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "main.rs"
            | "lib.rs"
            | "main.go"
            | "main.py"
            | "app.py"
            | "manage.py"
            | "index.ts"
            | "index.tsx"
            | "main.ts"
            | "main.tsx"
            | "App.tsx"
    )
}

fn summarize_reasons(reasons: &mut Vec<String>) -> String {
    reasons.dedup();
    reasons.join(", ")
}

fn compact_text(text: &str) -> String {
    let mut lines = Vec::new();
    let mut blank_run = 0usize;

    for line in text.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
            lines.push(String::new());
            continue;
        }

        blank_run = 0;
        lines.push(trimmed_end.to_string());
    }

    lines.join("\n")
}

struct SelectionStats {
    visited_files: usize,
    scan_limit: usize,
    scan_omissions: usize,
}

impl SelectionStats {
    fn new(scan_limit: usize) -> Self {
        Self {
            visited_files: 0,
            scan_limit,
            scan_omissions: 0,
        }
    }

    fn scan_limit_reached(&self) -> bool {
        self.visited_files >= self.scan_limit
    }

    fn render_notes(&self) -> Vec<String> {
        let mut notes = vec![format!(
            "files scanned for selection: {}",
            self.visited_files
        )];

        if self.scan_omissions > 0 {
            notes.push(format!("selection scan limit reached: {}", self.scan_limit));
        }

        notes
    }
}
