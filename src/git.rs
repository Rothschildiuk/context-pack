use std::path::PathBuf;
use std::process::Command;

use crate::ignore::IgnoreMatcher;
use crate::model::{AppConfig, GitBranchContext, GitChange, GitResult};
use crate::select;

pub fn collect(config: &AppConfig, summary_budget: usize) -> GitResult {
    if config.no_git {
        return GitResult {
            summary: "Git collection disabled.".to_string(),
            available: false,
            branch_context: GitBranchContext::default(),
            changes: Vec::new(),
            changed_files: Vec::new(),
            notes: Vec::new(),
        };
    }

    let output = git_output(config, ["status", "--short", "--untracked-files=all"]);

    let Ok(output) = output else {
        return GitResult {
            summary: "Git is unavailable.".to_string(),
            available: false,
            branch_context: GitBranchContext::default(),
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
            branch_context: GitBranchContext::default(),
            changes: Vec::new(),
            changed_files: Vec::new(),
            notes: vec![note],
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let diff_stats = collect_diff_stats(config);
    let changes = filter_changes(parse_changes(&stdout, &diff_stats), config);
    let branch_context = collect_branch_context(config);
    let mut notes = Vec::new();
    let summary = if stdout.trim().is_empty() {
        "Working tree clean.".to_string()
    } else if changes.is_empty() {
        notes.push("git changes omitted as low-signal noise".to_string());
        "No high-signal changes detected.".to_string()
    } else {
        trim_summary(&render_changes(&changes), summary_budget, &mut notes)
    };

    GitResult {
        summary,
        available: true,
        branch_context,
        changed_files: changes.iter().map(|change| change.path.clone()).collect(),
        changes,
        notes,
    }
}

fn collect_branch_context(config: &AppConfig) -> GitBranchContext {
    let current_branch = git_stdout(config, ["symbolic-ref", "--quiet", "--short", "HEAD"]);
    let local_branches = git_stdout(
        config,
        ["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .map(|value| {
        value
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    let upstream_branch = git_stdout(
        config,
        [
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    );

    let default_branch = infer_default_branch(config, upstream_branch.as_deref(), &local_branches);
    let comparison_target = if let Some(upstream) = upstream_branch.clone() {
        Some(upstream)
    } else if let (Some(current), Some(default_branch)) =
        (current_branch.as_ref(), default_branch.as_ref())
    {
        if current != default_branch && local_branches.iter().any(|branch| branch == default_branch)
        {
            Some(default_branch.clone())
        } else {
            None
        }
    } else {
        None
    };

    let (ahead, behind) = match (current_branch.as_deref(), comparison_target.as_deref()) {
        (Some(current), Some(target)) => parse_ahead_behind(
            git_stdout(
                config,
                [
                    "rev-list",
                    "--left-right",
                    "--count",
                    &format!("{current}...{target}"),
                ],
            )
            .as_deref(),
        ),
        _ => (0, 0),
    };

    GitBranchContext {
        current_branch,
        local_branches,
        upstream_branch,
        default_branch,
        comparison_target,
        ahead,
        behind,
    }
}

fn filter_changes(changes: Vec<GitChange>, config: &AppConfig) -> Vec<GitChange> {
    let matcher = IgnoreMatcher::load(&config.cwd, config);
    let mut filtered = Vec::new();

    for change in changes {
        if matcher.is_ignored(&change.path, false) {
            continue;
        }
        if !select::is_relevant_change_path(&change.path) {
            continue;
        }
        filtered.push(change);
    }

    filtered
}

fn infer_default_branch(
    config: &AppConfig,
    upstream_branch: Option<&str>,
    local_branches: &[String],
) -> Option<String> {
    if let Some(remote) = upstream_branch
        .and_then(|branch| branch.split('/').next())
        .filter(|remote| !remote.is_empty())
    {
        let ref_name = format!("refs/remotes/{remote}/HEAD");
        if let Some(value) = git_stdout(config, ["symbolic-ref", "--quiet", "--short", &ref_name]) {
            if let Some((_, branch)) = value.rsplit_once('/') {
                return Some(branch.to_string());
            }
        }
    }

    if let Some(value) = git_stdout(
        config,
        [
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        if let Some((_, branch)) = value.rsplit_once('/') {
            return Some(branch.to_string());
        }
    }

    for preferred in ["main", "master", "develop"] {
        if local_branches.iter().any(|branch| branch == preferred) {
            return Some(preferred.to_string());
        }
    }

    if local_branches.len() == 1 {
        return local_branches.first().cloned();
    }

    None
}

fn parse_ahead_behind(value: Option<&str>) -> (usize, usize) {
    let Some(value) = value else {
        return (0, 0);
    };

    let mut parts = value.split_whitespace();
    let ahead = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(0);
    let behind = parts
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(0);
    (ahead, behind)
}

fn git_stdout<const N: usize>(config: &AppConfig, args: [&str; N]) -> Option<String> {
    let output = git_output(config, args).ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn git_output<const N: usize>(
    config: &AppConfig,
    args: [&str; N],
) -> Result<std::process::Output, std::io::Error> {
    Command::new("git")
        .arg("-C")
        .arg(&config.cwd)
        .args(args)
        .output()
}

fn render_changes(changes: &[GitChange]) -> String {
    changes
        .iter()
        .map(|change| {
            let mut line = format!(" {} `{}`", change.status, change.path.display());
            if let Some(hint) = &change.hint {
                line.push_str(&format!(" ({hint})"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_changes(status: &str, diff_stats: &[DiffStat]) -> Vec<GitChange> {
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
        let kind = status_label(raw_status).to_string();
        let status = status_code(&kind).to_string();
        changes.push(GitChange {
            path: PathBuf::from(normalized),
            status,
            kind: kind.clone(),
            hint: diff_hint(PathBuf::from(normalized).as_path(), &kind, diff_stats),
        });
    }

    changes.sort_by(|left, right| left.path.cmp(&right.path));
    changes.dedup_by(|left, right| left.path == right.path);
    changes
}

#[derive(Debug, Clone)]
struct DiffStat {
    path: PathBuf,
    added: usize,
    deleted: usize,
}

fn collect_diff_stats(config: &AppConfig) -> Vec<DiffStat> {
    let mut stats = parse_numstat(
        git_stdout(config, ["diff", "--numstat", "--no-ext-diff"]).as_deref(),
    );
    let staged = parse_numstat(
        git_stdout(config, ["diff", "--cached", "--numstat", "--no-ext-diff"]).as_deref(),
    );

    for staged_stat in staged {
        if let Some(existing) = stats.iter_mut().find(|value| value.path == staged_stat.path) {
            existing.added += staged_stat.added;
            existing.deleted += staged_stat.deleted;
        } else {
            stats.push(staged_stat);
        }
    }

    stats
}

fn parse_numstat(output: Option<&str>) -> Vec<DiffStat> {
    let Some(output) = output else {
        return Vec::new();
    };

    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let added = parts.next()?.parse::<usize>().ok()?;
            let deleted = parts.next()?.parse::<usize>().ok()?;
            let path = parts.next()?;
            let normalized = path
                .rsplit_once(" -> ")
                .map(|(_, right)| right)
                .unwrap_or(path)
                .trim_matches('"');

            Some(DiffStat {
                path: PathBuf::from(normalized),
                added,
                deleted,
            })
        })
        .collect()
}

fn diff_hint(path: &std::path::Path, kind: &str, diff_stats: &[DiffStat]) -> Option<String> {
    if let Some(stat) = diff_stats.iter().find(|value| value.path == path) {
        if stat.added > 0 || stat.deleted > 0 {
            return Some(format!("+{} -{}", stat.added, stat.deleted));
        }
    }

    match kind {
        "untracked" | "added" => Some("new file".to_string()),
        "deleted" => Some("deleted file".to_string()),
        "renamed" => Some("renamed file".to_string()),
        _ => None,
    }
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

fn status_code(kind: &str) -> &'static str {
    match kind {
        "modified" => "M",
        "added" => "A",
        "deleted" => "D",
        "renamed" => "R",
        "untracked" => "??",
        "type_changed" => "T",
        _ => "M",
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
