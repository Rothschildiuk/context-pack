use std::fmt;
use std::num::ParseIntError;
use std::path::PathBuf;

use crate::model::{AppConfig, OutputFormat};

pub(crate) const DEFAULT_MAX_BYTES: usize = 4000;
pub(crate) const DEFAULT_MAX_FILES: usize = 12;
pub(crate) const DEFAULT_MAX_DEPTH: usize = 4;
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn parse_args<I>(args: I) -> Result<AppConfig, CliError>
where
    I: IntoIterator<Item = String>,
{
    let current_dir = std::env::current_dir().map_err(CliError::CurrentDir)?;
    let mut cwd = current_dir.clone();
    let mut format = OutputFormat::Markdown;
    let mut profile = None;
    let mut diff_from = None;
    let mut diff_to = None;
    let mut output = None;
    let mut init_memory = false;
    let mut refresh_memory = false;
    let mut mcp_server = false;
    let mut changed_only = false;
    let mut language_aware = true;
    let mut no_git = false;
    let mut no_tree = false;
    let mut max_bytes = DEFAULT_MAX_BYTES;
    let mut max_files = DEFAULT_MAX_FILES;
    let mut max_depth = DEFAULT_MAX_DEPTH;
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    let mut changed_only_set = false;
    let mut no_tree_set = false;
    let mut max_bytes_set = false;
    let mut max_files_set = false;

    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Err(CliError::Help(help_text())),
            "--version" | "-V" => return Err(CliError::Version(version_text())),
            "--init-memory" => init_memory = true,
            "--refresh-memory" => refresh_memory = true,
            "--mcp-server" => mcp_server = true,
            "--changed-only" => {
                changed_only = true;
                changed_only_set = true;
            }
            "--no-language-aware" => language_aware = false,
            "--no-git" => no_git = true,
            "--no-tree" => {
                no_tree = true;
                no_tree_set = true;
            }
            "--profile" => {
                let value = next_value(&mut iter, "--profile")?;
                validate_profile(&value)?;
                profile = Some(value);
            }
            "--format" => {
                let value = next_value(&mut iter, "--format")?;
                format = OutputFormat::parse(&value)?;
            }
            "--output" => {
                let value = next_value(&mut iter, "--output")?;
                output = Some(PathBuf::from(value));
            }
            "--diff-from" => {
                let value = next_value(&mut iter, "--diff-from")?;
                diff_from = Some(PathBuf::from(value));
            }
            "--diff-to" => {
                let value = next_value(&mut iter, "--diff-to")?;
                diff_to = Some(PathBuf::from(value));
            }
            "--cwd" => {
                let value = next_value(&mut iter, "--cwd")?;
                cwd = PathBuf::from(value);
            }
            "--max-bytes" => {
                let value = next_value(&mut iter, "--max-bytes")?;
                max_bytes = parse_usize("--max-bytes", &value)?;
                max_bytes_set = true;
            }
            "--max-files" => {
                let value = next_value(&mut iter, "--max-files")?;
                max_files = parse_usize("--max-files", &value)?;
                max_files_set = true;
            }
            "--max-depth" => {
                let value = next_value(&mut iter, "--max-depth")?;
                max_depth = parse_usize("--max-depth", &value)?;
            }
            "--include" => {
                let value = next_value(&mut iter, "--include")?;
                include.push(value);
            }
            "--exclude" => {
                let value = next_value(&mut iter, "--exclude")?;
                exclude.push(value);
            }
            value if value.starts_with('-') => {
                return Err(CliError::UnknownFlag(value.to_string()));
            }
            value => {
                return Err(CliError::UnexpectedArgument(value.to_string()));
            }
        }
    }

    apply_profile_defaults(
        profile.as_deref(),
        &mut changed_only,
        &mut no_tree,
        &mut max_bytes,
        &mut max_files,
        changed_only_set,
        no_tree_set,
        max_bytes_set,
        max_files_set,
    );

    if diff_from.is_some() != diff_to.is_some() {
        return Err(CliError::InvalidDiffArgs);
    }

    Ok(AppConfig {
        cwd: normalize_cwd(&current_dir, cwd),
        format,
        profile,
        diff_from,
        diff_to,
        output,
        init_memory,
        refresh_memory,
        mcp_server,
        changed_only,
        language_aware,
        no_git,
        no_tree,
        max_bytes,
        max_files,
        max_depth,
        include,
        exclude,
    })
}

