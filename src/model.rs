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

#[derive(Debug, Clone)]
pub struct ImportantFile {
    pub path: PathBuf,
    pub reason: String,
    pub excerpt: String,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub repo: RepoInfo,
    pub tree_summary: String,
    pub important_files: Vec<ImportantFile>,
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
    pub changed_files: Vec<PathBuf>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub files: Vec<ImportantFile>,
    pub notes: Vec<String>,
}
