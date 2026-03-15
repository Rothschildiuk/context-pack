use serde_json::json;

use crate::model::{BriefingItem, GitChange, ImportantFile, LargeCodeFile, RenderContext};

const VIKING_SCHEMA_VERSION: &str = "1.0";

pub fn render(context: &RenderContext) -> String {
    let document = json!({
        "schema_version": VIKING_SCHEMA_VERSION,
        "format": "viking",
        "tiers": {
            "L0": {
                "repo": {
                    "path": context.repo.path.to_string_lossy(),
                    "project_types": context.repo.project_types,
                    "primary_languages": context.repo.primary_languages
                },
                "guidance": {
                    "repo_summary": context.briefing.repo_summary,
                    "read_these_first": map_briefing_items(&context.briefing.read_these_first),
                    "caveats": context.briefing.caveats
                }
            },
            "L1": {
                "active": {
                    "active_work": context.briefing.active_work,
                    "likely_entry_points": map_briefing_items(&context.briefing.likely_entry_points),
                    "git": {
                        "available": context.git_available,
                        "summary": context.git_summary,
                        "branch_context": {
                            "current_branch": context.git_branch_context.current_branch,
                            "local_branches": context.git_branch_context.local_branches,
                            "upstream_branch": context.git_branch_context.upstream_branch,
                            "default_branch": context.git_branch_context.default_branch,
                            "comparison_target": context.git_branch_context.comparison_target,
                            "ahead": context.git_branch_context.ahead,
                            "behind": context.git_branch_context.behind
                        },
                        "changes": map_git_changes(&context.git_changes)
                    },
                    "selected_files": map_important_files(&context.important_files)
                }
            },
            "L2": {
                "deep": {
                    "tree_summary": context.tree_summary,
                    "dependency_summary": context.briefing.dependency_summary,
                    "docker_summary": context.briefing.docker_summary,
                    "large_code_files": map_large_code_files(&context.briefing.large_code_files),
                    "notes": context.notes
                }
            }
        }
    });

    serde_json::to_string_pretty(&document)
        .map(|value| format!("{value}\n"))
        .unwrap_or_else(|_| {
            "{\n  \"schema_version\": \"1.0\",\n  \"format\": \"viking\"\n}\n".to_string()
        })
}

fn map_briefing_items(items: &[BriefingItem]) -> Vec<serde_json::Value> {
    items
        .iter()
        .map(|item| {
            json!({
                "path": item.path.to_string_lossy(),
                "reason": item.reason
            })
        })
        .collect()
}

fn map_large_code_files(files: &[LargeCodeFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| {
            json!({
                "path": file.path.to_string_lossy(),
                "loc": file.loc,
                "reason": file.reason
            })
        })
        .collect()
}

fn map_git_changes(changes: &[GitChange]) -> Vec<serde_json::Value> {
    changes
        .iter()
        .map(|change| {
            json!({
                "path": change.path.to_string_lossy(),
                "status": change.status,
                "kind": change.kind,
                "hint": change.hint
            })
        })
        .collect()
}

fn map_important_files(files: &[ImportantFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| {
            json!({
                "path": file.path.to_string_lossy(),
                "reason": file.reason,
                "why": file.why,
                "category": file.category.label(),
                "score": file.score,
                "truncated": file.truncated,
                "redacted": file.redacted,
                "redaction_reason": file.redaction_reason,
                "excerpt": file.excerpt
            })
        })
        .collect()
}
