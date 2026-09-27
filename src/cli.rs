use std::fmt;
use std::num::ParseIntError;
use std::path::PathBuf;

use crate::model::{AppConfig, Command, OutputFormat, Profile};

pub(crate) const DEFAULT_MAX_BYTES: usize = 6000;
pub(crate) const DEFAULT_MAX_FILES: usize = 8;
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct Overrides {
    changed_only: Option<bool>,
    no_layout: Option<bool>,
    excerpts: Option<bool>,
    max_bytes: Option<usize>,
    max_files: Option<usize>,
}

pub fn parse_args<I>(args: I) -> Result<AppConfig, CliError>
where
    I: IntoIterator<Item = String>,
{
    let current_dir = std::env::current_dir().map_err(CliError::CurrentDir)?;
    let mut config = default_config(current_dir.clone());
    let mut overrides = Overrides::default();
    let mut positionals = Vec::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" | "help" => return Err(CliError::Help(help_text())),
            "--version" | "-V" | "version" => return Err(CliError::Version(version_text())),
            "--cwd" => config.cwd = PathBuf::from(next_value(&mut iter, "--cwd")?),
            "--format" => config.format = OutputFormat::parse(&next_value(&mut iter, "--format")?)?,
            "--output" | "-o" => {
                config.output = Some(PathBuf::from(next_value(&mut iter, "--output")?))
            }
            "--profile" => {
                config.profile = Some(Profile::parse(&next_value(&mut iter, "--profile")?)?)
            }
            "--changed-only" => overrides.changed_only = Some(true),
            "--no-git" => config.no_git = true,
            "--no-layout" | "--no-tree" => overrides.no_layout = Some(true),
            "--excerpts" => overrides.excerpts = Some(true),
            "--no-excerpts" => overrides.excerpts = Some(false),
            "--quiet" => {
                overrides.excerpts = Some(false);
                overrides.no_layout = Some(true);
            }
            "--max-bytes" => {
                overrides.max_bytes = Some(parse_usize(
                    "--max-bytes",
                    &next_value(&mut iter, "--max-bytes")?,
                )?)
            }
            "--max-files" => {
                overrides.max_files = Some(parse_usize(
                    "--max-files",
                    &next_value(&mut iter, "--max-files")?,
                )?)
            }
            "--include" => config.include.push(next_value(&mut iter, "--include")?),
            "--exclude" => config.exclude.push(next_value(&mut iter, "--exclude")?),
            "--mcp-server" => config.command = Command::McpServer,
            "--init-memory" => config.command = Command::InitMemory,
            "--refresh-memory" => config.command = Command::RefreshMemory,
            value if value.starts_with('-') => {
                return Err(CliError::UnknownFlag(value.to_string()))
            }
            value => positionals.push(value.to_string()),
        }
    }

    apply_command(&positionals, &mut config, &mut overrides)?;
    apply_profile(&mut config, &overrides);
    config.cwd = normalize_cwd(&current_dir, config.cwd);
    Ok(config)
}

pub(crate) fn default_config(cwd: PathBuf) -> AppConfig {
    AppConfig {
        cwd,
        command: Command::Brief,
        format: OutputFormat::Markdown,
        profile: None,
        output: None,
        changed_only: false,
        no_git: false,
        no_layout: false,
        excerpts: true,
        max_bytes: DEFAULT_MAX_BYTES,
        max_files: DEFAULT_MAX_FILES,
        include: Vec::new(),
        exclude: Vec::new(),
        note: None,
    }
}

