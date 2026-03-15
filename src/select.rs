use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, ImportantFile, LargeCodeFile, SelectionResult, SignalCategory};

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

pub struct RepoSignals {
    pub selection: SelectionResult,
    pub large_code_files: Vec<LargeCodeFile>,
}

#[derive(Clone)]
struct LanguageProfile {
    top_languages: Vec<String>,
}

impl LanguageProfile {
    fn rank(&self, language: &str) -> Option<usize> {
        self.top_languages.iter().position(|candidate| candidate == language)
    }
}

pub fn scan_repo_signals(
    config: &AppConfig,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    excerpt_budget: usize,
) -> RepoSignals {
    let language_profile = if config.language_aware {
        detect_language_profile(&config.cwd, matcher, config.changed_only)
    } else {
        LanguageProfile {
            top_languages: Vec::new(),
        }
    };
    let mut candidates = Vec::new();
    let mut large_code_files = Vec::new();
    let mut stats = SelectionStats::new(config.max_files.saturating_mul(200).max(400));

    if should_use_changed_only_fast_path(config, changed_files) {
        collect_changed_only_candidates(
            &config.cwd,
            matcher,
            changed_files,
            config,
            &mut candidates,
            &mut large_code_files,
            &mut stats,
            &language_profile,
        );
    } else {
        collect_candidates(
            &config.cwd,
            Path::new(""),
            matcher,
            changed_files,
            config,
            &mut candidates,
            &mut large_code_files,
            &mut stats,
            &language_profile,
        );
    }

    let mut extra_paths = Vec::new();
    for candidate in &candidates {
        if matches!(candidate.category, SignalCategory::ChangedSource | SignalCategory::EntryPoint) {
            if let Ok(content) = fs::read_to_string(config.cwd.join(&candidate.path)) {
                for dep_path in extract_local_dependencies_as_paths(&content, &candidate.path) {
                    extra_paths.push(dep_path);
                }
            }
        }
    }

    let mut visited_deps = HashSet::new();
    for dep_path in extra_paths {
        if !visited_deps.insert(dep_path.clone()) {
            continue;
        }

        let mut already_in_candidates = false;
        for c in &mut candidates {
            if c.path == dep_path {
                already_in_candidates = true;
                if !c.why.contains(&"referenced by active work or entrypoint".to_string()) {
                    c.score += 80;
                    c.reason = format!("{}, referenced by active work or entrypoint", c.reason);
                    c.why.push("referenced by active work or entrypoint".to_string());
                }
                break;
            }
        }

        if !already_in_candidates {
            let absolute_path = config.cwd.join(&dep_path);
            if absolute_path.is_file() {
                if let Ok(metadata) = fs::metadata(&absolute_path) {
                    process_file(
                        &absolute_path,
                        &dep_path,
                        metadata.len() as usize,
                        changed_files,
                        config.changed_only,
                        true,
                        &mut candidates,
                        &mut large_code_files,
                        &language_profile,
                    );

                    if let Some(c) = candidates.last_mut() {
                        if c.path == dep_path && !c.why.contains(&"referenced by active work or entrypoint".to_string()) {
                            c.score += 80;
                            c.reason = format!("{}, referenced by active work or entrypoint", c.reason);
                            c.why.push("referenced by active work or entrypoint".to_string());
                        }
                    }
                }
            }
        }
    }

    candidates.retain(|candidate| candidate.score >= 120 || candidate.forced || candidate.category == SignalCategory::IncludedSource);

    candidates.sort_by_key(|candidate| {
        (
            Reverse(usize::from(candidate.forced)),
            Reverse(candidate.score),
            candidate.depth,
            candidate.path.clone(),
        )
    });

    let shortlist_len = config.max_files.max(1);
    let shortlisted = candidates
        .into_iter()
        .take(shortlist_len)
        .collect::<Vec<_>>();
    let total_shortlisted = shortlisted.len();
    let mut files = Vec::new();
    let mut remaining = excerpt_budget.max(320);
    let mut notes = Vec::new();

    for candidate in shortlisted {
        if remaining < 120 {
            break;
        }

        let budget = per_file_budget(
            remaining,
            remaining_shortlist_slots(total_shortlisted, files.len()),
        );
        let Some(file) = read_important_file(&config.cwd, &candidate, budget, config.minify) else {
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
    if should_use_changed_only_fast_path(config, changed_files) {
        notes.push("changed-only fast path used".to_string());
    }
    if !language_profile.top_languages.is_empty() {
        notes.push(format!(
            "language-aware scoring: top languages = {}",
            language_profile.top_languages.join(", ")
        ));
    }
    notes.extend(stats.render_notes());

    large_code_files.sort_by_key(|file| {
        (
            Reverse(file.loc),
            Reverse(usize::from(file.reason.contains("changed"))),
            file.path.clone(),
        )
    });
    large_code_files.truncate(5);

    RepoSignals {
        selection: SelectionResult { files, notes },
        large_code_files,
    }
}

#[derive(Clone)]
struct Candidate {
    path: PathBuf,
    category: SignalCategory,
    score: usize,
    reason: String,
    why: Vec<String>,
    depth: usize,
    forced: bool,
}

fn collect_candidates(
    absolute_dir: &Path,
    relative_dir: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    config: &AppConfig,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    stats: &mut SelectionStats,
    language_profile: &LanguageProfile,
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
                large_code_files,
                stats,
                language_profile,
            );
            continue;
        }

        if stats.scan_limit_reached() {
            stats.scan_omissions += 1;
            return;
        }

        stats.visited_files += 1;
        let explicit_include = matcher.is_explicitly_included(&relative_path, false);
        process_file(
            &child,
            &relative_path,
            metadata.len() as usize,
            changed_files,
            config.changed_only,
            explicit_include,
            candidates,
            large_code_files,
            language_profile,
        );
    }
}

