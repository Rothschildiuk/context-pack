use crate::model::{
    AgentBriefing, BriefingItem, GitChange, ImportantFile, LargeCodeFile, RenderContext,
};

pub fn render(context: &RenderContext) -> String {
    let mut output = String::new();
    output.push_str("{\n");
    render_briefing(&mut output, &context.briefing);
    output.push_str(",\n");
    render_repo(&mut output, context);
    output.push_str(",\n");
    render_git(&mut output, context);
    output.push_str(",\n");
    render_important_files(&mut output, &context.important_files);
    output.push_str(",\n");
    push_string_field(&mut output, 1, "tree_summary", &context.tree_summary, true);
    render_string_array_field(&mut output, 1, "notes", &context.notes, false);
    output.push_str("\n}\n");
    output
}

fn render_briefing(output: &mut String, briefing: &AgentBriefing) {
    push_indent(output, 1);
    output.push_str("\"briefing\": {\n");
    render_string_array_field(output, 2, "repo_summary", &briefing.repo_summary, true);
    render_string_array_field(output, 2, "active_work", &briefing.active_work, true);
    render_briefing_items_field(
        output,
        2,
        "read_these_first",
        &briefing.read_these_first,
        true,
    );
    render_briefing_items_field(
        output,
        2,
        "likely_entry_points",
        &briefing.likely_entry_points,
        true,
    );
    render_string_array_field(output, 2, "docker_summary", &briefing.docker_summary, true);
    render_string_array_field(
        output,
        2,
        "dependency_summary",
        &briefing.dependency_summary,
        true,
    );
    render_large_code_files_field(
        output,
        2,
        "large_code_files",
        &briefing.large_code_files,
        true,
    );
    render_string_array_field(output, 2, "caveats", &briefing.caveats, false);
    output.push('\n');
    push_indent(output, 1);
    output.push('}');
}

fn render_repo(output: &mut String, context: &RenderContext) {
    push_indent(output, 1);
    output.push_str("\"repo\": {\n");
    push_string_field(
        output,
        2,
        "path",
        context.repo.path.to_string_lossy().as_ref(),
        true,
    );
    render_string_array_field(
        output,
        2,
        "project_types",
        &context.repo.project_types,
        true,
    );
    render_string_array_field(
        output,
        2,
        "primary_languages",
        &context.repo.primary_languages,
        false,
    );
    output.push('\n');
    push_indent(output, 1);
    output.push('}');
}

fn render_git(output: &mut String, context: &RenderContext) {
    push_indent(output, 1);
    output.push_str("\"git\": {\n");
    push_bool_field(output, 2, "available", context.git_available, true);
    render_git_branch_context(output, 2, context, true);
    push_string_field(output, 2, "summary", &context.git_summary, true);
    render_git_changes_field(output, 2, "changes", &context.git_changes, false);
    output.push('\n');
    push_indent(output, 1);
    output.push('}');
}

