use std::path::PathBuf;

use serde::Serialize;

use crate::cli::CliError;

pub const SCHEMA_VERSION: &str = "2.0";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub cwd: PathBuf,
    pub command: Command,
    pub format: OutputFormat,
    pub profile: Option<Profile>,
    pub output: Option<PathBuf>,
    pub changed_only: bool,
    pub no_git: bool,
    pub no_layout: bool,
    pub excerpts: bool,
    pub max_bytes: usize,
    pub max_files: usize,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// Text for `memory add`.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Brief,
    InitMemory,
    RefreshMemory,
    AddMemoryNote,
    RefreshContext,
    CheckContext,
    McpServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            _ => Err(CliError::InvalidFormat(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Compact,
    Deep,
    Review,
}

impl Profile {
    pub fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "compact" => Ok(Self::Compact),
            "deep" => Ok(Self::Deep),
            "review" => Ok(Self::Review),
            _ => Err(CliError::InvalidProfile(value.to_string())),
        }
    }
}

/// The whole briefing. Markdown and JSON are two renderings of this value.
#[derive(Debug, Clone, Serialize)]
pub struct Brief {
    pub schema_version: &'static str,
    pub tool_version: &'static str,
    pub repo: RepoSummary,
    pub instructions: Vec<FileRef>,
    pub commands: Vec<CommandHint>,
    pub entry_points: Vec<FileRef>,
    pub key_files: Vec<FileRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Workspace>,
    pub layout: Vec<LayoutEntry>,
    pub docs: Vec<FileRef>,
    pub config: Vec<FileRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryInfo>,
    pub excerpts: Vec<Excerpt>,
    pub notes: Vec<String>,
    pub stats: Stats,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoSummary {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub languages: Vec<LanguageShare>,
    /// Build systems and notable frameworks, e.g. `cargo`, `pnpm`, `next.js`.
    pub stack: Vec<String>,
    pub dependencies: Vec<DependencyList>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LanguageShare {
    pub name: String,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyList {
    pub manifest: String,
    pub runtime: Vec<String>,
    pub dev: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileRef {
    pub path: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandHint {
    /// setup, build, test, lint, format, typecheck, check, run, dev, ci
    pub kind: String,
    pub command: String,
    /// Where the command came from, with the underlying recipe when known.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub packages: usize,
    pub groups: Vec<WorkspaceGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceGroup {
    pub pattern: String,
    pub role: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutEntry {
    pub path: String,
    pub role: String,
    pub files: usize,
    pub languages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<LayoutEntry>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    /// Ref the current branch is compared against, when it is not the default branch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    pub branch_commits: usize,
    pub branch_changes: Vec<GitChange>,
    pub working_changes: Vec<GitChange>,
    pub recent_commits: Vec<String>,
    pub shallow: bool,
    #[serde(skip)]
    pub churn: Vec<(PathBuf, usize)>,
    #[serde(skip)]
    pub history_depth: usize,
    #[serde(skip)]
    pub latest_commit_unix: Option<u64>,
}

impl GitInfo {
    /// Files touched by the working tree or the current branch, newest first.
    pub fn active_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for change in self
            .working_changes
            .iter()
            .chain(self.branch_changes.iter())
        {
            let path = PathBuf::from(&change.path);
            if change.status != "D" && !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GitChange {
    pub path: String,
    /// Porcelain-style code: M, A, D, R, ?? or T.
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
    pub notes: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Excerpt {
    pub path: String,
    pub content: String,
    pub truncated: bool,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub files_indexed: usize,
    pub elapsed_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_from_commit: Option<String>,
}