fn collect_changed_only_candidates(
    root: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    config: &AppConfig,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    stats: &mut SelectionStats,
    language_profile: &LanguageProfile,
) {
    let mut visited = HashSet::new();
    collect_root_fast_path_files(
        root,
        matcher,
        changed_files,
        config,
        candidates,
        large_code_files,
        stats,
        &mut visited,
        language_profile,
    );

    if !config.include.is_empty() {
        collect_explicit_include_candidates(
            root,
            Path::new(""),
            matcher,
            changed_files,
            config,
            candidates,
            large_code_files,
            stats,
            &mut visited,
            language_profile,
        );
    }

    for relative_path in changed_files {
        process_specific_file(
            root,
            relative_path,
            matcher,
            changed_files,
            config.changed_only,
            candidates,
            large_code_files,
            stats,
            &mut visited,
            language_profile,
        );
    }
}

fn collect_explicit_include_candidates(
    absolute_dir: &Path,
    relative_dir: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    config: &AppConfig,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    stats: &mut SelectionStats,
    visited: &mut HashSet<PathBuf>,
    language_profile: &LanguageProfile,
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
            collect_explicit_include_candidates(
                &child,
                &relative_path,
                matcher,
                changed_files,
                config,
                candidates,
                large_code_files,
                stats,
                visited,
                language_profile,
            );
            continue;
        }

        if stats.scan_limit_reached() {
            stats.scan_omissions += 1;
            return;
        }

        stats.visited_files += 1;
        if !matcher.is_explicitly_included(&relative_path, false) {
            continue;
        }

        process_file(
            &child,
            &relative_path,
            metadata.len() as usize,
            changed_files,
            config.changed_only,
            true,
            candidates,
            large_code_files,
            language_profile,
        );
    }
}

fn collect_root_fast_path_files(
    root: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    config: &AppConfig,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    stats: &mut SelectionStats,
    visited: &mut HashSet<PathBuf>,
    language_profile: &LanguageProfile,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    let mut children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    children.sort();

    for child in children {
        let Some(file_name) = child.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !is_fast_path_root_candidate(file_name) {
            continue;
        }

        process_specific_file(
            root,
            Path::new(file_name),
            matcher,
            changed_files,
            config.changed_only,
            candidates,
            large_code_files,
            stats,
            visited,
            language_profile,
        );
    }
}

fn process_specific_file(
    root: &Path,
    relative_path: &Path,
    matcher: &IgnoreMatcher,
    changed_files: &[PathBuf],
    changed_only: bool,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    stats: &mut SelectionStats,
    visited: &mut HashSet<PathBuf>,
    language_profile: &LanguageProfile,
) {
    if !visited.insert(relative_path.to_path_buf()) {
        return;
    }

    let absolute_path = root.join(relative_path);
    let Ok(metadata) = fs::symlink_metadata(&absolute_path) else {
        return;
    };
    if metadata.is_dir() {
        return;
    }
    if matcher.is_ignored(relative_path, false) {
        return;
    }
    if stats.scan_limit_reached() {
        stats.scan_omissions += 1;
        return;
    }

    stats.visited_files += 1;
    let explicit_include = matcher.is_explicitly_included(relative_path, false);
    process_file(
        &absolute_path,
        relative_path,
        metadata.len() as usize,
        changed_files,
        changed_only,
        explicit_include,
        candidates,
        large_code_files,
        language_profile,
    );
}

fn process_file(
    absolute_path: &Path,
    relative_path: &Path,
    byte_len: usize,
    changed_files: &[PathBuf],
    changed_only: bool,
    explicit_include: bool,
    candidates: &mut Vec<Candidate>,
    large_code_files: &mut Vec<LargeCodeFile>,
    language_profile: &LanguageProfile,
) {
    if let Some(candidate) = score_candidate(
        absolute_path,
        relative_path,
        byte_len,
        changed_files,
        changed_only,
        explicit_include,
        language_profile,
    ) {
        candidates.push(candidate);
    }

    if let Some(file) = large_code_file(
        absolute_path,
        relative_path,
        changed_files,
        changed_only,
        explicit_include,
    ) {
        large_code_files.push(file);
    }
}

