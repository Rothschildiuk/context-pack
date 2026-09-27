//! One pass over the repository. Every later stage works from this index
//! instead of walking the file system again.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::model::AppConfig;

/// Hard cap so a pathological checkout cannot stall the briefing.
const MAX_INDEXED_FILES: usize = 200_000;
/// Files larger than this are never read for excerpts or parsing.
pub const MAX_READ_BYTES: u64 = 1024 * 1024;

/// Directories skipped even when a repo forgets to gitignore them.
const ALWAYS_SKIPPED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
    ".gradle",
    ".idea",
    ".DS_Store",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    ".context-pack",
];

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
}

pub struct RepoIndex {
    pub root: PathBuf,
    pub files: Vec<FileEntry>,
    pub truncated: bool,
    /// Files matched by `--include`; they are always surfaced as key files.
    pub forced: Vec<PathBuf>,
    lookup: HashSet<PathBuf>,
    dir_counts: HashMap<PathBuf, usize>,
}

impl RepoIndex {
    pub fn build(config: &AppConfig) -> Self {
        let exclude = build_globset(&config.exclude);
        let include = build_globset(&config.include);
        let mut files = Vec::new();
        let mut truncated = false;

        walk(&config.cwd, true, |path, size| {
            if exclude.as_ref().is_some_and(|set| set.is_match(path)) {
                return true;
            }
            if files.len() >= MAX_INDEXED_FILES {
                truncated = true;
                return false;
            }
            files.push(FileEntry {
                path: path.to_path_buf(),
                size,
            });
            true
        });

        let mut forced = Vec::new();
        if let Some(include) = &include {
            // Includes may point at gitignored files, so walk without ignore rules.
            let known = files
                .iter()
                .map(|entry| entry.path.clone())
                .collect::<HashSet<_>>();
            walk(&config.cwd, false, |path, size| {
                if include.is_match(path) {
                    if !known.contains(path) {
                        files.push(FileEntry {
                            path: path.to_path_buf(),
                            size,
                        });
                    }
                    forced.push(path.to_path_buf());
                }
                forced.len() < 64
            });
        }

        files.sort_by(|left, right| left.path.cmp(&right.path));
        forced.sort();
        let lookup = files.iter().map(|entry| entry.path.clone()).collect();
        let mut dir_counts: HashMap<PathBuf, usize> = HashMap::new();
        for entry in &files {
            for ancestor in entry.path.ancestors().skip(1) {
                *dir_counts.entry(ancestor.to_path_buf()).or_default() += 1;
            }
        }

        Self {
            root: config.cwd.clone(),
            files,
            truncated,
            forced,
            lookup,
            dir_counts,
        }
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        self.lookup.contains(path.as_ref())
    }

    /// Number of indexed files below `dir` (recursive). `""` is the whole repo.
    pub fn files_under(&self, dir: &Path) -> usize {
        self.dir_counts.get(dir).copied().unwrap_or(0)
    }

    pub fn size_of(&self, path: &Path) -> Option<u64> {
        self.files
            .binary_search_by(|entry| entry.path.as_path().cmp(path))
            .ok()
            .map(|index| self.files[index].size)
    }

    /// Read a UTF-8 text file from the index, refusing binaries and huge files.
    pub fn read(&self, path: impl AsRef<Path>) -> Option<String> {
        let path = path.as_ref();
        if self.size_of(path).is_some_and(|size| size > MAX_READ_BYTES) {
            return None;
        }
        read_text(&self.root.join(path))
    }
}

pub fn read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() as u64 > MAX_READ_BYTES || bytes.iter().take(8192).any(|byte| *byte == 0) {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn walk(root: &Path, respect_ignores: bool, mut visit: impl FnMut(&Path, u64) -> bool) {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .parents(false)
        .git_ignore(respect_ignores)
        .git_exclude(respect_ignores)
        .git_global(respect_ignores)
        .ignore(respect_ignores)
        .require_git(false)
        .follow_links(false)
        .sort_by_file_path(|left, right| left.cmp(right))
        .filter_entry(|entry| {
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            let name = entry.file_name().to_string_lossy();
            !(is_dir && entry.depth() > 0 && ALWAYS_SKIPPED_DIRS.contains(&name.as_ref()))
                && name != ".DS_Store"
        });

    for entry in builder.build().flatten() {
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        if !visit(relative, size) {
            break;
        }
    }
}

fn build_globset(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let pattern = pattern
            .trim()
            .trim_start_matches("./")
            .trim_end_matches('/');
        if pattern.is_empty() {
            continue;
        }
        // Treat patterns like gitignore does: a bare name matches at any depth,
        // and a directory pattern matches everything below it.
        let base = if pattern.contains('/') {
            pattern.trim_start_matches('/').to_string()
        } else {
            format!("**/{pattern}")
        };
        for candidate in [base.clone(), format!("{base}/**")] {
            if let Ok(glob) = Glob::new(&candidate) {
                builder.add(glob);
            }
        }
    }
    builder.build().ok()
}
