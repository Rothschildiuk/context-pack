//! Git facts: where the current branch stands, what is uncommitted, what the
//! branch changed relative to its base, and which files change most often.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::{GitChange, GitInfo};

const MAX_CHANGES: usize = 60;
const CHURN_COMMITS: &str = "300";
const OWN_ARTIFACTS: &str = ".context-pack/";

pub struct GitResult {
    pub info: Option<GitInfo>,
    pub notes: Vec<String>,
}

pub fn collect(cwd: &Path) -> GitResult {
    let git = Git { cwd };
    let Some(status) = git.run(&["status", "--porcelain=v1", "-z", "--untracked-files=all"]) else {
        return GitResult {
            info: None,
            notes: vec!["not a git repository (or git is unavailable)".to_string()],
        };
    };

    // Porcelain paths are relative to the repository root; `--cwd` may point
    // at a subdirectory, so strip that prefix and drop paths outside it.
    let prefix = git
        .stdout(&["rev-parse", "--show-prefix"])
        .unwrap_or_default();
    let mut info = GitInfo {
        head: git.stdout(&["rev-parse", "--short", "HEAD"]),
        branch: git.stdout(&["symbolic-ref", "--quiet", "--short", "HEAD"]),
        upstream: git.stdout(&[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ]),
        shallow: git
            .stdout(&["rev-parse", "--is-shallow-repository"])
            .as_deref()
            == Some("true"),
        latest_commit_unix: git
            .stdout(&["log", "-1", "--format=%ct"])
            .and_then(|value| value.parse().ok()),
        ..GitInfo::default()
    };

    if info.upstream.is_some() {
        (info.ahead, info.behind) = parse_counts(git.stdout(&[
            "rev-list",
            "--left-right",
            "--count",
            "HEAD...@{upstream}",
        ]));
    }

    let working_stats = parse_numstat(
        &git.stdout(&[
            "diff",
            "HEAD",
            "--numstat",
            "--relative",
            "--no-renames",
            "--no-ext-diff",
        ])
        .unwrap_or_default(),
    );
    info.working_changes = parse_status(&status, &prefix)
        .into_iter()
        .map(|mut change| {
            attach_stats(&mut change, &working_stats);
            change
        })
        .take(MAX_CHANGES)
        .collect();

    info.default_branch = default_branch(&git);
    if let Some(default_branch) = info.default_branch.clone() {
        let on_default = info.branch.as_deref() == Some(default_branch.as_str());
        if !on_default {
            let base = [format!("origin/{default_branch}"), default_branch.clone()]
                .into_iter()
                .find(|candidate| {
                    git.stdout(&["rev-parse", "--verify", "--quiet", candidate])
                        .is_some()
                });
            if let Some(base) = base {
                if let Some(merge_base) = git.stdout(&["merge-base", "HEAD", &base]) {
                    info.branch_commits = git
                        .stdout(&["rev-list", "--count", &format!("{merge_base}..HEAD")])
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(0);
                    if info.branch_commits > 0 {
                        let stats = parse_numstat(
                            &git.stdout(&[
                                "diff",
                                "--numstat",
                                "--relative",
                                "--no-renames",
                                "--no-ext-diff",
                                &merge_base,
                                "HEAD",
                            ])
                            .unwrap_or_default(),
                        );
                        info.branch_changes = parse_name_status(
                            &git.stdout(&[
                                "diff",
                                "--name-status",
                                "--relative",
                                "--no-renames",
                                &merge_base,
                                "HEAD",
                            ])
                            .unwrap_or_default(),
                        )
                        .into_iter()
                        .map(|mut change| {
                            attach_stats(&mut change, &stats);
                            change
                        })
                        .take(MAX_CHANGES)
                        .collect();
                    }
                    info.base = Some(base);
                }
            }
        }
    }

    info.recent_commits = git
        .stdout(&[
            "log",
            "-n",
            "5",
            "--no-merges",
            "--date=short",
            "--format=%h %ad %s",
        ])
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default();

    if !info.shallow {
        let (churn, depth) = parse_churn(
            &git.stdout(&[
                "log",
                "-n",
                CHURN_COMMITS,
                "--no-merges",
                "--format=@@",
                "--name-only",
                "--relative",
            ])
            .unwrap_or_default(),
        );
        info.churn = churn;
        info.history_depth = depth;
    }

    let mut notes = Vec::new();
    if info.shallow {
        notes.push("shallow clone: change-frequency signals unavailable".to_string());
    }
    GitResult {
        info: Some(info),
        notes,
    }
}