fn score_candidate(
    absolute_path: &Path,
    path: &Path,
    byte_len: usize,
    changed_files: &[PathBuf],
    changed_only: bool,
    explicit_include: bool,
    language_profile: &LanguageProfile,
) -> Option<Candidate> {
    let file_name = path.file_name()?.to_str()?;
    if should_skip_file(path, file_name)
        && !(explicit_include && sensitive_file_requires_omission(path, file_name))
    {
        return None;
    }

    let changed = changed_files.iter().any(|candidate| candidate == path);
    let depth = path.components().count().saturating_sub(1);
    let (category, mut score, mut reasons) = classify(file_name, path, changed)
        .or_else(|| classify_explicit_include(file_name, path, explicit_include))?;
    let mut why = reasons.clone();

    if matches!(category, SignalCategory::Overview) && is_placeholder_heavy_readme(absolute_path) {
        score = score.saturating_sub(260);
        reasons.push("placeholder-heavy template".to_string());
        why.push("placeholder-heavy template".to_string());
    }

    if depth == 0 {
        score += 40;
        why.push("repo root priority".to_string());
    } else if depth == 1 {
        score += 15;
        why.push("shallow path priority".to_string());
    }

    if byte_len <= 8 * 1024 {
        score += 20;
        why.push("compact file bonus".to_string());
    }

    if changed_only
        && !changed
        && !explicit_include
        && !matches!(
            category,
            SignalCategory::Instructions | SignalCategory::Overview | SignalCategory::Manifest
        )
    {
        return None;
    }

    if explicit_include {
        score += 25;
        reasons.push("explicit include".to_string());
        why.push("explicit include".to_string());
    }

    if let Some((bonus, note)) = language_score_bonus(path, file_name, category, language_profile) {
        score += bonus;
        reasons.push(note.clone());
        why.push(note);
    }

    Some(Candidate {
        path: path.to_path_buf(),
        category,
        score,
        reason: summarize_reasons(&mut reasons),
        why: dedupe_reasons(why),
        depth,
        forced: explicit_include,
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
    } else if let Some(reason) = agent_instruction_reason(file_name, path) {
        reasons.push(reason.to_string());
        SignalCategory::Instructions
    } else if let Some(reason) = repo_memory_reason(file_name, path) {
        reasons.push(reason.to_string());
        SignalCategory::Instructions
    } else if is_llms_file(path, file_name) {
        reasons.push("AI-facing repo summary".to_string());
        SignalCategory::Overview
    } else if is_root_readme(path, file_name) {
        reasons.push("project overview".to_string());
        SignalCategory::Overview
    } else if is_nested_readme(path, file_name) {
        reasons.push("module overview".to_string());
        SignalCategory::SupportingDoc
    } else if is_manifest(file_name) {
        reasons.push("project manifest".to_string());
        SignalCategory::Manifest
    } else if is_build_file(file_name) {
        reasons.push("build or orchestration entrypoint".to_string());
        SignalCategory::Build
    } else if file_name == ".env.example" {
        reasons.push("environment template".to_string());
        SignalCategory::Config
    } else if let Some(reason) = shared_ide_config_reason(file_name, path) {
        reasons.push(reason.to_string());
        SignalCategory::Config
    } else if let Some(reason) = supporting_doc_reason(file_name, path) {
        reasons.push(reason.to_string());
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
        SignalCategory::IncludedSource => 720,
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

    if matches!(category, SignalCategory::SupportingDoc) {
        score += supporting_doc_bonus(file_name, path);
    }

    Some((category, score, reasons))
}

fn classify_explicit_include(
    file_name: &str,
    path: &Path,
    explicit_include: bool,
) -> Option<(SignalCategory, usize, Vec<String>)> {
    if !explicit_include {
        return None;
    }

    if is_source_file(path) {
        return Some((
            SignalCategory::IncludedSource,
            680,
            vec!["explicitly included source file".to_string()],
        ));
    }

    if is_document_file(path) {
        if let Some(reason) = agent_instruction_reason(file_name, path) {
            return Some((
                SignalCategory::Instructions,
                980,
                vec![format!("explicitly included {reason}")],
            ));
        }

        if let Some(reason) = repo_memory_reason(file_name, path) {
            return Some((
                SignalCategory::Instructions,
                980,
                vec![format!("explicitly included {reason}")],
            ));
        }

        return Some((
            SignalCategory::SupportingDoc,
            560,
            vec!["explicitly included document".to_string()],
        ));
    }

    if sensitive_file_requires_omission(path, file_name) {
        return Some((
            SignalCategory::Config,
            660,
            vec!["explicitly included sensitive config".to_string()],
        ));
    }

    if file_name == ".env.example" {
        return Some((
            SignalCategory::Config,
            660,
            vec!["explicitly included config".to_string()],
        ));
    }

    if shared_ide_config_reason(file_name, path).is_some() {
        return Some((
            SignalCategory::Config,
            660,
            vec!["explicitly included config".to_string()],
        ));
    }

    None
}

fn read_important_file(root: &Path, candidate: &Candidate, budget: usize, minify: bool) -> Option<ImportantFile> {
    let bytes = fs::read(root.join(&candidate.path)).ok()?;
    if bytes.contains(&0) {
        return None;
    }

    let text = String::from_utf8_lossy(&bytes);
    let redaction = sanitize_excerpt_text(&candidate.path, &text);
    let (excerpt, truncated) = extract_excerpt(
        &candidate.path,
        candidate.category,
        &redaction.text,
        budget,
        minify,
    );

    Some(ImportantFile {
        path: candidate.path.clone(),
        reason: candidate.reason.clone(),
        why: candidate.why.clone(),
        category: candidate.category,
        score: candidate.score,
        excerpt,
        truncated,
        redacted: redaction.redacted,
        redaction_reason: redaction.reason,
    })
}

struct SanitizedExcerpt {
    text: String,
    redacted: bool,
    reason: Option<String>,
}

fn sanitize_excerpt_text(path: &Path, text: &str) -> SanitizedExcerpt {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if sensitive_file_requires_omission(path, file_name) {
        return SanitizedExcerpt {
            text: "[content omitted: sensitive file type]".to_string(),
            redacted: true,
            reason: Some("sensitive file type".to_string()),
        };
    }

    let sanitized = sanitize_sensitive_lines(text);
    if sanitized != text {
        SanitizedExcerpt {
            text: sanitized,
            redacted: true,
            reason: Some("potential secrets redacted".to_string()),
        }
    } else {
        SanitizedExcerpt {
            text: text.to_string(),
            redacted: false,
            reason: None,
        }
    }
}

fn extract_excerpt(
    path: &Path,
    category: SignalCategory,
    text: &str,
    budget: usize,
    minify: bool,
) -> (String, bool) {
    let minify_excerpt = minify
        && matches!(
            category,
            SignalCategory::ChangedSource
                | SignalCategory::IncludedSource
                | SignalCategory::EntryPoint
        );
    let cleaned = compact_text(text, minify_excerpt, path);
    let file_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    let excerpt = match category {
        SignalCategory::Instructions | SignalCategory::Overview | SignalCategory::SupportingDoc => {
            excerpt_sections(&cleaned, budget)
        }
        SignalCategory::Manifest | SignalCategory::Config => excerpt_manifest(&cleaned, budget),
        SignalCategory::Build if file_name == "Makefile" => excerpt_makefile(&cleaned, budget),
        SignalCategory::Build => excerpt_leading_block(&cleaned, budget, 18),
        SignalCategory::ChangedSource
        | SignalCategory::IncludedSource
        | SignalCategory::EntryPoint => excerpt_source(&cleaned, budget),
    };

    let truncated = excerpt != cleaned;
    if truncated {
        (format!("{}\n... [truncated]", excerpt.trim_end()), true)
    } else {
        (excerpt, false)
    }
}

fn large_code_file(
    absolute_path: &Path,
    relative_path: &Path,
    changed_files: &[PathBuf],
    changed_only: bool,
    explicit_include: bool,
) -> Option<LargeCodeFile> {
    if !is_source_file(relative_path) || !is_production_like_source(relative_path) {
        return None;
    }

    let content = fs::read_to_string(absolute_path).ok()?;
    let loc = count_code_lines(&content);
    if loc < 20 {
        return None;
    }

    let changed = changed_files
        .iter()
        .any(|candidate| candidate == relative_path);
    if changed_only && !changed && !explicit_include {
        return None;
    }
    let entrypoint = relative_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(is_entrypoint_file)
        .unwrap_or(false);

    let reason = if explicit_include && changed {
        "large explicitly included changed source file".to_string()
    } else if explicit_include {
        "large explicitly included source file".to_string()
    } else if changed && entrypoint {
        "large changed entrypoint".to_string()
    } else if changed {
        "large changed source file".to_string()
    } else if entrypoint {
        "large entrypoint-like source file".to_string()
    } else {
        "large production source file".to_string()
    };

    Some(LargeCodeFile {
        path: relative_path.to_path_buf(),
        loc,
        reason,
    })
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
    if text.len() <= budget {
        return text.to_string();
    }

    let all_lines = text.lines().collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut used = 0usize;
    let mut target_blocks = 0usize;
    let mut index = 0usize;

    while index < all_lines.len() {
        let line = all_lines[index];
        let trimmed = line.trim();

        if trimmed.starts_with(".PHONY") {
            if !append_excerpt_line(&mut lines, &mut used, line, budget) {
                break;
            }
            index += 1;
            continue;
        }

        if !is_make_target(line) {
            index += 1;
            continue;
        }

        if target_blocks >= 6 || !append_excerpt_line(&mut lines, &mut used, line, budget) {
            break;
        }

        target_blocks += 1;
        index += 1;

        let mut recipe_lines = 0usize;
        while index < all_lines.len() {
            let next = all_lines[index];
            let next_trimmed = next.trim();

            if is_make_target(next) {
                break;
            }

            if next.starts_with('\t') {
                if !append_excerpt_line(&mut lines, &mut used, next, budget) {
                    index = all_lines.len();
                    break;
                }

                recipe_lines += 1;
                if recipe_lines >= 2 {
                    index += 1;
                    while index < all_lines.len() && !is_make_target(all_lines[index]) {
                        index += 1;
                    }
                    break;
                }
            } else if next_trimmed.is_empty() && recipe_lines > 0 {
                break;
            }

            index += 1;
        }
    }

    if lines.is_empty() {
        return excerpt_leading_block(text, budget, 12);
    }

    lines.join("\n")
}

fn excerpt_source(text: &str, budget: usize) -> String {
    excerpt_structured_source(text, budget)
        .unwrap_or_else(|| excerpt_leading_block(text, budget, 18))
}

#[derive(Clone, Copy)]
struct SourceBlock {
    start: usize,
    end: usize,
    priority: usize,
}

fn excerpt_structured_source(text: &str, budget: usize) -> Option<String> {
    if text.len() <= budget {
        return Some(text.to_string());
    }

    let lines = text.lines().collect::<Vec<_>>();
    let blocks = collect_source_blocks(&lines);
    if blocks.is_empty() {
        return None;
    }

    let mut selected = select_source_blocks(&lines, blocks, budget);
    if selected.is_empty() {
        return None;
    }

    selected.sort_by_key(|block| block.start);
    render_source_blocks(&lines, &selected, budget)
}

fn collect_source_blocks(lines: &[&str]) -> Vec<SourceBlock> {
    let mut blocks = Vec::new();

    for index in 0..lines.len() {
        let Some(priority) = significant_source_priority(lines, index) else {
            continue;
        };

        blocks.push(SourceBlock {
            start: decorator_block_start(lines, index),
            end: source_block_end(lines, index),
            priority,
        });
    }

    blocks
}

fn select_source_blocks(
    lines: &[&str],
    mut blocks: Vec<SourceBlock>,
    budget: usize,
) -> Vec<SourceBlock> {
    blocks.sort_by_key(|block| (Reverse(block.priority), block.start));

    let mut selected = Vec::new();
    let mut used = 0usize;

    for block in blocks {
        if selected
            .iter()
            .any(|existing| source_blocks_overlap(existing, &block))
        {
            continue;
        }

        let block_len = render_source_block_len(lines, &block);
        let separator_len = if selected.is_empty() { 0 } else { 5 };
        if used + separator_len + block_len > budget {
            continue;
        }

        used += separator_len + block_len;
        selected.push(block);

        if selected.len() >= 6 {
            break;
        }
    }

    selected
}

fn render_source_blocks(lines: &[&str], blocks: &[SourceBlock], budget: usize) -> Option<String> {
    let mut output = String::new();

    for block in blocks {
        let snippet = lines[block.start..=block.end].join("\n");
        let separator = if output.is_empty() { "" } else { "\n...\n" };

        if output.len() + separator.len() + snippet.len() > budget {
            break;
        }

        output.push_str(separator);
        output.push_str(&snippet);
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

fn render_source_block_len(lines: &[&str], block: &SourceBlock) -> usize {
    lines[block.start..=block.end]
        .iter()
        .map(|line| line.len())
        .sum::<usize>()
        + block.end.saturating_sub(block.start)
}

fn source_blocks_overlap(left: &SourceBlock, right: &SourceBlock) -> bool {
    left.start <= right.end.saturating_add(1) && right.start <= left.end.saturating_add(1)
}

fn significant_source_priority(lines: &[&str], index: usize) -> Option<usize> {
    let trimmed = lines.get(index)?.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with('@')
        || trimmed == "{"
        || trimmed == "}"
    {
        return None;
    }

    if trimmed.contains("if __name__ ==") {
        return Some(6);
    }

    if is_route_call(trimmed) {
        return Some(6);
    }

    if is_framework_bootstrap_line(trimmed) {
        return Some(5);
    }

    if is_signature_line(trimmed) {
        let mut priority = if trimmed.contains(" main(") || trimmed.starts_with("main(") {
            6
        } else {
            4
        };

        if has_route_decorator(lines, index) {
            priority = priority.max(5);
        }

        return Some(priority);
    }

    if looks_like_arrow_function(trimmed) || looks_like_java_method_signature(trimmed) {
        return Some(4);
    }

    None
}

fn decorator_block_start(lines: &[&str], index: usize) -> usize {
    let mut start = index;

    while start > 0 {
        let previous = lines[start - 1].trim();
        if previous.starts_with('@') {
            start -= 1;
            continue;
        }
        break;
    }

    start
}

fn source_block_end(lines: &[&str], index: usize) -> usize {
    let current_indent = indentation(lines[index]);
    let mut end = index;
    let mut cursor = index + 1;
    let mut included = 0usize;

    while cursor < lines.len() && included < 2 {
        let next = lines[cursor];
        let trimmed = next.trim();

        if trimmed.is_empty() {
            if included == 0 {
                cursor += 1;
                continue;
            }
            break;
        }

        let previous = lines[end].trim();
        let next_indent = indentation(next);
        let include = trimmed == "{"
            || previous == "{"
            || previous.ends_with('{')
            || previous.ends_with(':')
            || previous.ends_with("=>")
            || next_indent > current_indent;

        if !include {
            break;
        }

        end = cursor;
        included += 1;
        cursor += 1;
    }

    end
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn has_route_decorator(lines: &[&str], index: usize) -> bool {
    let mut cursor = index;

    while cursor > 0 {
        let previous = lines[cursor - 1].trim();
        if previous.is_empty() {
            break;
        }
        if !previous.starts_with('@') {
            break;
        }
        if is_route_decorator(previous) {
            return true;
        }
        cursor -= 1;
    }

    false
}

fn is_route_decorator(line: &str) -> bool {
    matches_route_target(line.trim_start_matches('@'))
}

fn is_route_call(line: &str) -> bool {
    matches_route_target(line)
}

fn matches_route_target(line: &str) -> bool {
    let trimmed = line.trim();
    let has_route_method = [
        ".get(", ".post(", ".put(", ".patch(", ".delete(", ".route(", ".use(",
    ]
    .iter()
    .any(|needle| trimmed.contains(needle));

    if !has_route_method {
        return false;
    }

    ["app", "router", "bp", "blueprint", "server"]
        .iter()
        .any(|target| trimmed.contains(target))
}

fn is_framework_bootstrap_line(line: &str) -> bool {
    [
        "FastAPI(",
        "APIRouter(",
        "Flask(",
        "Blueprint(",
        "express(",
        "Router(",
        "createServer(",
        "uvicorn.run(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn is_signature_line(line: &str) -> bool {
    let trimmed = line.trim();

    [
        "fn ",
        "pub fn ",
        "async fn ",
        "pub async fn ",
        "def ",
        "async def ",
        "class ",
        "struct ",
        "pub struct ",
        "enum ",
        "pub enum ",
        "trait ",
        "pub trait ",
        "impl ",
        "function ",
        "export function ",
        "export async function ",
        "interface ",
        "record ",
        "public class ",
        "final class ",
        "sealed class ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn looks_like_arrow_function(line: &str) -> bool {
    let trimmed = line.trim();

    (trimmed.starts_with("const ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("var ")
        || trimmed.starts_with("export const "))
        && trimmed.contains('=')
        && trimmed.contains("=>")
}

fn looks_like_java_method_signature(line: &str) -> bool {
    let trimmed = line.trim();

    (trimmed.starts_with("public ")
        || trimmed.starts_with("private ")
        || trimmed.starts_with("protected "))
        && trimmed.contains('(')
        && trimmed.contains(')')
        && !trimmed.contains('=')
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

fn remaining_shortlist_slots(total: usize, selected_so_far: usize) -> usize {
    total.saturating_sub(selected_so_far).max(1)
}

fn should_skip_file(path: &Path, file_name: &str) -> bool {
    if EXCLUDED_FILES.contains(&file_name) {
        return true;
    }

    if has_non_project_context(path) {
        return true;
    }

    if file_name.starts_with('.')
        && file_name != ".env.example"
        && shared_ide_config_reason(file_name, path).is_none()
    {
        return true;
    }

    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".min.js") || lower.ends_with(".min.css") {
        return true;
    }

    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        value == "target" || value == "dist" || value == "build" || is_vendor_like_component(&value)
    })
}

fn has_non_project_context(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        matches!(
            value.as_str(),
            "tests"
                | "test"
                | "__tests__"
                | "fixtures"
                | "fixture"
                | "third_party"
                | "node_modules"
        ) || is_vendor_like_component(&value)
    })
}

fn is_production_like_source(path: &Path) -> bool {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect::<Vec<_>>();

    if components.is_empty() {
        return false;
    }

    if components.iter().any(|component| {
        matches!(
            component.as_str(),
            "tests"
                | "test"
                | "__tests__"
                | "fixtures"
                | "fixture"
                | "third_party"
                | "docs"
                | "doc"
                | "examples"
                | "example"
                | "samples"
                | "sample"
                | "migrations"
                | "node_modules"
        ) || is_vendor_like_component(component)
    }) {
        return false;
    }

    if components.len() == 1 {
        return true;
    }

    matches!(
        components.first().map(String::as_str),
        Some("src" | "app" | "core" | "services" | "service" | "ui" | "lib" | "server" | "client")
    )
}

fn count_code_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
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
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
    )
}

fn is_supporting_doc(file_name: &str) -> bool {
    matches!(
        file_name,
        "ARCHITECTURE.md"
            | "CONTRIBUTING.md"
            | "DATA_SOURCES.md"
            | "DESIGN.md"
            | "OPERATIONS.md"
            | "RUNBOOK.md"
            | "SERIES_GUIDE.md"
            | "TROUBLESHOOTING.md"
    ) || file_name.ends_with("_GUIDE.md")
        || file_name.ends_with("_OVERVIEW.md")
}

fn is_document_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("md" | "mdx" | "txt" | "rst" | "adoc")
    )
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt" | "c" | "h" | "v" | "hs")
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
            | "index.js"
            | "index.jsx"
            | "index.ts"
            | "index.tsx"
            | "main.js"
            | "main.jsx"
            | "main.ts"
            | "main.tsx"
            | "app.js"
            | "app.jsx"
            | "App.tsx"
            | "server.js"
            | "server.ts"
            | "Main.java"
            | "Application.java"
    )
}

