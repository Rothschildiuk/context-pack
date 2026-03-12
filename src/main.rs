mod briefing;
mod cli;
mod git;
mod ignore;
mod model;
mod render_markdown;
mod select;
mod walk;

use cli::{parse_args, CliError};
use model::{AppConfig, OutputBudgets, OutputFormat, RenderContext, RepoInfo};
use select::select_files;
use walk::build_tree_summary;

fn main() {
    if let Err(err) = run() {
        match err {
            CliError::Help(text) => {
                println!("{text}");
                std::process::exit(0);
            }
            other => {
                eprintln!("{other}");
                std::process::exit(1);
            }
        }
    }
}

fn run() -> Result<(), CliError> {
    let config = parse_args(std::env::args().skip(1))?;
    let output = render_bundle(&config);

    match &config.output {
        Some(path) => {
            std::fs::write(path, output).map_err(|source| CliError::Io {
                path: path.clone(),
                source,
            })?;
        }
        None => {
            print!("{output}");
        }
    }

    Ok(())
}

fn render_bundle(config: &AppConfig) -> String {
    match config.format {
        OutputFormat::Markdown => {
            let budgets = split_budgets(config.max_bytes);
            let walk_result = build_tree_summary(config, budgets.tree);
            let git_result = git::collect(config, budgets.git);
            let selection_result =
                select_files(config, &git_result.changed_files, budgets.excerpts);
            let repo = RepoInfo {
                path: config.cwd.clone(),
                project_types: detect_project_types(&selection_result.files),
                primary_languages: detect_languages(&selection_result.files),
            };
            let briefing = briefing::build(
                config,
                &repo,
                &selection_result.files,
                &git_result,
                &walk_result,
                budgets.briefing,
            );

            let context = RenderContext {
                briefing,
                repo,
                tree_summary: walk_result.tree_summary,
                important_files: selection_result.files,
                git_summary: git_result.summary,
                notes: build_notes(
                    config,
                    budgets,
                    walk_result.notes,
                    git_result.notes,
                    selection_result.notes,
                ),
            };

            render_markdown::render(&context)
        }
        OutputFormat::Json => format!(
            concat!(
                "{{\n",
                "  \"status\": \"not_implemented\",\n",
                "  \"message\": \"JSON output is not implemented in the first vertical slice.\",\n",
                "  \"cwd\": \"{}\"\n",
                "}}\n"
            ),
            escape_json_string(config.cwd.to_string_lossy().as_ref())
        ),
    }
}

fn build_notes(
    config: &AppConfig,
    budgets: OutputBudgets,
    walk_notes: Vec<String>,
    git_notes: Vec<String>,
    selection_notes: Vec<String>,
) -> Vec<String> {
    let mut notes = Vec::new();
    notes.push(format!("max bytes: {}", config.max_bytes));
    notes.push(format!("max files: {}", config.max_files));
    notes.push(format!("max depth: {}", config.max_depth));
    notes.push(format!(
        "budget split: briefing={}, git={}, excerpts={}, tree={}",
        budgets.briefing, budgets.git, budgets.excerpts, budgets.tree
    ));

    if config.changed_only {
        notes.push("changed-only mode enabled".to_string());
    }

    if config.no_tree {
        notes.push("tree output disabled".to_string());
    }

    if !config.include.is_empty() {
        notes.push(format!("include globs: {}", config.include.join(", ")));
    }

    if !config.exclude.is_empty() {
        notes.push(format!("exclude globs: {}", config.exclude.join(", ")));
    }

    notes.extend(walk_notes);
    notes.extend(git_notes);
    notes.extend(selection_notes);
    notes
}

fn split_budgets(max_bytes: usize) -> OutputBudgets {
    let total = max_bytes.max(600);
    let briefing = (total / 4).clamp(260, 900);
    let git = (total / 8).clamp(120, 500);
    let tree = (total / 5).clamp(120, 900);
    let reserved = briefing + git + tree;
    let excerpts = total.saturating_sub(reserved).max(240);

    OutputBudgets {
        briefing,
        git,
        excerpts,
        tree,
    }
}

fn detect_project_types(files: &[model::ImportantFile]) -> Vec<String> {
    let mut types = Vec::new();

    if has_file(files, "Cargo.toml") {
        types.push("rust".to_string());
    }
    if has_file(files, "package.json") {
        types.push("node".to_string());
    }
    if has_file(files, "pyproject.toml") || has_file(files, "requirements.txt") {
        types.push("python".to_string());
    }
    if has_file(files, "go.mod") {
        types.push("go".to_string());
    }

    if types.is_empty() {
        types.push("unknown".to_string());
    }

    types
}

fn detect_languages(files: &[model::ImportantFile]) -> Vec<String> {
    let mut languages = Vec::new();

    if has_file(files, "Cargo.toml") || has_extension(files, "rs") {
        languages.push("rust".to_string());
    }
    if has_file(files, "go.mod") || has_extension(files, "go") {
        languages.push("go".to_string());
    }
    if has_file(files, "pyproject.toml") || has_extension(files, "py") {
        languages.push("python".to_string());
    }
    if has_file(files, "package.json") || has_extension(files, "ts") || has_extension(files, "tsx")
    {
        languages.push("typescript".to_string());
    }
    if has_extension(files, "js") || has_extension(files, "jsx") {
        languages.push("javascript".to_string());
    }

    languages
}

fn has_file(files: &[model::ImportantFile], name: &str) -> bool {
    files
        .iter()
        .any(|file| file.path.file_name().and_then(|value| value.to_str()) == Some(name))
}

fn has_extension(files: &[model::ImportantFile], extension: &str) -> bool {
    files
        .iter()
        .any(|file| file.path.extension().and_then(|value| value.to_str()) == Some(extension))
}

fn escape_json_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }

    escaped
}