fn apply_command(
    positionals: &[String],
    config: &mut AppConfig,
    overrides: &mut Overrides,
) -> Result<(), CliError> {
    let words = positionals.iter().map(String::as_str).collect::<Vec<_>>();
    match words.as_slice() {
        [] | ["brief"] => {}
        ["changed"] => overrides.changed_only = Some(true),
        ["review"] => config.profile = Some(Profile::Review),
        ["compact"] => config.profile = Some(Profile::Compact),
        ["deep"] => config.profile = Some(Profile::Deep),
        ["json"] => config.format = OutputFormat::Json,
        ["mcp"] => config.command = Command::McpServer,
        ["memory", "init"] | ["memory-init"] => config.command = Command::InitMemory,
        ["memory", "refresh"] | ["memory-refresh"] => config.command = Command::RefreshMemory,
        ["context", "refresh"] => config.command = Command::RefreshContext,
        ["context", "check"] => config.command = Command::CheckContext,
        ["memory", "add", note @ ..] if !note.is_empty() => {
            config.command = Command::AddMemoryNote;
            config.note = Some(note.join(" "));
        }
        ["memory"] | ["memory", "add"] => {
            return Err(CliError::MissingValue("memory <init|refresh|add \"note\">"))
        }
        ["context"] => return Err(CliError::MissingValue("context <refresh|check>")),
        [_, extra, ..] if matches!(words[0], "memory" | "context") => {
            return Err(CliError::UnexpectedArgument(extra.to_string()))
        }
        [first, ..] => return Err(CliError::UnexpectedArgument(first.to_string())),
    }
    Ok(())
}

fn apply_profile(config: &mut AppConfig, overrides: &Overrides) {
    match config.profile {
        Some(Profile::Compact) => {
            config.max_bytes = 2000;
            config.max_files = 5;
            config.excerpts = false;
        }
        Some(Profile::Deep) => {
            config.max_bytes = 16000;
            config.max_files = 16;
        }
        Some(Profile::Review) => {
            config.changed_only = true;
            config.no_layout = true;
            config.max_files = 20;
        }
        None => {}
    }
    // Explicit flags always win over profile defaults.
    if let Some(value) = overrides.changed_only {
        config.changed_only = value;
    }
    if let Some(value) = overrides.no_layout {
        config.no_layout = value;
    }
    if let Some(value) = overrides.excerpts {
        config.excerpts = value;
    }
    if let Some(value) = overrides.max_bytes {
        config.max_bytes = value;
    }
    if let Some(value) = overrides.max_files {
        config.max_files = value.max(1);
    }
}

pub(crate) fn normalize_cwd(current_dir: &std::path::Path, cwd: PathBuf) -> PathBuf {
    let absolute = if cwd.is_absolute() {
        cwd
    } else {
        current_dir.join(cwd)
    };
    absolute.canonicalize().unwrap_or(absolute)
}

fn next_value<I>(iter: &mut I, flag: &'static str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    iter.next().ok_or(CliError::MissingValue(flag))
}

fn parse_usize(flag: &'static str, value: &str) -> Result<usize, CliError> {
    value.parse().map_err(|source| CliError::InvalidNumber {
        flag,
        value: value.to_string(),
        source,
    })
}

fn help_text() -> String {
    format!(
        "{heading}

First-pass repository briefing for coding agents: instructions to follow,
commands to build and test, entry points, key files, layout, and active work.

Usage:
  context-pack [command] [options]

Commands:
  brief                  Repository briefing (default)
  changed                Briefing focused on uncommitted and branch changes
  review                 Review preset: changed files, branch diff, no layout
  compact | deep         Smaller (2 KB) or larger (16 KB) briefing presets
  json                   Same as --format json
  memory init            Create .context-pack/memory.md for durable notes
  memory refresh         Mark memory notes as reviewed (never rewrites notes)
  memory add <note>      Append one durable fact to the memory notes
  context refresh        Write .context-pack/PROJECT_CONTEXT.{{md,json}}
  context check          Verify context artifacts exist and match HEAD
  mcp                    Run the MCP server over stdio

Options:
  --cwd <path>           Repository root to inspect (default: current directory)
  --format <markdown|json>
  --output, -o <path>    Write to a file instead of stdout
  --profile <compact|deep|review>
  --changed-only         Limit key files to active work
  --max-bytes <n>        Markdown size budget (default: {DEFAULT_MAX_BYTES})
  --max-files <n>        Maximum key files (default: {DEFAULT_MAX_FILES})
  --include <glob>       Always surface matching files (repeatable)
  --exclude <glob>       Ignore matching files (repeatable)
  --no-git               Skip git inspection
  --no-layout            Skip the directory map
  --no-excerpts          Skip file excerpts
  --quiet                Same as --no-layout --no-excerpts
  --version, -V
  --help, -h

Examples:
  context-pack
  context-pack review --format json
  context-pack --cwd ../service --max-bytes 3000
  context-pack context refresh",
        heading = version_text()
    )
}