fn summarize_reasons(reasons: &mut Vec<String>) -> String {
    reasons.dedup();
    reasons.join(", ")
}

fn dedupe_reasons(mut reasons: Vec<String>) -> Vec<String> {
    reasons.dedup();
    reasons
}

fn language_score_bonus(
    path: &Path,
    file_name: &str,
    category: SignalCategory,
    profile: &LanguageProfile,
) -> Option<(usize, String)> {
    if !matches!(
        category,
        SignalCategory::EntryPoint
            | SignalCategory::ChangedSource
            | SignalCategory::IncludedSource
            | SignalCategory::Build
    ) {
        return None;
    }

    let language = detect_language_for_path(path, file_name)?;
    let rank = profile.rank(language)?;
    let mut bonus = match rank {
        0 => 55,
        1 => 35,
        2 => 20,
        _ => 0,
    };

    if bonus == 0 {
        return None;
    }

    if matches!(category, SignalCategory::EntryPoint | SignalCategory::Build) && rank == 0 {
        bonus += 15;
    }

    Some((
        bonus,
        format!("language-aware boost ({language}, top-{})", rank + 1),
    ))
}

fn detect_language_for_path<'a>(path: &Path, file_name: &'a str) -> Option<&'a str> {
    if matches!(file_name, "Cargo.toml") {
        return Some("rust");
    }
    if matches!(file_name, "pyproject.toml" | "requirements.txt") {
        return Some("python");
    }
    if matches!(file_name, "go.mod") {
        return Some("go");
    }
    if matches!(file_name, "cabal.project" | "stack.yaml" | "package.yaml") {
        return Some("haskell");
    }
    if matches!(
        file_name,
        "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
    ) {
        return Some("java");
    }
    if matches!(file_name, "package.json") {
        return Some("javascript");
    }
    if matches!(file_name, "tsconfig.json") {
        return Some("typescript");
    }

    match path.extension().and_then(|value| value.to_str()) {
        Some("rs") => Some("rust"),
        Some("py") => Some("python"),
        Some("go") => Some("go"),
        Some("java" | "kt") => Some("java"),
        Some("ts" | "tsx") => Some("typescript"),
        Some("js" | "jsx") => Some("javascript"),
        Some("c" | "h") => Some("c"),
        Some("v") => Some("coq"),
        Some("hs") => Some("haskell"),
        _ => None,
    }
}

