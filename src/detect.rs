use std::path::Path;

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, ImportantFile, RepoInfo};

pub fn detect_repo_info_with_matcher(
    config: &AppConfig,
    files: &[ImportantFile],
    matcher: &IgnoreMatcher,
) -> RepoInfo {
    let mut types = detect_project_types(files);
    let mut languages = detect_languages(files);

    if types.is_empty() || languages.is_empty() {
        let fallback = scan_repo(config, matcher);
        merge_unique(&mut types, fallback.project_types);
        merge_unique(&mut languages, fallback.primary_languages);
    }

    if types.is_empty() {
        types.push("unknown".to_string());
    }
    languages.truncate(2);

    RepoInfo {
        path: config.cwd.clone(),
        project_types: types,
        primary_languages: languages,
    }
}

struct DetectionState {
    project_types: Vec<String>,
    primary_languages: Vec<String>,
    scanned_files: usize,
    scan_limit: usize,
}

impl DetectionState {
    fn new() -> Self {
        Self {
            project_types: Vec::new(),
            primary_languages: Vec::new(),
            scanned_files: 0,
            scan_limit: 200,
        }
    }

    fn limit_reached(&self) -> bool {
        self.scanned_files >= self.scan_limit
    }
}

fn scan_repo(config: &AppConfig, matcher: &IgnoreMatcher) -> RepoInfo {
    let mut state = DetectionState::new();
    scan_dir(&config.cwd, Path::new(""), 0, matcher, &mut state);

    RepoInfo {
        path: config.cwd.clone(),
        project_types: state.project_types,
        primary_languages: state.primary_languages,
    }
}

fn scan_dir(
    absolute_dir: &Path,
    relative_dir: &Path,
    depth: usize,
    matcher: &IgnoreMatcher,
    state: &mut DetectionState,
) {
    if depth > 2 || state.limit_reached() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(absolute_dir) else {
        return;
    };

    let mut children = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    children.sort();

    for child in children {
        if state.limit_reached() {
            return;
        }

        let Ok(metadata) = std::fs::symlink_metadata(&child) else {
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
            scan_dir(&child, &relative_path, depth + 1, matcher, state);
            continue;
        }

        state.scanned_files += 1;
        record_path(&relative_path, state);
    }
}

fn record_path(path: &Path, state: &mut DetectionState) {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return;
    };
    if let Some(root_dir) = path.iter().next().and_then(|value| value.to_str()) {
        if root_dir == "C" {
            push_unique(&mut state.project_types, "c");
            push_unique(&mut state.primary_languages, "c");
        }
        if root_dir == "Coq" {
            push_unique(&mut state.project_types, "coq");
            push_unique(&mut state.primary_languages, "coq");
        }
    }

    if matches!(file_name, "Cargo.toml") {
        push_unique(&mut state.project_types, "rust");
        push_unique(&mut state.primary_languages, "rust");
    }
    if is_java_manifest(file_name) {
        push_unique(&mut state.project_types, "java");
        push_unique(&mut state.primary_languages, "java");
    }
    if matches!(file_name, "package.json") && is_repo_root_file(path) {
        push_unique(&mut state.project_types, "node");
    }
    if matches!(file_name, "tsconfig.json") && is_repo_root_file(path) {
        push_unique(&mut state.project_types, "node");
        push_unique(&mut state.primary_languages, "typescript");
    }
    if matches!(file_name, "pyproject.toml" | "requirements.txt") {
        push_unique(&mut state.project_types, "python");
        push_unique(&mut state.primary_languages, "python");
    }
    if matches!(file_name, "go.mod") {
        push_unique(&mut state.project_types, "go");
        push_unique(&mut state.primary_languages, "go");
    }
    if matches!(file_name, "cabal.project" | "stack.yaml" | "package.yaml") {
        push_unique(&mut state.project_types, "haskell");
        push_unique(&mut state.primary_languages, "haskell");
    }

    match path.extension().and_then(|value| value.to_str()) {
        Some("rs") => {
            push_unique(&mut state.project_types, "rust");
            push_unique(&mut state.primary_languages, "rust");
        }
        Some("py") => {
            push_unique(&mut state.project_types, "python");
            push_unique(&mut state.primary_languages, "python");
        }
        Some("go") => {
            push_unique(&mut state.project_types, "go");
            push_unique(&mut state.primary_languages, "go");
        }
        Some("java" | "kt") => {
            push_unique(&mut state.project_types, "java");
            push_unique(&mut state.primary_languages, "java");
        }
        Some("ts" | "tsx") => {
            push_unique(&mut state.project_types, "node");
            push_unique(&mut state.primary_languages, "typescript");
        }
        Some("js" | "jsx") => {
            push_unique(&mut state.project_types, "node");
            push_unique(&mut state.primary_languages, "javascript");
        }
        Some("c" | "h") => {
            push_unique(&mut state.project_types, "c");
            push_unique(&mut state.primary_languages, "c");
        }
        Some("v") => {
            push_unique(&mut state.project_types, "coq");
            push_unique(&mut state.primary_languages, "coq");
        }
        Some("hs") => {
            push_unique(&mut state.project_types, "haskell");
            push_unique(&mut state.primary_languages, "haskell");
        }
        _ => {}
    }
}

