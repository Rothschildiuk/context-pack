mod briefing;
mod cli;
mod dependency_summary;
mod detect;
mod diff;
mod docker_summary;
mod git;
mod ignore;
mod mcp;
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

    if let (Some(from), Some(to)) = (&config.diff_from, &config.diff_to) {
        let output = diff::render_diff_from_files(from, to)?;
        match &config.output {
            Some(path) => {
                std::fs::write(path, output).map_err(|source| CliError::Io {
                    action: "write output",
                    path: path.clone(),
                    source,
                })?;
            }
            None => {
                print!("{output}");
            }
        }
        return Ok(());
    }

    if config.mcp_server {
        return mcp::serve();
    }

    if config.init_memory {
        let message = init_memory_template(&config)?;
        println!("{message}");
        return Ok(());
    }
    if config.refresh_memory {
        let message = refresh_memory_template(&config)?;
        println!("{message}");
        return Ok(());
    }

    let output = render_bundle(&config);

    match &config.output {
        Some(path) => {
            std::fs::write(path, output).map_err(|source| CliError::Io {
                action: "write output",
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

pub(crate) fn init_memory_template(config: &AppConfig) -> Result<String, CliError> {
    write_memory_template(config, false)
}

pub(crate) fn refresh_memory_template(config: &AppConfig) -> Result<String, CliError> {
    write_memory_template(config, true)
}

pub(crate) fn write_memory_template(
    config: &AppConfig,
    overwrite: bool,
) -> Result<String, CliError> {
    let memory_dir = config.cwd.join(".context-pack");
    let memory_path = memory_dir.join("memory.md");

    if memory_path.exists() && !overwrite {
        return Err(CliError::MemoryExists(memory_path));
    }

    std::fs::create_dir_all(&memory_dir).map_err(|source| CliError::Io {
        action: "create memory directory",
        path: memory_dir.clone(),
        source,
    })?;

    let context = build_context(config);
    let content = memory_template(&context);
    std::fs::write(&memory_path, content).map_err(|source| CliError::Io {
        action: "write memory template",
        path: memory_path.clone(),
        source,
    })?;

    Ok(if overwrite {
        format!("Updated {}", memory_path.display())
    } else {
        format!("Created {}", memory_path.display())
    })
}

pub(crate) fn render_bundle(config: &AppConfig) -> String {
    let mut context = build_context(config);
    let initial = match config.format {
        OutputFormat::Markdown => render_markdown::render(&context),
        OutputFormat::Json => render_json::render(&context),
    };
    let token_estimate = rough_token_estimate(&initial);
    context
        .notes
        .insert(1, format!("approx tokens: {}", token_estimate));

    match config.format {
        OutputFormat::Markdown => render_markdown::render(&context),
        OutputFormat::Json => render_json::render(&context),
    }
}

pub(crate) fn build_context(config: &AppConfig) -> RenderContext {
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
    if let Some(profile) = &config.profile {
        notes.push(format!("profile: {profile}"));
    }
    if !config.language_aware {
        notes.push("language-aware scoring disabled".to_string());
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

fn rough_token_estimate(text: &str) -> usize {
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    let char_based = chars.div_ceil(4);
    char_based.max(words)
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

fn memory_template(context: &RenderContext) -> String {
    let repo_name = context
        .repo
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repository");

    let mut output = String::new();
    output.push_str("# Learned Repo Memory\n\n");
    output.push_str("## Repo\n");
    output.push_str(&format!("- name: {repo_name}\n"));

    if let Some(summary) = context.briefing.repo_summary.first() {
        output.push_str(&format!("- purpose: {summary}\n"));
    } else {
        output.push_str("- purpose: \n");
    }

    if !context.repo.project_types.is_empty() {
        output.push_str(&format!(
            "- project types: {}\n",
            context.repo.project_types.join(", ")
        ));
    }

    if !context.repo.primary_languages.is_empty() {
        output.push_str(&format!(
            "- primary languages: {}\n",
            context.repo.primary_languages.join(", ")
        ));
    }

    output.push('\n');
    push_briefing_items_section(
        &mut output,
        "## Read First",
        &context.briefing.read_these_first,
    );
    push_briefing_items_section(
        &mut output,
        "## Entry Points",
        &context.briefing.likely_entry_points,
    );
    push_hotspots_section(&mut output, context);
    push_string_section(&mut output, "## Known Pitfalls", &context.briefing.caveats);
    push_string_section(
        &mut output,
        "## Operational Notes",
        &context.briefing.dependency_summary,
    );
    push_string_section(
        &mut output,
        "## Debugging Notes",
        &context.briefing.active_work,
    );
    output.push_str(
        "## Open Questions\n- Fill this in as you learn where the repo still fights back.\n",
    );
    output
}

fn push_briefing_items_section(output: &mut String, title: &str, items: &[model::BriefingItem]) {
    output.push_str(title);
    output.push('\n');

    if items.is_empty() {
        output.push_str("- none yet\n\n");
        return;
    }

    for item in items.iter().take(5) {
        output.push_str(&format!("- `{}`: {}\n", item.path.display(), item.reason));
    }
    output.push('\n');
}

fn push_hotspots_section(output: &mut String, context: &RenderContext) {
    output.push_str("## Hotspots\n");

    let mut entries = Vec::new();

    for item in &context.briefing.likely_entry_points {
        entries.push((item.path.clone(), item.reason.clone(), 100usize));
    }

    for file in &context.important_files {
        let priority = match file.category {
            model::SignalCategory::ChangedSource => 95,
            model::SignalCategory::EntryPoint => 90,
            model::SignalCategory::IncludedSource => 80,
            model::SignalCategory::Manifest => 30,
            model::SignalCategory::Build => 20,
            _ => {
                if is_production_source_path(&file.path) {
                    70
                } else {
                    0
                }
            }
        };

        if priority > 0 {
            entries.push((file.path.clone(), file.reason.clone(), priority));
        }
    }

    for file in &context.briefing.large_code_files {
        entries.push((file.path.clone(), file.reason.clone(), 85));
    }

    entries.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| {
                left.0
                    .components()
                    .count()
                    .cmp(&right.0.components().count())
            })
            .then_with(|| left.0.cmp(&right.0))
    });

    entries.dedup_by(|left, right| left.0 == right.0);

    if entries.is_empty() {
        output.push_str("- none yet\n");
    } else {
        for (path, reason, _) in entries.into_iter().take(5) {
            output.push_str(&format!("- `{}`: {}\n", path.display(), reason));
        }
    }

    output.push('\n');
}

fn is_production_source_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "go" | "py" | "ts" | "tsx" | "js" | "jsx" | "java" | "kt")
    ) && !path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        matches!(
            value.as_str(),
            "tests"
                | "test"
                | "__tests__"
                | "fixtures"
                | "fixture"
                | "docs"
                | "doc"
                | "examples"
                | "example"
                | "samples"
                | "sample"
        ) || value.contains("vendor")
    })
}

fn push_string_section(output: &mut String, title: &str, items: &[String]) {
    output.push_str(title);
    output.push('\n');

    if items.is_empty() {
        output.push_str("- none yet\n\n");
        return;
    }

    for item in items.iter().take(5) {
        output.push_str(&format!("- {item}\n"));
    }
    output.push('\n');
}