fn compact_text(text: &str, minify: bool, path: &Path) -> String {
    let mut lines = Vec::new();
    let mut blank_run = 0usize;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let is_c_style = matches!(ext, "js" | "jsx" | "ts" | "tsx" | "rs" | "go" | "java" | "c" | "cpp" | "h" | "hpp");
    let is_python_style = matches!(ext, "py" | "rb" | "sh" | "yaml" | "yml");

    for line in text.lines() {
        let mut trimmed_end = line.trim_end();

        if minify {
            let trimmed = trimmed_end.trim_start();
            if (is_c_style && trimmed.starts_with("//")) || (is_python_style && trimmed.starts_with('#')) {
                continue;
            }
            trimmed_end = trimmed;
        }

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

fn extract_local_dependencies_as_paths(text: &str, relative_path: &Path) -> Vec<PathBuf> {
    let mut deps = Vec::new();
    let ext = relative_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
    
    for line in text.lines() {
        let trimmed = line.trim();
        if ext == "rs" {
            if trimmed.starts_with("mod ") {
                if let Some(name) = trimmed.strip_prefix("mod ").and_then(|s| s.strip_suffix(';')) {
                    deps.push(parent.join(format!("{name}.rs")));
                    deps.push(parent.join(name).join("mod.rs"));
                }
            } else if trimmed.starts_with("use crate::") {
                if let Some(path_str) = trimmed.strip_prefix("use crate::").and_then(|s| s.split("::").next()) {
                    let path_str = path_str.trim_end_matches(';').trim();
                    deps.push(PathBuf::from("src").join(format!("{path_str}.rs")));
                    deps.push(PathBuf::from("src").join(path_str).join("mod.rs"));
                }
            } else if trimmed.starts_with("use super::") {
                if let Some(path_str) = trimmed.strip_prefix("use super::").and_then(|s| s.split("::").next()) {
                    let path_str = path_str.trim_end_matches(';').trim();
                    let file_name = relative_path.file_name().and_then(|name| name.to_str()).unwrap_or("");
                    let super_dir = if file_name == "mod.rs" {
                        parent.parent().unwrap_or(parent)
                    } else {
                        parent
                    };
                    deps.push(super_dir.join(format!("{path_str}.rs")));
                    deps.push(super_dir.join(path_str).join("mod.rs"));
                }
            }
        } else if matches!(ext, "ts" | "tsx" | "js" | "jsx") {
            let try_extract = |path_str: &str| -> Option<String> {
                let unquoted = path_str.trim_matches(|c| c == '\'' || c == '"' || c == ';');
                if unquoted.starts_with('.') {
                    Some(unquoted.to_string())
                } else {
                    None
                }
            };
            
            let mut extracted = None;
            if trimmed.starts_with("import ") && trimmed.contains(" from ") {
                if let Some(last) = trimmed.split(" from ").last() {
                    extracted = try_extract(last);
                }
            } else if trimmed.contains("require(") {
                if let Some(after) = trimmed.split("require(").nth(1) {
                    if let Some(quoted) = after.split(')').next() {
                        extracted = try_extract(quoted);
                    }
                }
            }
            
            if let Some(rel) = extracted {
                for try_ext in ["ts", "tsx", "js", "jsx"] {
                    deps.push(parent.join(format!("{rel}.{try_ext}")));
                    deps.push(parent.join(&rel).join(format!("index.{try_ext}")));
                }
            }
        } else if ext == "py" {
            if trimmed.starts_with("from .") {
                if let Some(module) = trimmed.strip_prefix("from .").and_then(|s| s.split(" import").next()) {
                    deps.push(parent.join(format!("{module}.py")));
                }
            }
        }
    }
    deps
}

fn sensitive_file_requires_omission(path: &Path, file_name: &str) -> bool {
    let lower_name = file_name.to_ascii_lowercase();
    if matches!(
        lower_name.as_str(),
        ".env" | ".npmrc" | ".pypirc" | ".netrc" | "id_rsa" | "id_ed25519"
    ) {
        return true;
    }

    if lower_name.starts_with(".env.")
        && lower_name != ".env.example"
        && lower_name != ".env.sample"
        && lower_name != ".env.template"
    {
        return true;
    }

    if lower_name.ends_with(".pem") || lower_name.ends_with(".key") {
        return true;
    }

    if lower_name.contains("secret")
        || lower_name.contains("token")
        || lower_name.contains("credential")
        || lower_name.contains("private_key")
    {
        return true;
    }

    let lower_path = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect::<Vec<_>>();

    lower_path
        .windows(2)
        .any(|parts| matches!(parts[0].as_str(), ".aws" | "aws") && parts[1] == "credentials")
}

fn sanitize_sensitive_lines(text: &str) -> String {
    let mut changed = false;
    let mut output = Vec::new();

    for line in text.lines() {
        let sanitized = sanitize_sensitive_line(line);
        if sanitized != line {
            changed = true;
        }
        output.push(sanitized);
    }

    if changed {
        output.join("\n")
    } else {
        text.to_string()
    }
}

fn sanitize_sensitive_line(line: &str) -> String {
    if line.trim().is_empty() || line.trim_start().starts_with('#') {
        return line.to_string();
    }

    if let Some(sanitized) = sanitize_assignment_like_line(line, '=') {
        return sanitized;
    }

    if let Some(sanitized) = sanitize_assignment_like_line(line, ':') {
        return sanitized;
    }

    line.to_string()
}

fn sanitize_assignment_like_line(line: &str, delimiter: char) -> Option<String> {
    let comment_trimmed = line.trim_start();
    if comment_trimmed.starts_with('-') && delimiter == ':' && !comment_trimmed.contains(": ") {
        return None;
    }

    let delimiter_index = line.find(delimiter)?;
    let key = &line[..delimiter_index];
    let value = &line[delimiter_index + delimiter.len_utf8()..];
    if !looks_like_secret_key(key) || value.trim().is_empty() {
        return None;
    }

    let redacted_value = preserve_value_wrapper(value);
    Some(format!("{key}{delimiter}{redacted_value}"))
}

fn looks_like_secret_key(key: &str) -> bool {
    let normalized = key
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_start_matches('-')
        .trim()
        .to_ascii_lowercase();

    [
        "key",
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "client_secret",
        "access_token",
        "refresh_token",
        "private_key",
        "credential",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn preserve_value_wrapper(value: &str) -> String {
    let leading_ws_len = value.len() - value.trim_start().len();
    let leading_ws = &value[..leading_ws_len];
    let trimmed = value.trim();

    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        format!("{leading_ws}\"[REDACTED]\"")
    } else if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        format!("{leading_ws}'[REDACTED]'")
    } else {
        format!("{leading_ws}[REDACTED]")
    }
}

fn append_excerpt_line(
    lines: &mut Vec<String>,
    used: &mut usize,
    line: &str,
    budget: usize,
) -> bool {
    let next = if lines.is_empty() {
        line.len()
    } else {
        line.len() + 1
    };

    if *used + next > budget {
        return false;
    }

    lines.push(line.to_string());
    *used += next;
    true
}

fn is_make_target(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || line.starts_with('\t') {
        return false;
    }

    if trimmed.contains(":=")
        || trimmed.contains("?=")
        || trimmed.contains("+=")
        || trimmed.contains("!=")
    {
        return false;
    }

    trimmed.contains(':')
}

fn is_build_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "Makefile"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | "compose.yml"
            | "compose.yaml"
            | "Justfile"
            | "Taskfile.yml"
            | "Taskfile.yaml"
    ) || file_name == "Dockerfile"
        || file_name.starts_with("Dockerfile.")
}

