use crate::model::{AgentBriefing, BriefingItem, ImportantFile, LargeCodeFile, RenderContext};

pub fn render(context: &RenderContext) -> String {
    let mut output = String::new();

    output.push_str("# Context Pack\n\n");
    render_briefing(&mut output, &context.briefing);

    output.push_str("## Repo\n");
    output.push_str(&format!("- path: {}\n", context.repo.path.display()));
    output.push_str(&format!(
        "- project types: {}\n",
        render_list(&context.repo.project_types)
    ));
    output.push_str(&format!(
        "- primary languages: {}\n\n",
        render_list(&context.repo.primary_languages)
    ));

    output.push_str("## Git Changes\n");
    render_git_branch_context(&mut output, context);
    output.push_str(&context.git_summary);
    output.push_str("\n\n");

    output.push_str("## Important Files\n");
    if context.important_files.is_empty() {
        output.push_str("_No files selected yet._\n\n");
    } else {
        for file in &context.important_files {
            render_important_file(&mut output, file);
        }
    }

    output.push_str("## Tree\n");
    output.push_str(&context.tree_summary);
    output.push_str("\n\n");

    output.push_str("## Notes\n");
    if context.notes.is_empty() {
        output.push_str("- none\n");
    } else {
        for note in &context.notes {
            output.push_str(&format!("- {note}\n"));
        }
    }

    output
}

fn render_git_branch_context(output: &mut String, context: &RenderContext) {
    if !context.git_available {
        return;
    }

    let branch = &context.git_branch_context;
    let mut has_lines = false;

    if let Some(current) = &branch.current_branch {
        output.push_str(&format!("- current branch: `{current}`\n"));
        has_lines = true;
    }
    if !branch.local_branches.is_empty() {
        output.push_str(&format!(
            "- local branches: {}\n",
            render_branch_list(&branch.local_branches)
        ));
        has_lines = true;
    }
    if let Some(upstream) = &branch.upstream_branch {
        output.push_str(&format!("- upstream branch: `{upstream}`\n"));
        has_lines = true;
    }
    if let Some(default_branch) = &branch.default_branch {
        output.push_str(&format!("- default branch: `{default_branch}`\n"));
        output.push_str(&format!(
            "- primary development branch likely `{default_branch}`\n"
        ));
        has_lines = true;
    }
    if let Some(target) = &branch.comparison_target {
        output.push_str(&format!(
            "- relative to `{target}`: ahead {}, behind {}\n",
            branch.ahead, branch.behind
        ));
        has_lines = true;
    }

    if has_lines {
        output.push('\n');
    }
}

fn render_briefing(output: &mut String, briefing: &AgentBriefing) {
    output.push_str("## Agent Briefing\n");
    render_bullet_block(output, "### What This Repo Is", &briefing.repo_summary);
    render_bullet_block(output, "### Active Work", &briefing.active_work);
    render_briefing_items(output, "### Read These First", &briefing.read_these_first);
    render_briefing_items(
        output,
        "### Likely Entry Points",
        &briefing.likely_entry_points,
    );
    render_optional_bullet_block(output, "### Docker Summary", &briefing.docker_summary);
    render_optional_bullet_block(
        output,
        "### Dependency Summary",
        &briefing.dependency_summary,
    );
    render_large_code_files(output, "### Large Code Files", &briefing.large_code_files);
    render_bullet_block(output, "### Caveats", &briefing.caveats);
}

fn render_important_file(output: &mut String, file: &ImportantFile) {
    output.push_str(&format!("### {}\n", file.path.display()));
    output.push_str(&format!("- reason: {}\n", file.reason));
    if !file.why.is_empty() {
        output.push_str(&format!("- why: {}\n", file.why.join(", ")));
    }
    output.push_str(&format!("- category: {}\n", file.category.label()));
    output.push_str(&format!("- score: {}\n", file.score));
    output.push_str(&format!("- truncated: {}\n", file.truncated));
    if file.redacted {
        output.push_str("- redacted: true\n");
        if let Some(reason) = &file.redaction_reason {
            output.push_str(&format!("- redaction reason: {reason}\n"));
        }
    }
    output.push('\n');
    output.push_str("```text\n");
    output.push_str(&file.excerpt);
    if !file.excerpt.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n\n");
}

fn render_bullet_block(output: &mut String, title: &str, items: &[String]) {
    output.push_str(title);
    output.push('\n');
    if items.is_empty() {
        output.push_str("- none\n\n");
        return;
    }

    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
    output.push('\n');
}

fn render_optional_bullet_block(output: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }

    render_bullet_block(output, title, items);
}

fn render_briefing_items(output: &mut String, title: &str, items: &[BriefingItem]) {
    output.push_str(title);
    output.push('\n');
    if items.is_empty() {
        output.push_str("- none\n\n");
        return;
    }

    for item in items {
        output.push_str(&format!("- `{}`: {}\n", item.path.display(), item.reason));
    }
    output.push('\n');
}

fn render_large_code_files(output: &mut String, title: &str, items: &[LargeCodeFile]) {
    output.push_str(title);
    output.push('\n');
    if items.is_empty() {
        output.push_str("- none\n\n");
        return;
    }

    for item in items {
        output.push_str(&format!(
            "- `{}` ({}) : {}\n",
            item.path.display(),
            format!("{} LOC", item.loc),
            item.reason
        ));
    }
    output.push('\n');
}

fn render_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn render_branch_list(values: &[String]) -> String {
    let visible = values
        .iter()
        .take(4)
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>();

    if values.len() > visible.len() {
        format!(
            "{}, +{} more",
            visible.join(", "),
            values.len() - visible.len()
        )
    } else {
        visible.join(", ")
    }
}