fn render_git_branch_context(
    output: &mut String,
    indent: usize,
    context: &RenderContext,
    trailing_comma: bool,
) {
    push_indent(output, indent);
    output.push_str("\"branch_context\": {\n");
    push_optional_string_field(
        output,
        indent + 1,
        "current_branch",
        context.git_branch_context.current_branch.as_deref(),
        true,
    );
    render_string_array_field(
        output,
        indent + 1,
        "local_branches",
        &context.git_branch_context.local_branches,
        true,
    );
    push_optional_string_field(
        output,
        indent + 1,
        "upstream_branch",
        context.git_branch_context.upstream_branch.as_deref(),
        true,
    );
    push_optional_string_field(
        output,
        indent + 1,
        "default_branch",
        context.git_branch_context.default_branch.as_deref(),
        true,
    );
    push_optional_string_field(
        output,
        indent + 1,
        "comparison_target",
        context.git_branch_context.comparison_target.as_deref(),
        true,
    );
    push_number_field(
        output,
        indent + 1,
        "ahead",
        context.git_branch_context.ahead,
        true,
    );
    push_number_field(
        output,
        indent + 1,
        "behind",
        context.git_branch_context.behind,
        false,
    );
    output.push('\n');
    push_indent(output, indent);
    output.push('}');
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn render_important_files(output: &mut String, files: &[ImportantFile]) {
    push_indent(output, 1);
    output.push_str("\"important_files\": [");

    if files.is_empty() {
        output.push(']');
        return;
    }

    output.push('\n');
    for (index, file) in files.iter().enumerate() {
        push_indent(output, 2);
        output.push_str("{\n");
        push_string_field(output, 3, "path", &file.path.display().to_string(), true);
        push_string_field(output, 3, "reason", &file.reason, true);
        push_string_field(output, 3, "category", file.category.label(), true);
        push_number_field(output, 3, "score", file.score, true);
        push_bool_field(output, 3, "truncated", file.truncated, true);
        push_string_field(output, 3, "excerpt", &file.excerpt, false);
        output.push('\n');
        push_indent(output, 2);
        output.push('}');
        if index + 1 != files.len() {
            output.push(',');
        }
        output.push('\n');
    }
    push_indent(output, 1);
    output.push(']');
}

fn render_briefing_items_field(
    output: &mut String,
    indent: usize,
    name: &str,
    items: &[BriefingItem],
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": [");

    if items.is_empty() {
        output.push(']');
        if trailing_comma {
            output.push(',');
        }
        output.push('\n');
        return;
    }

    output.push('\n');
    for (index, item) in items.iter().enumerate() {
        push_indent(output, indent + 1);
        output.push_str("{\n");
        push_string_field(
            output,
            indent + 2,
            "path",
            &item.path.display().to_string(),
            true,
        );
        push_string_field(output, indent + 2, "reason", &item.reason, false);
        output.push('\n');
        push_indent(output, indent + 1);
        output.push('}');
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    push_indent(output, indent);
    output.push(']');
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn render_large_code_files_field(
    output: &mut String,
    indent: usize,
    name: &str,
    items: &[LargeCodeFile],
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": [");

    if items.is_empty() {
        output.push(']');
        if trailing_comma {
            output.push(',');
        }
        output.push('\n');
        return;
    }

    output.push('\n');
    for (index, item) in items.iter().enumerate() {
        push_indent(output, indent + 1);
        output.push_str("{\n");
        push_string_field(
            output,
            indent + 2,
            "path",
            &item.path.display().to_string(),
            true,
        );
        push_number_field(output, indent + 2, "loc", item.loc, true);
        push_string_field(output, indent + 2, "reason", &item.reason, false);
        output.push('\n');
        push_indent(output, indent + 1);
        output.push('}');
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    push_indent(output, indent);
    output.push(']');
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn render_git_changes_field(
    output: &mut String,
    indent: usize,
    name: &str,
    changes: &[GitChange],
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": [");

    if changes.is_empty() {
        output.push(']');
        if trailing_comma {
            output.push(',');
        }
        output.push('\n');
        return;
    }

    output.push('\n');
    for (index, change) in changes.iter().enumerate() {
        push_indent(output, indent + 1);
        output.push_str("{\n");
        push_string_field(
            output,
            indent + 2,
            "path",
            &change.path.display().to_string(),
            true,
        );
        push_string_field(output, indent + 2, "kind", &change.kind, false);
        output.push('\n');
        push_indent(output, indent + 1);
        output.push('}');
        if index + 1 != changes.len() {
            output.push(',');
        }
        output.push('\n');
    }
    push_indent(output, indent);
    output.push(']');
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn render_string_array_field(
    output: &mut String,
    indent: usize,
    name: &str,
    items: &[String],
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": [");

    if items.is_empty() {
        output.push(']');
        if trailing_comma {
            output.push(',');
        }
        output.push('\n');
        return;
    }

    output.push('\n');
    for (index, item) in items.iter().enumerate() {
        push_indent(output, indent + 1);
        write_json_string(output, item);
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    push_indent(output, indent);
    output.push(']');
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn push_string_field(
    output: &mut String,
    indent: usize,
    name: &str,
    value: &str,
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": ");
    write_json_string(output, value);
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn push_optional_string_field(
    output: &mut String,
    indent: usize,
    name: &str,
    value: Option<&str>,
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(": ");
    match value {
        Some(value) => write_json_string(output, value),
        None => output.push_str("null"),
    }
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn push_number_field(
    output: &mut String,
    indent: usize,
    name: &str,
    value: usize,
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(&format!(": {value}"));
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn push_bool_field(
    output: &mut String,
    indent: usize,
    name: &str,
    value: bool,
    trailing_comma: bool,
) {
    push_indent(output, indent);
    write_json_string(output, name);
    output.push_str(if value { ": true" } else { ": false" });
    if trailing_comma {
        output.push(',');
    }
    output.push('\n');
}

fn push_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push_str("  ");
    }
}

fn write_json_string(output: &mut String, value: &str) {
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
}
