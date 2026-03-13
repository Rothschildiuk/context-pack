mod briefing;
mod cli;
mod dependency_summary;
mod detect;
mod docker_summary;
mod git;
mod ignore;
mod model;
mod render_json;
mod render_markdown;
mod select;
mod walk;

use cli::{parse_args, CliError};
use ignore::IgnoreMatcher;
use model::{AppConfig, OutputBudgets, OutputFormat, RenderContext};
use select::scan_repo_signals;
use walk::build_tree_summary_with_matcher;

fn main() {
    if let Err(err) = run() {
        match err {
            CliError::Help(text) | CliError::Version(text) => {
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

    if config.init_memory {
        return init_memory_template(&config);
    }

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

fn init_memory_template(config: &AppConfig) -> Result<(), CliError> {
    let memory_dir = config.cwd.join(".context-pack");
    let memory_path = memory_dir.join("memory.md");

    if memory_path.exists() {
        return Err(CliError::MemoryExists(memory_path));
    }

    std::fs::create_dir_all(&memory_dir).map_err(|source| CliError::Io {
        path: memory_dir.clone(),
        source,
    })?;

    std::fs::write(&memory_path, memory_template(&config.cwd)).map_err(|source| CliError::Io {
        path: memory_path.clone(),
        source,
    })?;

    println!("Created {}", memory_path.display());
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
    let matcher = IgnoreMatcher::load(&config.cwd, config);
    let walk_result = build_tree_summary_with_matcher(config, &matcher, budgets.tree);
    let git_result = git::collect(config, budgets.git);
    let signals = scan_repo_signals(
        config,
        &matcher,
        &git_result.changed_files,
        budgets.excerpts,
    );
    let selection = signals.selection;
    let large_code_files = signals.large_code_files;
    let repo = detect::detect_repo_info_with_matcher(config, &selection.files, &matcher);
    let docker_summary = docker_summary::collect(config, &selection.files, 500);
    let dependency_summary = dependency_summary::collect(config, &selection.files, 500);
    let briefing = briefing::build(
        config,
        &repo,
        &selection.files,
        &large_code_files,
        &docker_summary,
        &dependency_summary,
        &git_result,
        &walk_result,
        budgets.briefing,
    );

    RenderContext {
        briefing,
        repo,
        tree_summary: walk_result.tree_summary,
        important_files: selection.files,
        git_available: git_result.available,
        git_branch_context: git_result.branch_context,
        git_changes: git_result.changes,
        git_summary: git_result.summary,
        notes: build_notes(
            config,
            budgets,
            walk_result.notes,
            git_result.notes,
            selection.notes,
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

fn memory_template(cwd: &std::path::Path) -> String {
    let repo_name = cwd
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repository");

    format!(
        "# Learned Repo Memory\n\n## Repo\n- name: {repo_name}\n- purpose:\n\n## Entry Points\n- \n\n## Known Pitfalls\n- \n\n## Operational Notes\n- \n\n## Debugging Notes\n- \n\n## Open Questions\n- \n"
    )
}
