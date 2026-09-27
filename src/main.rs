mod briefing;
mod cli;
mod commands;
mod excerpt;
mod git;
mod index;
mod layout;
mod manifest;
mod mcp;
mod memory;
mod model;
mod paths;
mod render_json;
mod render_markdown;
mod select;

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use cli::{parse_args, CliError};
use model::{AppConfig, Command, OutputFormat};

const CONTEXT_DIR: &str = ".context-pack";

fn main() {
    let result = parse_args(std::env::args().skip(1)).and_then(|config| run(&config));
    match result {
        Ok(()) => {}
        Err(CliError::Help(text) | CliError::Version(text)) => println!("{text}"),
        Err(error) => {
            eprintln!("context-pack: {error}");
            std::process::exit(1);
        }
    }
}

fn run(config: &AppConfig) -> Result<(), CliError> {
    let message = match config.command {
        Command::McpServer => return mcp::serve(),
        Command::InitMemory => init_memory(config)?,
        Command::RefreshMemory => refresh_memory(config)?,
        Command::AddMemoryNote => add_memory_note(config)?,
        Command::RefreshContext => refresh_context(config)?,
        Command::CheckContext => check_context(config)?,
        Command::Brief => {
            let output = render(config);
            return match &config.output {
                Some(path) => write_file(path, &output),
                None => {
                    print!("{output}");
                    Ok(())
                }
            };
        }
    };
    println!("{message}");
    Ok(())
}

pub(crate) fn render(config: &AppConfig) -> String {
    let brief = briefing::build(config);
    match config.format {
        OutputFormat::Markdown => render_markdown::render(&brief),
        OutputFormat::Json => render_json::render(&brief),
    }
}

pub(crate) fn init_memory(config: &AppConfig) -> Result<String, CliError> {
    let path = config.cwd.join(memory::MEMORY_PATH);
    if path.exists() {
        return Err(CliError::MemoryExists(path));
    }
    write_file(&path, &memory::template(&repo_name(&config.cwd)))?;
    Ok(format!("Created {}", path.display()))
}

pub(crate) fn refresh_memory(config: &AppConfig) -> Result<String, CliError> {
    let path = config.cwd.join(memory::MEMORY_PATH);
    if !path.exists() {
        return init_memory(config);
    }
    let content = read_file(&path)?;
    write_file(&path, &memory::refresh(&content))?;
    Ok(format!(
        "Marked {} as reviewed (notes unchanged)",
        path.display()
    ))
}

fn add_memory_note(config: &AppConfig) -> Result<String, CliError> {
    let path = config.cwd.join(memory::MEMORY_PATH);
    if !path.exists() {
        init_memory(config)?;
    }
    let note = config.note.as_deref().unwrap_or_default();
    write_file(&path, &memory::add_note(&read_file(&path)?, note))?;
    Ok(format!("Added note to {}", path.display()))
}

pub(crate) fn refresh_context(config: &AppConfig) -> Result<String, CliError> {
    let memory_path = config.cwd.join(memory::MEMORY_PATH);
    let mut messages = Vec::new();
    if !memory_path.exists() {
        messages.push(init_memory(config)?);
    }

    let dir = config.cwd.join(CONTEXT_DIR);
    for (format, name) in [
        (OutputFormat::Markdown, "PROJECT_CONTEXT.md"),
        (OutputFormat::Json, "PROJECT_CONTEXT.json"),
    ] {
        let mut artifact_config = config.clone();
        artifact_config.format = format;
        let path = dir.join(name);
        write_file(&path, &render(&artifact_config))?;
        messages.push(format!("Updated {}", path.display()));
    }
    Ok(messages.join("\n"))
}

pub(crate) fn check_context(config: &AppConfig) -> Result<String, CliError> {
    let dir = config.cwd.join(CONTEXT_DIR);
    let markdown_path = dir.join("PROJECT_CONTEXT.md");
    let json_path = dir.join("PROJECT_CONTEXT.json");
    let memory_path = dir.join("memory.md");

    let markdown = read_file(&markdown_path)?;
    if !markdown.contains("— context pack") {
        return Err(invalid(
            &markdown_path,
            "missing context pack header; run `context-pack context refresh`",
        ));
    }

    let payload: serde_json::Value = serde_json::from_str(&read_file(&json_path)?)
        .map_err(|error| invalid(&json_path, &error.to_string()))?;
    if payload
        .get("schema_version")
        .and_then(|value| value.as_str())
        != Some(model::SCHEMA_VERSION)
    {
        return Err(invalid(
            &json_path,
            "schema version is outdated; run `context-pack context refresh`",
        ));
    }

    let memory = read_file(&memory_path)?;
    if let Some(missing) = memory::REQUIRED_METADATA
        .iter()
        .find(|field| !memory.contains(*field))
    {
        return Err(invalid(
            &memory_path,
            &format!("missing metadata '{}'", missing.trim()),
        ));
    }

    let generated_from = payload
        .pointer("/stats/generated_from_commit")
        .and_then(|value| value.as_str());
    if let (Some(generated_from), Some(head)) = (generated_from, current_head(&config.cwd)) {
        if generated_from != head {
            return Err(invalid(
                &json_path,
                &format!("generated from {generated_from} but HEAD is {head}; run `context-pack context refresh`"),
            ));
        }
    }
    Ok("Context artifacts are present and match HEAD".to_string())
}

fn current_head(cwd: &Path) -> Option<String> {
    let output = Process::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|head| !head.is_empty())
}

fn repo_name(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repository")
        .to_string()
}

fn invalid(path: &Path, reason: &str) -> CliError {
    CliError::InvalidArtifact(format!(
        "invalid context artifact '{}': {reason}",
        path.display()
    ))
}

fn read_file(path: &Path) -> Result<String, CliError> {
    std::fs::read_to_string(path).map_err(|source| CliError::Io {
        action: "read",
        path: path.to_path_buf(),
        source,
    })
}

fn write_file(path: &PathBuf, content: &str) -> Result<(), CliError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| CliError::Io {
            action: "create directory",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, content).map_err(|source| CliError::Io {
        action: "write",
        path: path.clone(),
        source,
    })
}