fn version_text() -> String {
    format!("{APP_NAME} {APP_VERSION}")
}

#[derive(Debug)]
pub enum CliError {
    Help(String),
    Version(String),
    CurrentDir(std::io::Error),
    MissingValue(&'static str),
    InvalidFormat(String),
    InvalidProfile(String),
    InvalidNumber {
        flag: &'static str,
        value: String,
        source: ParseIntError,
    },
    UnknownFlag(String),
    UnexpectedArgument(String),
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    MemoryExists(PathBuf),
    InvalidArtifact(String),
    Mcp(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Help(text) | Self::Version(text) => write!(f, "{text}"),
            Self::CurrentDir(source) => write!(f, "failed to resolve current directory: {source}"),
            Self::MissingValue(flag) => write!(f, "missing value for {flag}"),
            Self::InvalidFormat(value) => {
                write!(f, "invalid format '{value}', expected 'markdown' or 'json'")
            }
            Self::InvalidProfile(value) => {
                write!(f, "invalid profile '{value}', expected 'compact', 'deep', or 'review'")
            }
            Self::InvalidNumber { flag, value, source } => {
                write!(f, "invalid numeric value for {flag}: '{value}' ({source})")
            }
            Self::UnknownFlag(flag) => write!(f, "unknown flag '{flag}' (see --help)"),
            Self::UnexpectedArgument(value) => {
                write!(f, "unexpected argument '{value}' (see --help)")
            }
            Self::Io { action, path, source } => {
                write!(f, "failed to {action} '{}': {source}", path.display())
            }
            Self::MemoryExists(path) => write!(
                f,
                "memory file already exists at '{}'. Edit it directly, or run `context-pack memory refresh` after reviewing it.",
                path.display()
            ),
            Self::InvalidArtifact(message) | Self::Mcp(message) => write!(f, "{message}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<AppConfig, CliError> {
        parse_args(args.iter().map(|value| value.to_string()))
    }

    #[test]
    fn defaults_are_a_full_markdown_brief() {
        let config = parse(&[]).unwrap();
        assert_eq!(config.command, Command::Brief);
        assert_eq!(config.format, OutputFormat::Markdown);
        assert!(config.excerpts);
        assert_eq!(config.max_bytes, DEFAULT_MAX_BYTES);
    }

    #[test]
    fn subcommand_help_is_help() {
        assert!(matches!(
            parse(&["context", "--help"]),
            Err(CliError::Help(_))
        ));
        assert!(matches!(parse(&["help"]), Err(CliError::Help(_))));
    }

    #[test]
    fn version_flag_returns_package_version() {
        match parse(&["-V"]) {
            Err(CliError::Version(text)) => assert_eq!(text, format!("{APP_NAME} {APP_VERSION}")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn review_profile_is_changed_only_without_layout() {
        let config = parse(&["review"]).unwrap();
        assert!(config.changed_only);
        assert!(config.no_layout);
    }

    #[test]
    fn explicit_flags_beat_profile_defaults() {
        let config = parse(&["compact", "--max-bytes", "900", "--excerpts"]).unwrap();
        assert_eq!(config.max_bytes, 900);
        assert!(config.excerpts);
        assert_eq!(config.max_files, 5);
    }

    #[test]
    fn nested_subcommands_are_parsed() {
        assert_eq!(
            parse(&["memory", "refresh"]).unwrap().command,
            Command::RefreshMemory
        );
        assert_eq!(
            parse(&["context", "check"]).unwrap().command,
            Command::CheckContext
        );
        assert!(matches!(
            parse(&["context", "nope"]),
            Err(CliError::UnexpectedArgument(_))
        ));
        let note = parse(&["memory", "add", "tests", "need", "docker"]).unwrap();
        assert_eq!(note.command, Command::AddMemoryNote);
        assert_eq!(note.note.as_deref(), Some("tests need docker"));
    }

    #[test]
    fn removed_flags_are_rejected() {
        assert!(matches!(
            parse(&["--format", "viking"]),
            Err(CliError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse(&["--minify"]),
            Err(CliError::UnknownFlag(_))
        ));
    }
}
