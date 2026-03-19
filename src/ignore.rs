use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_IGNORES: &[&str] = &[
    ".git/",
    "node_modules/",
    "dist/",
    "build/",
    ".next/",
    ".turbo/",
    ".venv/",
    "venv/",
    "coverage/",
    "target/",
    "out/",
    ".idea/",
    ".vscode/",
];

#[derive(Clone)]
pub struct IgnoreMatcher {
    root: PathBuf,
    rules: Vec<Rule>,
    include_rules: Vec<Rule>,
    exclude_rules: Vec<Rule>,
}

impl IgnoreMatcher {
    pub fn load(root: &Path, config: &crate::model::AppConfig) -> Self {
        let mut rules = Vec::new();
        let mut include_rules = Vec::new();
        let mut exclude_rules = Vec::new();

        for pattern in DEFAULT_IGNORES {
            rules.push(Rule::parse(pattern));
        }

        for pattern in &config.exclude {
            let rule = Rule::parse(pattern);
            rules.push(rule.clone());
            exclude_rules.push(rule);
        }

        if config.no_tests {
            for pattern in [
                "tests/",
                "test/",
                "__tests__/",
                "spec/",
                "specs/",
                "fixtures/",
            ] {
                let rule = Rule::parse(pattern);
                rules.push(rule.clone());
                exclude_rules.push(rule);
            }
        }

        rules.extend(load_ignore_file(root.join(".gitignore")));
        rules.extend(load_ignore_file(root.join(".ignore")));

        for pattern in &config.include {
            let include_rule = Rule::parse(pattern);
            let mut negated_rule = include_rule.clone();
            negated_rule.negated = true;
            rules.push(negated_rule);
            include_rules.push(include_rule);
        }

        Self {
            root: root.to_path_buf(),
            rules,
            include_rules,
            exclude_rules,
        }
    }

    pub fn is_ignored(&self, relative_path: &Path, is_dir: bool) -> bool {
        let normalized = normalize_path(relative_path);
        let file_name = relative_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();

        let mut ignored = false;

        for rule in &self.rules {
            if rule.matches(relative_path, &normalized, file_name, is_dir) {
                ignored = !rule.negated;
            }
        }

        if ignored
            && (self.matches_include_rule(relative_path, &normalized, file_name, is_dir)
                || (is_dir
                    && self
                        .include_rules
                        .iter()
                        .any(|rule| rule.may_match_descendant(&normalized))))
        {
            return false;
        }

        if ignored
            && !self.matches_exclude_rule(relative_path, &normalized, file_name, is_dir)
            && (is_shared_ide_config(relative_path, file_name, is_dir)
                || (is_dir && self.should_descend_for_shared_ide_config(relative_path)))
        {
            return false;
        }

        ignored
    }

    pub fn is_explicitly_included(&self, relative_path: &Path, is_dir: bool) -> bool {
        let normalized = normalize_path(relative_path);
        let file_name = relative_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();

        self.matches_include_rule(relative_path, &normalized, file_name, is_dir)
    }

    fn matches_include_rule(
        &self,
        relative_path: &Path,
        normalized: &str,
        file_name: &str,
        is_dir: bool,
    ) -> bool {
        self.include_rules
            .iter()
            .any(|rule| rule.matches(relative_path, normalized, file_name, is_dir))
    }

    fn matches_exclude_rule(
        &self,
        relative_path: &Path,
        normalized: &str,
        file_name: &str,
        is_dir: bool,
    ) -> bool {
        self.exclude_rules
            .iter()
            .any(|rule| rule.matches(relative_path, normalized, file_name, is_dir))
    }

    fn should_descend_for_shared_ide_config(&self, relative_path: &Path) -> bool {
        if is_vscode_dir(relative_path) {
            let dir = self.root.join(relative_path);
            return ["tasks.json", "launch.json", "extensions.json"]
                .iter()
                .any(|file_name| dir.join(file_name).is_file());
        }

        if is_idea_dir(relative_path) {
            return has_run_configuration_files(
                &self.root.join(relative_path).join("runConfigurations"),
            );
        }

        if is_idea_run_config_dir(relative_path) {
            return has_run_configuration_files(&self.root.join(relative_path));
        }

        false
    }
}

#[derive(Debug, Clone)]
struct Rule {
    pattern: String,
    negated: bool,
    directory_only: bool,
    anchored: bool,
    has_slash: bool,
}

impl Rule {
    fn parse(input: &str) -> Self {
        let mut value = input.trim().replace('\\', "/");
        let negated = value.starts_with('!');
        if negated {
            value = value[1..].to_string();
        }

        let directory_only = value.ends_with('/');
        if directory_only {
            value.pop();
        }

        let anchored = value.starts_with('/');
        if anchored {
            value = value[1..].to_string();
        }

        let has_slash = value.contains('/');

        Self {
            pattern: value,
            negated,
            directory_only,
            anchored,
            has_slash,
        }
    }