fn shared_ide_config_reason(file_name: &str, path: &Path) -> Option<&'static str> {
    if file_name == ".editorconfig" {
        return Some("shared editor config");
    }

    if is_vscode_shared_config(path, file_name) {
        return Some(match file_name {
            "tasks.json" => "shared VS Code task config",
            "launch.json" => "shared VS Code launch config",
            "extensions.json" => "shared VS Code extension recommendations",
            _ => return None,
        });
    }

    if is_idea_run_config(path, file_name) {
        return Some("shared IntelliJ run config");
    }

    None
}

fn agent_instruction_reason(file_name: &str, path: &Path) -> Option<&'static str> {
    if is_clio_instruction_file(path, file_name) {
        return Some("tool-specific agent instructions");
    }

    None
}

fn repo_memory_reason(file_name: &str, path: &Path) -> Option<&'static str> {
    if is_repo_memory_file(path, file_name) {
        return Some("learned repo memory");
    }

    None
}

fn is_llms_file(path: &Path, file_name: &str) -> bool {
    file_name == "llms.txt" && is_repo_root_file(path)
}

fn is_clio_instruction_file(path: &Path, file_name: &str) -> bool {
    file_name == "instructions.md"
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some(".clio")
}

fn is_repo_memory_file(path: &Path, file_name: &str) -> bool {
    if file_name == "REPO_MEMORY.md" && is_repo_root_file(path) {
        return true;
    }

    file_name == "memory.md"
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some(".context-pack")
}

