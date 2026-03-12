mod briefing;
mod cli;
mod detect;
mod git;
mod ignore;
mod model;
mod render_json;
mod render_markdown;
mod select;
mod walk;

use cli::{parse_args, CliError};
use model::{AppConfig, OutputBudgets, OutputFormat, RenderContext};
use select::{collect_large_code_files, select_files};
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
    let context = build_context(config);

    match config.format {
        OutputFormat::Markdown => render_markdown::render(&context),
        OutputFormat::Json => render_json::render(&context),
    }
}

fn build_context(config: &AppConfig) -> RenderContext {
    let budgets = split_budgets(config.max_bytes);
    let walk_result = build_tree_summary(config, budgets.tree);
    let git_result = git::collect(config, budgets.git);
    let selection_result = select_files(config, &git_result.changed_files, budgets.excerpts);
    let large_code_files = collect_large_code_files(config, &git_result.changed_files);
    let repo = detect::detect_repo_info(config, &selection_result.files);
    let briefing = briefing::build(
        config,
        &repo,
        &selection_result.files,
        &large_code_files,
        &git_result,
        &walk_result,
        budgets.briefing,
    );

    RenderContext {
        briefing,
        repo,
        tree_summary: walk_result.tree_summary,
        important_files: selection_result.files,
        git_available: git_result.available,
        git_changes: git_result.changes,
        git_summary: git_result.summary,
        notes: build_notes(
            config,
            budgets,
            walk_result.notes,
            git_result.notes,
            selection_result.notes,
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
