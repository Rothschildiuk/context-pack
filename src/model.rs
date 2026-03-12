use std::path::PathBuf;

use crate::cli::CliError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub cwd: PathBuf,
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
    pub changed_only: bool,
    pub no_git: bool,
    pub no_tree: bool,
    pub max_bytes: usize,
    pub max_files: usize,
    pub max_depth: usize,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
}

impl OutputFormat {
    pub fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "markdown" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            _ => Err(CliError::InvalidFormat(value.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub project_types: Vec<String>,
    pub primary_languages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCategory {
    Instructions,
    Overview,
    Manifest,
    Build,
    ChangedSource,
    EntryPoint,
    Config,
    SupportingDoc,
}

impl SignalCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Instructions => "instructions",
            Self::Overview => "overview",
            Self::Manifest => "manifest",
            Self::Build => "build",
            Self::ChangedSource => "changed_source",
            Self::EntryPoint => "entrypoint",
            Self::Config => "config",
            Self::SupportingDoc => "supporting_doc",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportantFile {
    pub path: PathBuf,
    pub reason: String,
    pub category: SignalCategory,
    pub score: usize,
    pub excerpt: String,
    pub truncated: bool,
}

impl ImportantFile {
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|value| value.to_str())
    }
}

#[derive(Debug, Clone)]
pub struct BriefingItem {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct LargeCodeFile {
    pub path: PathBuf,
    pub loc: usize,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct AgentBriefing {
    pub repo_summary: Vec<String>,
    pub active_work: Vec<String>,
    pub read_these_first: Vec<BriefingItem>,
    pub likely_entry_points: Vec<BriefingItem>,
    pub docker_summary: Vec<String>,
    pub dependency_summary: Vec<String>,
    pub large_code_files: Vec<LargeCodeFile>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub briefing: AgentBriefing,
    pub repo: RepoInfo,
    pub tree_summary: String,
    pub important_files: Vec<ImportantFile>,
    pub git_available: bool,
    pub git_branch_context: GitBranchContext,
    pub git_changes: Vec<GitChange>,
    pub git_summary: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WalkResult {
    pub tree_summary: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GitResult {
    pub summary: String,
    pub available: bool,
    pub branch_context: GitBranchContext,
    pub changes: Vec<GitChange>,
    pub changed_files: Vec<PathBuf>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GitBranchContext {
    pub current_branch: Option<String>,
    pub local_branches: Vec<String>,
    pub upstream_branch: Option<String>,
    pub default_branch: Option<String>,
    pub comparison_target: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone)]
pub struct GitChange {
    pub path: PathBuf,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub files: Vec<ImportantFile>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct OutputBudgets {
    pub briefing: usize,
    pub git: usize,
    pub excerpts: usize,
    pub tree: usize,
}
