use std::path::PathBuf;
use std::process::Command;

use crate::model::{AppConfig, GitChange, GitResult};

pub fn collect(config: &AppConfig, summary_budget: usize) -> GitResult {
    if config.no_git {
        return GitResult {
            summary: "Git collection disabled.".to_string(),
            available: false,
            changes: Vec::new(),
            changed_files: Vec::new(),
            notes: Vec::new(),
        };
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(&config.cwd)
        .args(["status", "--short", "--untracked-files=all"])
        .output();

    let Ok(output) = output else {
        return GitResult {
            summary: "Git is unavailable.".to_string(),
            available: false,
            changes: Vec::new(),
            changed_files: Vec::new(),
            notes: vec!["git command failed to start".to_string()],
        };
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let note = if stderr.is_empty() {
            "git status returned a non-zero exit code".to_string()
        } else {
            format!("git status failed: {stderr}")
        };

        return GitResult {
            summary: "Git context unavailable.".to_string(),
            available: false,
            changes: Vec::new(),
            changed_files: Vec::new(),
            notes: vec![note],
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let changes = parse_changes(&stdout);
    let mut notes = Vec::new();
    let summary = if stdout.trim().is_empty() {
        "Working tree clean.".to_string()
    } else {
        trim_summary(&stdout, summary_budget, &mut notes)
    };

    GitResult {
        summary,
        available: true,
        changed_files: changes.iter().map(|change| change.path.clone()).collect(),
        changes,
        notes,
    }
}

fn parse_changes(status: &str) -> Vec<GitChange> {
    let mut changes = Vec::new();

    for line in status.lines() {
        if line.len() < 4 {
            continue;
        }

        let raw_status = &line[..2];
        let path = line[3..].trim();
        if path.is_empty() {
            continue;
        }

        let renamed = path
            .rsplit_once(" -> ")
            .map(|(_, right)| right)
            .unwrap_or(path);
        let normalized = renamed.trim_matches('"');
        changes.push(GitChange {
            path: PathBuf::from(normalized),
            kind: status_label(raw_status).to_string(),
        });
    }

    changes.sort_by(|left, right| left.path.cmp(&right.path));
    changes.dedup_by(|left, right| left.path == right.path);
    changes
}

fn status_label(status: &str) -> &'static str {
    if status.contains('?') {
        "untracked"
    } else if status.contains('R') {
        "renamed"
    } else if status.contains('A') {
        "added"
    } else if status.contains('D') {
        "deleted"
    } else if status.contains('M') {
        "modified"
    } else if status.contains('T') {
        "type_changed"
    } else {
        "changed"
    }
}

fn trim_summary(summary: &str, budget: usize, notes: &mut Vec<String>) -> String {
    let compact = summary.trim_end();
    if compact.len() <= budget {
        return compact.to_string();
    }

    let mut end = 0usize;
    for (index, _) in compact.char_indices() {
        if index > budget {
            break;
        }
        end = index;
    }

    notes.push(format!("git summary truncated to budget: {}", budget));
    format!("{}\n... [truncated]", compact[..end].trim_end())
}
