use std::fmt::Write as _;

use crate::model::{Brief, CommandHint, FileRef, GitChange, GitInfo, LayoutEntry};

pub fn render(brief: &Brief) -> String {
    let mut out = String::new();
    render_header(&mut out, brief);
    render_instructions(&mut out, &brief.instructions);
    render_commands(&mut out, &brief.commands);
    render_files(&mut out, "Entry points", &brief.entry_points);
    render_files(&mut out, "Key files", &brief.key_files);
    render_workspace(&mut out, brief);
    render_layout(&mut out, &brief.layout);
    if let Some(git) = &brief.git {
        render_git(&mut out, git);
    }
    render_files(&mut out, "Docs", &brief.docs);
    render_files(&mut out, "Config", &brief.config);
    render_dependencies(&mut out, brief);
    render_memory(&mut out, brief);
    render_excerpts(&mut out, brief);

    if !brief.notes.is_empty() {
        out.push_str("## Notes\n");
        for note in &brief.notes {
            let _ = writeln!(out, "- {note}");
        }
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "<!-- context-pack {} · schema {} · {} files indexed -->",
        brief.tool_version, brief.schema_version, brief.stats.files_indexed
    );
    out
}

fn render_header(out: &mut String, brief: &Brief) {
    let repo = &brief.repo;
    let _ = writeln!(out, "# {} — context pack\n", repo.name);
    if let Some(description) = &repo.description {
        let _ = writeln!(out, "> {description}\n");
    }
    if !repo.languages.is_empty() {
        let languages = repo
            .languages
            .iter()
            .map(|language| format!("{} ({})", language.name, language.files))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "- Languages: {languages}");
    }
    if !repo.stack.is_empty() {
        let _ = writeln!(out, "- Stack: {}", repo.stack.join(", "));
    }
    if let Some(git) = &brief.git {
        let _ = writeln!(out, "- Git: {}", git_headline(git));
    }
    if let Some(memory) = &brief.memory {
        let status = match &memory.stale_reason {
            Some(reason) => format!("stale — {reason}"),
            None => "see Repo memory below".to_string(),
        };
        let _ = writeln!(out, "- Memory: `{}` ({status})", memory.path);
    }
    out.push('\n');
}

fn git_headline(git: &GitInfo) -> String {
    let mut parts = Vec::new();
    match (&git.branch, &git.head) {
        (Some(branch), _) => parts.push(format!("branch `{branch}`")),
        (None, Some(head)) => parts.push(format!("detached at `{head}`")),
        (None, None) => parts.push("no commits yet".to_string()),
    }
    if let Some(upstream) = &git.upstream {
        let mut tracking = format!("tracks `{upstream}`");
        if git.ahead > 0 || git.behind > 0 {
            let _ = write!(tracking, " (ahead {}, behind {})", git.ahead, git.behind);
        }
        parts.push(tracking);
    }
    if let Some(base) = &git.base {
        parts.push(match git.branch_commits {
            0 => format!("no commits ahead of `{base}`"),
            count => format!("{count} commit(s) ahead of `{base}`"),
        });
    } else if let Some(default_branch) = &git.default_branch {
        if git.branch.as_deref() == Some(default_branch.as_str()) {
            parts.push("default branch".to_string());
        } else {
            parts.push(format!("default branch `{default_branch}`"));
        }
    }
    let uncommitted = git.working_changes.len();
    parts.push(if uncommitted == 0 {
        "working tree clean".to_string()
    } else {
        format!("{uncommitted} uncommitted change(s)")
    });
    parts.join(", ")
}

fn render_instructions(out: &mut String, files: &[FileRef]) {
    if files.is_empty() {
        return;
    }
    out.push_str("## Agent instructions\nRead and follow these before editing:\n");
    for file in files {
        render_file_line(out, file);
    }
    out.push('\n');
}

fn render_commands(out: &mut String, commands: &[CommandHint]) {
    if commands.is_empty() {
        return;
    }
    out.push_str("## Commands\n");
    for hint in commands {
        let _ = writeln!(out, "- {}: `{}` — {}", hint.kind, hint.command, hint.source);
    }
    out.push('\n');
}

fn render_files(out: &mut String, title: &str, files: &[FileRef]) {
    if files.is_empty() {
        return;
    }
    let _ = writeln!(out, "## {title}");
    for file in files {
        render_file_line(out, file);
    }
    out.push('\n');
}

fn render_file_line(out: &mut String, file: &FileRef) {
    let _ = write!(out, "- `{}` — {}", file.path, file.reason);
    if let Some(lines) = file.lines {
        if !file.reason.contains(" lines") {
            let _ = write!(out, " · {lines} lines");
        }
    }
    out.push('\n');
}

