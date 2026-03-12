use std::fs;
use std::path::Path;

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, WalkResult};

pub fn build_tree_summary(config: &AppConfig) -> WalkResult {
    if config.no_tree {
        return WalkResult {
            tree_summary: "Tree output disabled.".to_string(),
            notes: Vec::new(),
        };
    }

    let matcher = IgnoreMatcher::load(&config.cwd, config);
    let mut state = WalkState::new(config.max_files);

    if !config.cwd.exists() {
        return WalkResult {
            tree_summary: "Repository root does not exist.".to_string(),
            notes: vec![format!("missing cwd: {}", config.cwd.display())],
        };
    }

    if !config.cwd.is_dir() {
        return WalkResult {
            tree_summary: "Repository root is not a directory.".to_string(),
            notes: vec![format!("invalid cwd: {}", config.cwd.display())],
        };
    }

    let root_name = config
        .cwd
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(".");

    state.push_line(format!("{root_name}/"));
    visit_dir(
        &config.cwd,
        Path::new(""),
        0,
        config,
        &matcher,
        &mut state,
    );

    WalkResult {
        tree_summary: state.render_tree(),
        notes: state.render_notes(),
    }
}

fn visit_dir(
    absolute_dir: &Path,
    relative_dir: &Path,
    depth: usize,
    config: &AppConfig,
    matcher: &IgnoreMatcher,
    state: &mut WalkState,
) {
    if depth >= config.max_depth {
        state.depth_omissions += 1;
        return;
    }

    let Ok(entries) = fs::read_dir(absolute_dir) else {
        state.io_omissions += 1;
        return;
    };

    let mut children = Vec::new();

    for entry in entries.flatten() {
        children.push(entry.path());
    }

    children.sort_by(|left, right| {
        let left_name = left.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let right_name = right.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        left_name.cmp(right_name)
    });

    for child in children {
        let name = child
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let relative_path = relative_dir.join(&name);

        let Ok(metadata) = fs::symlink_metadata(&child) else {
            state.io_omissions += 1;
            continue;
        };

        let is_dir = metadata.is_dir();

        if matcher.is_ignored(&relative_path, is_dir) {
            state.ignored_entries += 1;
            continue;
        }

        if state.limit_reached() {
            state.limit_omissions += 1;
            continue;
        }

        let indent = "  ".repeat(depth + 1);
        if is_dir {
            state.push_line(format!("{indent}{name}/"));
            visit_dir(&child, &relative_path, depth + 1, config, matcher, state);
        } else {
            state.push_line(format!("{indent}{name}"));
        }
    }
}

struct WalkState {
    lines: Vec<String>,
    entry_budget: usize,
    rendered_entries: usize,
    ignored_entries: usize,
    limit_omissions: usize,
    depth_omissions: usize,
    io_omissions: usize,
}

impl WalkState {
    fn new(entry_budget: usize) -> Self {
        Self {
            lines: Vec::new(),
            entry_budget,
            rendered_entries: 0,
            ignored_entries: 0,
            limit_omissions: 0,
            depth_omissions: 0,
            io_omissions: 0,
        }
    }

    fn push_line(&mut self, line: String) {
        self.lines.push(line);
        self.rendered_entries += 1;
    }

    fn limit_reached(&self) -> bool {
        self.rendered_entries >= self.entry_budget
    }

    fn render_tree(&self) -> String {
        self.lines.join("\n")
    }

    fn render_notes(&self) -> Vec<String> {
        let mut notes = Vec::new();

        if self.ignored_entries > 0 {
            notes.push(format!("ignored entries: {}", self.ignored_entries));
        }

        if self.limit_omissions > 0 {
            notes.push(format!("tree entries omitted by limit: {}", self.limit_omissions));
        }

        if self.depth_omissions > 0 {
            notes.push(format!("subtrees omitted by depth: {}", self.depth_omissions));
        }

        if self.io_omissions > 0 {
            notes.push(format!("entries omitted due to I/O errors: {}", self.io_omissions));
        }

        notes
    }
}