struct Git<'a> {
    cwd: &'a Path,
}

impl Git<'_> {
    fn run(&self, args: &[&str]) -> Option<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.cwd)
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn stdout(&self, args: &[&str]) -> Option<String> {
        let value = self.run(args)?.trim().to_string();
        (!value.is_empty()).then_some(value)
    }
}

fn default_branch(git: &Git) -> Option<String> {
    if let Some(value) = git.stdout(&[
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ]) {
        if let Some((_, branch)) = value.split_once('/') {
            return Some(branch.to_string());
        }
    }
    ["main", "master", "trunk", "develop"]
        .into_iter()
        .find(|candidate| {
            git.stdout(&[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/heads/{candidate}"),
            ])
            .is_some()
                || git
                    .stdout(&[
                        "rev-parse",
                        "--verify",
                        "--quiet",
                        &format!("refs/remotes/origin/{candidate}"),
                    ])
                    .is_some()
        })
        .map(str::to_string)
}

fn parse_counts(value: Option<String>) -> (usize, usize) {
    let value = value.unwrap_or_default();
    let mut parts = value
        .split_whitespace()
        .map(|part| part.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

/// Parse `git status --porcelain=v1 -z` output.
fn parse_status(status: &str, prefix: &str) -> Vec<GitChange> {
    let mut changes = Vec::new();
    let mut records = status.split('\0').filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let code = &record[..2];
        let path = &record[3..];
        if code.contains('R') || code.contains('C') {
            // The next record is the original path of the rename.
            records.next();
        }
        let Some(path) = path.strip_prefix(prefix) else {
            continue;
        };
        if path.starts_with(OWN_ARTIFACTS) {
            continue;
        }
        changes.push(GitChange {
            path: path.to_string(),
            status: status_code(code).to_string(),
            added: None,
            deleted: None,
        });
    }
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    changes.dedup_by(|left, right| left.path == right.path);
    changes
}

fn status_code(code: &str) -> &'static str {
    if code == "??" {
        "??"
    } else if code.contains('R') {
        "R"
    } else if code.contains('A') {
        "A"
    } else if code.contains('D') {
        "D"
    } else if code.contains('T') {
        "T"
    } else {
        "M"
    }
}

fn parse_name_status(text: &str) -> Vec<GitChange> {
    text.lines()
        .filter_map(|line| {
            let (code, path) = line.split_once('\t')?;
            if path.starts_with(OWN_ARTIFACTS) {
                return None;
            }
            Some(GitChange {
                path: unquote(path),
                status: status_code(code).to_string(),
                added: None,
                deleted: None,
            })
        })
        .collect()
}

fn parse_numstat(text: &str) -> HashMap<String, (usize, usize)> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let added = parts.next()?.parse().ok()?;
            let deleted = parts.next()?.parse().ok()?;
            Some((unquote(parts.next()?), (added, deleted)))
        })
        .collect()
}

fn attach_stats(change: &mut GitChange, stats: &HashMap<String, (usize, usize)>) {
    if let Some((added, deleted)) = stats.get(&change.path) {
        change.added = Some(*added);
        change.deleted = Some(*deleted);
    }
}

fn parse_churn(text: &str) -> (Vec<(PathBuf, usize)>, usize) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut commits = 0usize;
    for line in text.lines() {
        if line == "@@" {
            commits += 1;
        } else if !line.is_empty() {
            *counts.entry(line).or_default() += 1;
        }
    }
    let mut churn = counts
        .into_iter()
        .map(|(path, count)| (PathBuf::from(unquote(path)), count))
        .collect::<Vec<_>>();
    churn.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    (churn, commits)
}

fn unquote(path: &str) -> String {
    path.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_z_handles_renames_and_prefix() {
        let status = "R  sub/new.rs\0sub/old.rs\0 M sub/lib.rs\0?? other/x.rs\0";
        let changes = parse_status(status, "sub/");
        assert_eq!(
            changes
                .iter()
                .map(|change| (change.path.as_str(), change.status.as_str()))
                .collect::<Vec<_>>(),
            vec![("lib.rs", "M"), ("new.rs", "R")]
        );
    }

    #[test]
    fn churn_counts_commits_and_files() {
        let (churn, commits) = parse_churn("@@\n\na.rs\nb.rs\n@@\n\na.rs\n");
        assert_eq!(commits, 2);
        assert_eq!(churn[0], (PathBuf::from("a.rs"), 2));
    }
}
