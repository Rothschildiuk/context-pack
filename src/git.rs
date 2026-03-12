use std::path::PathBuf;
use std::process::Command;

use crate::model::{AppConfig, GitResult};

pub fn collect(config: &AppConfig) -> GitResult {
    if config.no_git {
        return GitResult {
            summary: "Git collection disabled.".to_string(),
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
            changed_files: Vec::new(),
            notes: vec![note],
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary = if stdout.trim().is_empty() {
        "Working tree clean.".to_string()
    } else {
        stdout.into_owned()
    };

    GitResult {
        summary,
        changed_files: parse_changed_files(&stdout),
        notes: Vec::new(),
    }
}

fn parse_changed_files(status: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for line in status.lines() {
        if line.len() < 4 {
            continue;
        }

        let path = line[3..].trim();
        if path.is_empty() {
            continue;
        }

        let renamed = path.rsplit_once(" -> ").map(|(_, right)| right).unwrap_or(path);
        let normalized = renamed.trim_matches('"');
        paths.push(PathBuf::from(normalized));
    }

    paths.sort();
    paths.dedup();
    paths
}