fn is_vscode_shared_config(path: &Path, file_name: &str) -> bool {
    matches!(file_name, "tasks.json" | "launch.json" | "extensions.json")
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some(".vscode")
}

fn is_idea_run_config(path: &Path, file_name: &str) -> bool {
    file_name.ends_with(".xml")
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some("runConfigurations")
        && path
            .parent()
            .and_then(|parent| parent.parent())
            .and_then(|grandparent| grandparent.file_name())
            .and_then(|value| value.to_str())
            == Some(".idea")
}

fn is_root_readme(path: &Path, file_name: &str) -> bool {
    is_repo_root_file(path) && matches!(file_name, "README.md" | "README")
}

fn is_nested_readme(path: &Path, file_name: &str) -> bool {
    !is_repo_root_file(path) && matches!(file_name, "README.md" | "README")
}

fn is_repo_root_file(path: &Path) -> bool {
    path.components().count() == 1
}

fn supporting_doc_reason(file_name: &str, _path: &Path) -> Option<&'static str> {
    match file_name {
        "ARCHITECTURE.md" | "DESIGN.md" => Some("architecture guide"),
        "DATA_SOURCES.md" => Some("data source guide"),
        "MEMORY.md" => Some("memory system guide"),
        "MULTI_AGENT_COORDINATION.md" => Some("multi-agent coordination guide"),
        "PERFORMANCE.md" => Some("performance guide"),
        "REMOTE_EXECUTION.md" => Some("remote execution guide"),
        "SANDBOX.md" => Some("sandbox guide"),
        "SERIES_GUIDE.md" => Some("domain guide"),
        "OPERATIONS.md" | "RUNBOOK.md" | "TROUBLESHOOTING.md" => Some("operations guide"),
        "CONTRIBUTING.md" => Some("contributor guide"),
        _ if file_name.ends_with("_GUIDE.md") || file_name.ends_with("_OVERVIEW.md") => {
            Some("repo guide")
        }
        _ => None,
    }
}

