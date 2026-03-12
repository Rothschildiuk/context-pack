use crate::model::{ImportantFile, RenderContext};

pub fn render(context: &RenderContext) -> String {
    let mut output = String::new();

    output.push_str("# Context Pack\n\n");
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

    output.push_str("## Tree\n");
    output.push_str(&context.tree_summary);
    output.push_str("\n\n");

    output.push_str("## Important Files\n");
    if context.important_files.is_empty() {
        output.push_str("_No files selected yet._\n\n");
    } else {
        for file in &context.important_files {
            render_important_file(&mut output, file);
        }
    }

    output.push_str("## Git Changes\n");
    output.push_str(&context.git_summary);
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

fn render_important_file(output: &mut String, file: &ImportantFile) {
    output.push_str(&format!("### {}\n", file.path.display()));
    output.push_str(&format!("- reason: {}\n", file.reason));
    output.push_str(&format!("- truncated: {}\n\n", file.truncated));
    output.push_str("```text\n");
    output.push_str(&file.excerpt);
    if !file.excerpt.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("```\n\n");
}

fn render_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}