    fn matches(
        &self,
        relative_path: &Path,
        normalized: &str,
        file_name: &str,
        is_dir: bool,
    ) -> bool {
        if self.pattern.is_empty() {
            return false;
        }

        if self.directory_only && !is_dir && !has_directory_ancestor(relative_path, &self.pattern) {
            return false;
        }

        if self.has_slash {
            if self.anchored {
                return path_matches_pattern(normalized, &self.pattern)
                    || (self.directory_only
                        && starts_with_path_segment(normalized, &self.pattern));
            }

            return normalized == self.pattern
                || normalized.ends_with(&format!("/{}", self.pattern))
                || starts_with_path_segment(normalized, &self.pattern)
                || path_matches_pattern(normalized, &self.pattern);
        }

        if self.directory_only {
            return has_directory_ancestor(relative_path, &self.pattern)
                || (is_dir && component_matches(relative_path, &self.pattern));
        }

        component_matches(relative_path, &self.pattern) || wildcard_match(file_name, &self.pattern)
    }

    fn may_match_descendant(&self, normalized_dir: &str) -> bool {
        if self.pattern.is_empty() {
            return false;
        }

        if !self.has_slash {
            return true;
        }

        let prefix = literal_prefix(&self.pattern);
        if prefix.is_empty() {
            return true;
        }

        let candidate = prefix.trim_matches('/');
        let normalized_dir = normalized_dir.trim_matches('/');

        if candidate.is_empty() || normalized_dir.is_empty() {
            return true;
        }

        starts_with_path_segment(candidate, normalized_dir)
            || starts_with_path_segment(normalized_dir, candidate)
    }
}

fn load_ignore_file(path: PathBuf) -> Vec<Rule> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }

            Some(Rule::parse(trimmed))
        })
        .collect()
}

fn normalize_path(path: &Path) -> String {
    path.iter()
        .map(|segment| segment.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_shared_ide_config(relative_path: &Path, file_name: &str, is_dir: bool) -> bool {
    if is_dir {
        return false;
    }

    if file_name == ".editorconfig" {
        return true;
    }

    if is_vscode_child(relative_path) {
        return matches!(file_name, "tasks.json" | "launch.json" | "extensions.json");
    }

    is_idea_run_config_file(relative_path, file_name)
}

fn is_vscode_child(path: &Path) -> bool {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        == Some(".vscode")
}

fn is_vscode_dir(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some(".vscode")
}

fn is_idea_dir(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some(".idea")
}

fn is_idea_run_config_dir(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some("runConfigurations")
        && path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some(".idea")
}

fn is_idea_run_config_file(path: &Path, file_name: &str) -> bool {
    file_name.ends_with(".xml") && path.parent().map(is_idea_run_config_dir).unwrap_or(false)
}

fn has_run_configuration_files(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };

    entries.flatten().any(|entry| {
        entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or(false)
            && entry.path().extension().and_then(|value| value.to_str()) == Some("xml")
    })
}

fn component_matches(path: &Path, pattern: &str) -> bool {
    path.iter()
        .any(|segment| wildcard_match(&segment.to_string_lossy(), pattern))
}

fn has_directory_ancestor(path: &Path, pattern: &str) -> bool {
    let mut current = PathBuf::new();

    for segment in path.iter() {
        current.push(segment);
        if wildcard_match(&segment.to_string_lossy(), pattern) {
            return true;
        }
    }

    let normalized = normalize_path(&current);
    starts_with_path_segment(&normalized, pattern)
}

fn starts_with_path_segment(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    wildcard_match(path, pattern)
}

fn literal_prefix(pattern: &str) -> &str {
    let mut end = pattern.len();

    for (index, ch) in pattern.char_indices() {
        if matches!(ch, '*' | '?') {
            end = index;
            break;
        }
    }

    &pattern[..end]
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value_chars: Vec<char> = value.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let mut value_index = 0usize;
    let mut pattern_index = 0usize;
    let mut star_index = None;
    let mut match_index = 0usize;

    while value_index < value_chars.len() {
        if pattern_index < pattern_chars.len()
            && (pattern_chars[pattern_index] == '?'
                || pattern_chars[pattern_index] == value_chars[value_index])
        {
            value_index += 1;
            pattern_index += 1;
            continue;
        }

        if pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
            star_index = Some(pattern_index);
            match_index = value_index;
            pattern_index += 1;
            continue;
        }

        if let Some(star) = star_index {
            pattern_index = star + 1;
            match_index += 1;
            value_index = match_index;
            continue;
        }

        return false;
    }

    while pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
        pattern_index += 1;
    }

    pattern_index == pattern_chars.len()
}