pub(crate) fn normalize_cwd(current_dir: &PathBuf, cwd: PathBuf) -> PathBuf {
    let absolute = if cwd.is_absolute() {
        cwd
    } else {
        current_dir.join(cwd)
    };

    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}

fn next_value<I>(iter: &mut I, flag: &'static str) -> Result<String, CliError>
where
    I: Iterator<Item = String>,
{
    iter.next().ok_or(CliError::MissingValue(flag))
}

fn parse_usize(flag: &'static str, value: &str) -> Result<usize, CliError> {
    value
        .parse::<usize>()
        .map_err(|source| CliError::InvalidNumber {
            flag,
            value: value.to_string(),
            source,
        })
}

fn help_text() -> String {
    let heading = version_text();

    [
        &heading,
        "",
        "Usage:",
        "  context-pack [options]",
        "",
        "Options:",
        "  --format <markdown|json>  Output format (default: markdown)",
        "  --output <path>           Write output to a file instead of stdout",
        "  --diff-from <path>        Compare from an existing context-pack output file",
        "  --diff-to <path>          Compare to an existing context-pack output file",
        "  --init-memory             Create .context-pack/memory.md template",
        "  --refresh-memory          Regenerate .context-pack/memory.md",
        "  --mcp-server              Run the Context Pack MCP server over stdio",
        "  --cwd <path>              Repository root to inspect",
        "  --changed-only            Focus on active work",
        "  --profile <name>          Preset profile: onboarding|review|incident",
        "  --no-language-aware       Disable language-aware ranking boosts",
        "  --max-bytes <n>           Output byte budget (default: 4000)",
        "  --max-files <n>           Maximum selected files (default: 12)",
        "  --max-depth <n>           Maximum tree depth (default: 4)",
        "  --include <glob>          Extra include glob (repeatable)",
        "  --exclude <glob>          Extra exclude glob (repeatable)",
        "  --no-git                  Disable git collection",
        "  --no-tree                 Disable tree output",
        "  --version, -V             Show the program version",
        "  --help, -h                Show this help text",
    ]
    .join("\n")
}

fn version_text() -> String {
    format!("{APP_NAME} {APP_VERSION}")
}

fn validate_profile(value: &str) -> Result<(), CliError> {
    if matches!(value, "onboarding" | "review" | "incident") {
        Ok(())
    } else {
        Err(CliError::InvalidProfile(value.to_string()))
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_profile_defaults(
    profile: Option<&str>,
    changed_only: &mut bool,
    no_tree: &mut bool,
    max_bytes: &mut usize,
    max_files: &mut usize,
    changed_only_set: bool,
    no_tree_set: bool,
    max_bytes_set: bool,
    max_files_set: bool,
) {
    match profile {
        Some("review") => {
            if !changed_only_set {
                *changed_only = true;
            }
            if !no_tree_set {
                *no_tree = true;
            }
            if !max_files_set {
                *max_files = (*max_files).max(16);
            }
        }
        Some("incident") => {
            if !changed_only_set {
                *changed_only = true;
            }
            if !no_tree_set {
                *no_tree = true;
            }
            if !max_files_set {
                *max_files = (*max_files).max(20);
            }
            if !max_bytes_set {
                *max_bytes = (*max_bytes).max(5000);
            }
        }
        _ => {}
    }
}

#[derive(Debug)]
pub enum CliError {
    Help(String),
    Version(String),
    CurrentDir(std::io::Error),
    MissingValue(&'static str),
    InvalidFormat(String),
    InvalidProfile(String),
    InvalidDiffArgs,
    InvalidNumber {
        flag: &'static str,
        value: String,
        source: ParseIntError,
    },
    UnknownFlag(String),
    UnexpectedArgument(String),
    Mcp(String),
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    MemoryExists(PathBuf),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Help(text) => write!(f, "{text}"),
            Self::Version(text) => write!(f, "{text}"),
            Self::CurrentDir(source) => write!(f, "failed to resolve current directory: {source}"),
            Self::MissingValue(flag) => write!(f, "missing value for {flag}"),
            Self::InvalidFormat(value) => {
                write!(f, "invalid format '{value}', expected 'markdown' or 'json'")
            }
            Self::InvalidProfile(value) => {
                write!(
                    f,
                    "invalid profile '{value}', expected 'onboarding', 'review', or 'incident'"
                )
            }
            Self::InvalidDiffArgs => {
                write!(f, "both --diff-from and --diff-to must be provided together")
            }
            Self::InvalidNumber {
                flag,
                value,
                source,
            } => {
                write!(f, "invalid numeric value for {flag}: '{value}' ({source})")
            }
            Self::UnknownFlag(flag) => write!(f, "unknown flag '{flag}'"),
            Self::UnexpectedArgument(value) => {
                write!(f, "unexpected positional argument '{value}'")
            }
            Self::Mcp(message) => write!(f, "{message}"),
            Self::Io {
                action,
                path,
                source,
            } => {
                write!(f, "failed to {action} '{}': {source}", path.display())
            }
            Self::MemoryExists(path) => {
                write!(
                    f,
                    "memory file already exists at '{}'\nUse --refresh-memory to regenerate it, or edit the file manually.",
                    path.display()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{parse_args, CliError, APP_NAME, APP_VERSION};

    #[test]
    fn version_flag_returns_package_version() {
        let err = parse_args(["--version".to_string()]).expect_err("version exits early");

        match err {
            CliError::Version(text) => assert_eq!(text, format!("{APP_NAME} {APP_VERSION}")),
            other => panic!("expected version output, got {other}"),
        }
    }

    #[test]
    fn short_version_flag_returns_package_version() {
        let err = parse_args(["-V".to_string()]).expect_err("version exits early");

        match err {
            CliError::Version(text) => assert_eq!(text, format!("{APP_NAME} {APP_VERSION}")),
            other => panic!("expected version output, got {other}"),
        }
    }

    #[test]
    fn io_error_includes_action_context() {
        let err = CliError::Io {
            action: "write output",
            path: PathBuf::from("/tmp/out.md"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
        };

        assert_eq!(
            err.to_string(),
            "failed to write output '/tmp/out.md': permission denied"
        );
    }

    #[test]
    fn mcp_server_flag_is_parsed() {
        let config =
            parse_args(["--mcp-server".to_string()]).expect("mcp server flag should parse");

        assert!(config.mcp_server);
    }

    #[test]
    fn no_language_aware_flag_is_parsed() {
        let config = parse_args(["--no-language-aware".to_string()])
            .expect("no-language-aware flag should parse");

        assert!(!config.language_aware);
    }

    #[test]
    fn review_profile_enables_changed_only_and_no_tree() {
        let config = parse_args(["--profile".to_string(), "review".to_string()])
            .expect("review profile should parse");

        assert_eq!(config.profile.as_deref(), Some("review"));
        assert!(config.changed_only);
        assert!(config.no_tree);
        assert!(config.max_files >= 16);
    }

    #[test]
    fn diff_args_must_be_provided_together() {
        let err = parse_args(["--diff-from".to_string(), "a.md".to_string()])
            .expect_err("single diff arg should fail");

        match err {
            CliError::InvalidDiffArgs => {}
            other => panic!("expected InvalidDiffArgs, got {other}"),
        }
    }
}