fn detect_project_types(files: &[ImportantFile]) -> Vec<String> {
    let mut types = Vec::new();

    if has_file(files, "Cargo.toml") {
        types.push("rust".to_string());
    }
    if has_file(files, "pom.xml")
        || has_file(files, "build.gradle")
        || has_file(files, "build.gradle.kts")
        || has_file(files, "settings.gradle")
        || has_file(files, "settings.gradle.kts")
        || has_extension(files, "java")
        || has_extension(files, "kt")
    {
        types.push("java".to_string());
    }
    if has_file(files, "package.json")
        || has_extension(files, "ts")
        || has_extension(files, "tsx")
        || has_extension(files, "js")
        || has_extension(files, "jsx")
    {
        types.push("node".to_string());
    }
    if has_file(files, "pyproject.toml") || has_file(files, "requirements.txt") {
        types.push("python".to_string());
    }
    if has_file(files, "go.mod") {
        types.push("go".to_string());
    }
    if has_extension(files, "hs")
        || has_file(files, "cabal.project")
        || has_file(files, "stack.yaml")
    {
        types.push("haskell".to_string());
    }
    if has_extension(files, "c") || has_extension(files, "h") {
        types.push("c".to_string());
    }
    if has_extension(files, "v") {
        types.push("coq".to_string());
    }

    types
}

fn detect_languages(files: &[ImportantFile]) -> Vec<String> {
    let mut languages = Vec::new();

    if has_file(files, "Cargo.toml") || has_extension(files, "rs") {
        languages.push("rust".to_string());
    }
    if has_file(files, "go.mod") || has_extension(files, "go") {
        languages.push("go".to_string());
    }
    if has_file(files, "pom.xml")
        || has_file(files, "build.gradle")
        || has_file(files, "build.gradle.kts")
        || has_file(files, "settings.gradle")
        || has_file(files, "settings.gradle.kts")
        || has_extension(files, "java")
        || has_extension(files, "kt")
    {
        languages.push("java".to_string());
    }
    if has_file(files, "pyproject.toml") || has_extension(files, "py") {
        languages.push("python".to_string());
    }
    if has_file(files, "tsconfig.json") || has_extension(files, "ts") || has_extension(files, "tsx")
    {
        languages.push("typescript".to_string());
    }
    if has_extension(files, "js") || has_extension(files, "jsx") || has_file(files, "package.json")
    {
        languages.push("javascript".to_string());
    }
    if has_extension(files, "c") || has_extension(files, "h") {
        languages.push("c".to_string());
    }
    if has_extension(files, "v") {
        languages.push("coq".to_string());
    }
    if has_extension(files, "hs") {
        languages.push("haskell".to_string());
    }

    languages
}

fn is_repo_root_file(path: &Path) -> bool {
    path.components().count() == 1
}

fn has_file(files: &[ImportantFile], name: &str) -> bool {
    files.iter().any(|file| file.file_name() == Some(name))
}

fn has_extension(files: &[ImportantFile], extension: &str) -> bool {
    files.iter().any(|file| {
        file.path.extension().and_then(|value| value.to_str()) == Some(extension)
            && !is_auxiliary_script_path(&file.path)
    })
}

fn is_auxiliary_script_path(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "scripts" || s == "script" || s == "tools" || s == "hack"
    })
}

fn merge_unique(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        push_unique(target, &item);
    }
}

fn push_unique(target: &mut Vec<String>, item: &str) {
    if !target.iter().any(|value| value == item) {
        target.push(item.to_string());
    }
}

fn is_java_manifest(file_name: &str) -> bool {
    matches!(
        file_name,
        "pom.xml" | "build.gradle" | "build.gradle.kts" | "settings.gradle" | "settings.gradle.kts"
    )
}
