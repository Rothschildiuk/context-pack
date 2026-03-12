use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, ImportantFile, SelectionResult};

const ALWAYS_INCLUDE: &[(&str, usize, &str)] = &[
    ("AGENTS.md", 300, "agent instructions"),
    ("README.md", 240, "project overview"),
    ("README", 220, "project overview"),
    ("package.json", 220, "project manifest"),
    ("pyproject.toml", 220, "project manifest"),
    ("Cargo.toml", 220, "project manifest"),
    ("go.mod", 220, "project manifest"),
    ("requirements.txt", 210, "dependency manifest"),
    ("pom.xml", 210, "project manifest"),
    ("build.gradle", 210, "build definition"),
    ("Makefile", 200, "build entrypoint"),
    ("docker-compose.yml", 180, "runtime topology"),
    (".env.example", 170, "environment template"),
    ("ARCHITECTURE.md", 160, "architecture notes"),
    ("CONTRIBUTING.md", 150, "contribution guide"),
];

pub fn select_files(config: &AppConfig, changed_files: &[PathBuf]) -> SelectionResult {
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

    let mut picked = Vec::new();
    let mut remaining = config.max_bytes.max(256);
    let mut notes = Vec::new();

    for candidate in candidates.into_iter().take(config.max_files) {
        let budget = per_file_budget(remaining, config.max_files.saturating_sub(picked.len()));
        let Some(file) = read_important_file(&config.cwd, &candidate, budget) else {
            continue;
        };

        remaining = remaining.saturating_sub(file.excerpt.len());
        picked.push(file);
    }

    if picked.is_empty() {
        notes.push("no important files selected".to_string());
    } else {
        notes.push(format!("selected files: {}", picked.len()));
    }
    notes.extend(stats.render_notes());

    SelectionResult { files: picked, notes }
}

#[derive(Clone)]
struct Candidate {
    path: PathBuf,
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

    let mut children = entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
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
        let Some(candidate) = score_candidate(&relative_path, metadata.len() as usize, changed_files, config.changed_only) else {
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
    let depth = path.components().count().saturating_sub(1);
    let mut score = 0usize;
    let mut reason = None;

    for (name, points, label) in ALWAYS_INCLUDE {
        if file_name == *name {
            score += points;
            reason = Some((*label).to_string());
            break;
        }
    }

    let changed = changed_files.iter().any(|candidate| candidate == path);
    if changed {
        score += 140;
        if reason.is_none() {
            reason = Some("changed file".to_string());
        }
    }

    if depth == 0 {
        score += 40;
    } else if depth == 1 {
        score += 15;
    }

    if is_source_entrypoint(file_name) {
        score += 40;
        if reason.is_none() {
            reason = Some("entrypoint-like source file".to_string());
        }
    }

    if byte_len <= 8 * 1024 {
        score += 20;
    }

    if changed_only && !changed && score < 150 {
        return None;
    }

    if score < 60 {
        return None;
    }

    Some(Candidate {
        path: path.to_path_buf(),
        score,
        reason: reason.unwrap_or_else(|| "high-signal file".to_string()),
        depth,
    })
}

fn is_source_entrypoint(file_name: &str) -> bool {
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

fn per_file_budget(remaining: usize, remaining_slots: usize) -> usize {
    let slots = remaining_slots.max(1);
    let fair_share = remaining / slots;
    fair_share.clamp(160, 1200).min(remaining.max(1))
}

fn read_important_file(root: &Path, candidate: &Candidate, budget: usize) -> Option<ImportantFile> {
    let bytes = fs::read(root.join(&candidate.path)).ok()?;
    if bytes.contains(&0) {
        return None;
    }

    let text = String::from_utf8_lossy(&bytes);
    let (excerpt, truncated) = trim_excerpt(&text, budget);

    Some(ImportantFile {
        path: candidate.path.clone(),
        reason: candidate.reason.clone(),
        truncated,
        excerpt,
    })
}

fn trim_excerpt(text: &str, budget: usize) -> (String, bool) {
    if text.len() <= budget {
        return (text.to_string(), false);
    }

    let mut end = 0usize;
    for (index, _) in text.char_indices() {
        if index > budget {
            break;
        }
        end = index;
    }

    let snippet = text[..end].trim_end();
    (format!("{snippet}\n... [truncated]"), true)
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
        let mut notes = vec![format!("files scanned for selection: {}", self.visited_files)];

        if self.scan_omissions > 0 {
            notes.push(format!("selection scan limit reached: {}", self.scan_limit));
        }

        notes
    }
}