fn supporting_doc_bonus(file_name: &str, path: &Path) -> usize {
    let base = match file_name {
        "ARCHITECTURE.md" | "DESIGN.md" => 260,
        "DATA_SOURCES.md" => 240,
        "MEMORY.md" => 250,
        "MULTI_AGENT_COORDINATION.md" => 240,
        "PERFORMANCE.md" => 230,
        "REMOTE_EXECUTION.md" => 230,
        "SANDBOX.md" => 230,
        "SERIES_GUIDE.md" => 220,
        "OPERATIONS.md" | "RUNBOOK.md" | "TROUBLESHOOTING.md" => 210,
        "CONTRIBUTING.md" => 160,
        _ if file_name.ends_with("_GUIDE.md") || file_name.ends_with("_OVERVIEW.md") => 180,
        _ => 0,
    };

    if is_repo_root_file(path) {
        base
    } else {
        base.saturating_sub(120)
    }
}

fn should_use_changed_only_fast_path(config: &AppConfig, changed_files: &[PathBuf]) -> bool {
    config.changed_only && !changed_files.is_empty()
}

fn is_fast_path_root_candidate(file_name: &str) -> bool {
    file_name == "AGENTS.md"
        || file_name == "REPO_MEMORY.md"
        || file_name == "llms.txt"
        || file_name == "README.md"
        || file_name == "README"
        || file_name == ".env.example"
        || is_manifest(file_name)
        || is_build_file(file_name)
        || is_supporting_doc(file_name)
}

fn is_placeholder_heavy_readme(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };

    let placeholder_tokens = [
        "<Title>",
        "<Header>",
        "<Usage>",
        "<Tests>",
        "<Repository>",
        "<Role>",
        "<Team>",
        "<URL>",
    ];

    let hits = placeholder_tokens
        .iter()
        .filter(|token| content.contains(**token))
        .count();

    hits >= 3
}

fn is_vendor_like_component(value: &str) -> bool {
    value.contains("vendor")
}

fn detect_language_profile(root: &Path, matcher: &IgnoreMatcher, changed_only: bool) -> LanguageProfile {
    let mut counts = HashMap::new();
    collect_language_counts(root, Path::new(""), matcher, changed_only, &mut counts);

    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|(language, count)| (Reverse(*count), language.clone()));

    LanguageProfile {
        top_languages: ranked
            .into_iter()
            .take(3)
            .map(|(language, _)| language)
            .collect::<Vec<_>>(),
    }
}

fn collect_language_counts(
    absolute_dir: &Path,
    relative_dir: &Path,
    matcher: &IgnoreMatcher,
    changed_only: bool,
    counts: &mut HashMap<String, usize>,
) {
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
            collect_language_counts(&child, &relative_path, matcher, changed_only, counts);
            continue;
        }

        if changed_only || has_non_project_context(&relative_path) {
            continue;
        }

        if let Some(language) = detect_language_for_path(&relative_path, &name) {
            *counts.entry(language.to_string()).or_insert(0) += 1;
        }
    }
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