fn render_workspace(out: &mut String, brief: &Brief) {
    let Some(workspace) = &brief.workspace else {
        return;
    };
    let _ = writeln!(out, "## Workspace ({} packages)", workspace.packages);
    for group in &workspace.groups {
        let role = if group.role == "source" {
            String::new()
        } else {
            format!(" [{}]", group.role)
        };
        let more = group.count.saturating_sub(group.examples.len());
        let examples = group
            .examples
            .iter()
            .map(|example| format!("`{example}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if more > 0 {
            format!(", +{more} more")
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "- `{}` — {} package(s){role}: {examples}{suffix}",
            group.pattern, group.count
        );
    }
    out.push('\n');
}

fn render_layout(out: &mut String, layout: &[LayoutEntry]) {
    if layout.is_empty() {
        return;
    }
    out.push_str("## Layout\n");
    for entry in layout {
        render_layout_entry(out, entry, 0);
        for child in &entry.children {
            render_layout_entry(out, child, 1);
        }
    }
    out.push('\n');
}

fn render_layout_entry(out: &mut String, entry: &LayoutEntry, depth: usize) {
    let indent = "  ".repeat(depth);
    let _ = write!(out, "{indent}- `{}` — {} file(s)", entry.path, entry.files);
    if !entry.languages.is_empty() {
        let _ = write!(out, ", {}", entry.languages.join("/"));
    }
    if entry.role != "source" {
        let _ = write!(out, " [{}]", entry.role);
    }
    out.push('\n');
}

fn render_git(out: &mut String, git: &GitInfo) {
    let has_branch = git.base.is_some() && !git.branch_changes.is_empty();
    if !has_branch && git.working_changes.is_empty() && git.recent_commits.is_empty() {
        return;
    }
    out.push_str("## Active work\n");
    if has_branch {
        let base = git.base.as_deref().unwrap_or_default();
        let _ = writeln!(
            out,
            "- Branch changes vs `{base}` ({} commit(s), {} file(s)):",
            git.branch_commits,
            git.branch_changes.len()
        );
        render_changes(out, &git.branch_changes);
    }
    if !git.working_changes.is_empty() {
        let _ = writeln!(out, "- Uncommitted ({}):", git.working_changes.len());
        render_changes(out, &git.working_changes);
    }
    if !git.recent_commits.is_empty() {
        out.push_str("- Recent commits:\n");
        for commit in &git.recent_commits {
            let _ = writeln!(out, "  - {commit}");
        }
    }
    out.push('\n');
}

fn render_changes(out: &mut String, changes: &[GitChange]) {
    const SHOWN: usize = 15;
    for change in changes.iter().take(SHOWN) {
        let _ = write!(out, "  - {} `{}`", change.status, change.path);
        if let (Some(added), Some(deleted)) = (change.added, change.deleted) {
            let _ = write!(out, " (+{added} -{deleted})");
        }
        out.push('\n');
    }
    if changes.len() > SHOWN {
        let _ = writeln!(out, "  - … {} more", changes.len() - SHOWN);
    }
}

fn render_dependencies(out: &mut String, brief: &Brief) {
    if brief.repo.dependencies.is_empty() {
        return;
    }
    out.push_str("## Dependencies\n");
    for list in &brief.repo.dependencies {
        let mut line = format!("- `{}`: {}", list.manifest, join_or_none(&list.runtime));
        if !list.dev.is_empty() {
            let _ = write!(line, "; dev: {}", list.dev.join(", "));
        }
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn render_memory(out: &mut String, brief: &Brief) {
    let Some(memory) = &brief.memory else {
        return;
    };
    let reviewed = memory
        .refreshed_at
        .as_deref()
        .map(|value| format!(", reviewed {}", value.split('T').next().unwrap_or(value)))
        .unwrap_or_default();
    let _ = writeln!(out, "## Repo memory (`{}`{reviewed})", memory.path);
    if memory.notes.is_empty() {
        out.push_str("- no notes yet: add durable repo facts here as you learn them\n\n");
        return;
    }
    out.push_str(&demote_headings(&memory.notes));
    if memory.truncated {
        out.push_str("\n- … truncated, read the file for the rest");
    }
    out.push_str("\n\n");
}

/// Keep embedded markdown from breaking the briefing's own heading levels.
fn demote_headings(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with('#') {
                format!("###{line}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_excerpts(out: &mut String, brief: &Brief) {
    if brief.excerpts.is_empty() {
        return;
    }
    out.push_str("## Excerpts\n");
    for excerpt in &brief.excerpts {
        let mut flags = Vec::new();
        if excerpt.truncated {
            flags.push("truncated");
        }
        if excerpt.redacted {
            flags.push("secrets redacted");
        }
        let flags = if flags.is_empty() {
            String::new()
        } else {
            format!(" ({})", flags.join(", "))
        };
        let fence = if excerpt.content.contains("```") {
            "````"
        } else {
            "```"
        };
        let _ = writeln!(
            out,
            "### `{}`{flags}\n{fence}\n{}\n{fence}\n",
            excerpt.path, excerpt.content
        );
    }
}
