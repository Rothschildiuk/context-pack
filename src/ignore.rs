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

pub struct IgnoreMatcher {
    rules: Vec<Rule>,
}

impl IgnoreMatcher {
    pub fn load(root: &Path, config: &crate::model::AppConfig) -> Self {
        let mut rules = Vec::new();

        for pattern in DEFAULT_IGNORES {
            rules.push(Rule::parse(pattern));
        }

        for pattern in &config.exclude {
            rules.push(Rule::parse(pattern));
        }

        rules.extend(load_ignore_file(root.join(".gitignore")));
        rules.extend(load_ignore_file(root.join(".ignore")));

        for pattern in &config.include {
            let mut rule = Rule::parse(pattern);
            rule.negated = true;
            rules.push(rule);
        }

        Self { rules }
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

        ignored
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

    fn matches(&self, relative_path: &Path, normalized: &str, file_name: &str, is_dir: bool) -> bool {
        if self.pattern.is_empty() {
            return false;
        }

        if self.directory_only && !is_dir && !has_directory_ancestor(relative_path, &self.pattern) {
            return false;
        }

        if self.has_slash {
            if self.anchored {
                return path_matches_pattern(normalized, &self.pattern)
                    || (self.directory_only && starts_with_path_segment(normalized, &self.pattern));
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

fn component_matches(path: &Path, pattern: &str) -> bool {
    path.iter().any(|segment| wildcard_match(&segment.to_string_lossy(), pattern))
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
