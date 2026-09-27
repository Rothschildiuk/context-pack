//! A directory map instead of a raw tree: each top-level directory with its
//! size, languages, and role, expanded one level where the code lives.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::index::RepoIndex;
use crate::model::LayoutEntry;
use crate::paths::{self, PathRole};

const MAX_TOP_LEVEL: usize = 14;
const MAX_CHILDREN: usize = 8;

#[derive(Default)]
struct DirStats {
    files: usize,
    languages: BTreeMap<&'static str, usize>,
    children: BTreeMap<String, DirStats>,
}

impl DirStats {
    fn add(&mut self, language: Option<&'static str>) {
        self.files += 1;
        if let Some(language) = language {
            *self.languages.entry(language).or_default() += 1;
        }
    }

    fn top_languages(&self) -> Vec<String> {
        let mut languages = self.languages.iter().collect::<Vec<_>>();
        languages.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
        languages
            .into_iter()
            .filter(|(_, count)| **count * 10 >= self.files.max(1) || self.files < 10)
            .take(2)
            .map(|(language, _)| language.to_string())
            .collect()
    }

    fn source_files(&self) -> usize {
        self.languages.values().sum()
    }
}

pub fn build(index: &RepoIndex) -> (Vec<LayoutEntry>, Vec<String>) {
    let mut top: BTreeMap<String, DirStats> = BTreeMap::new();
    let mut root = DirStats::default();
    let mut root_code = 0usize;

    for entry in &index.files {
        let language = paths::language(&entry.path);
        let mut components = entry
            .path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned());
        let first = components.next().unwrap_or_default();
        let Some(second) = components.next() else {
            root.add(language);
            if language.is_some() && !is_tool_config(paths::file_name(&entry.path)) {
                root_code += 1;
            }
            continue;
        };
        let stats = top.entry(first).or_default();
        stats.add(language);
        if components.next().is_some() {
            stats.children.entry(second).or_default().add(language);
        }
    }

    let total_source =
        top.values().map(DirStats::source_files).sum::<usize>() + root.source_files();
    let mut entries = top
        .iter()
        .filter(|(name, _)| !name.starts_with('.') || name.as_str() == ".github")
        .map(|(name, stats)| {
            let path = PathBuf::from(name);
            let role = dir_role(&path);
            let expand = role == PathRole::Source
                && stats.children.len() > 1
                && (stats.source_files() * 5 >= total_source.max(1) || stats.files >= 40);
            let children = if expand {
                children(&path, stats)
            } else {
                Vec::new()
            };
            (
                role,
                LayoutEntry {
                    path: format!("{name}/"),
                    role: role.label().to_string(),
                    files: stats.files,
                    languages: stats.top_languages(),
                    children,
                },
            )
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        role_order(left.0)
            .cmp(&role_order(right.0))
            .then_with(|| right.1.files.cmp(&left.1.files))
            .then_with(|| left.1.path.cmp(&right.1.path))
    });

    let mut layout = Vec::new();
    if root_code >= 3 {
        layout.push(LayoutEntry {
            path: "./".to_string(),
            role: "source".to_string(),
            files: root.files,
            languages: root.top_languages(),
            children: Vec::new(),
        });
    }

    let mut notes = Vec::new();
    let hidden = entries.len().saturating_sub(MAX_TOP_LEVEL);
    layout.extend(
        entries
            .into_iter()
            .take(MAX_TOP_LEVEL)
            .map(|(_, entry)| entry),
    );
    if hidden > 0 {
        notes.push(format!(
            "layout: {hidden} smaller top-level directories not shown"
        ));
    }
    (layout, notes)
}

fn children(parent: &Path, stats: &DirStats) -> Vec<LayoutEntry> {
    let mut children = stats
        .children
        .iter()
        .filter(|(name, _)| !name.starts_with('.'))
        .map(|(name, child)| {
            let path = parent.join(name);
            LayoutEntry {
                path: format!("{}/", paths::display(&path)),
                role: dir_role(&path).label().to_string(),
                files: child.files,
                languages: child.top_languages(),
                children: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    children.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then_with(|| left.path.cmp(&right.path))
    });
    let total = children.len();
    children.truncate(MAX_CHILDREN);
    if total > MAX_CHILDREN {
        let rest = stats
            .children
            .values()
            .map(|child| child.files)
            .sum::<usize>()
            - children.iter().map(|child| child.files).sum::<usize>();
        children.push(LayoutEntry {
            path: format!(
                "{}/… (+{} dirs)",
                paths::display(parent),
                total - MAX_CHILDREN
            ),
            role: "source".to_string(),
            files: rest,
            languages: Vec::new(),
            children: Vec::new(),
        });
    }
    children
}

/// `eslint.config.js`, `vite.config.ts`, `.prettierrc.js`, `setup.py`: tooling, not product code.
fn is_tool_config(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.starts_with('.')
        || lower.contains(".config.")
        || lower.contains("rc.")
        || matches!(
            lower.as_str(),
            "setup.py"
                | "conftest.py"
                | "noxfile.py"
                | "build.rs"
                | "knip.js"
                | "gulpfile.js"
                | "gruntfile.js"
                | "rakefile"
                | "dangerfile.js"
                | "dangerfile.ts"
        )
}

fn dir_role(dir: &Path) -> PathRole {
    // Classify the directory as if it contained a plain file.
    paths::role(&dir.join("x"))
}

fn role_order(role: PathRole) -> usize {
    match role {
        PathRole::Source => 0,
        PathRole::Tests => 1,
        PathRole::Docs => 2,
        PathRole::Scripts => 3,
        PathRole::Examples => 4,
        PathRole::Benchmarks => 5,
        PathRole::Fixtures => 6,
        PathRole::Ci => 7,
        PathRole::Generated => 8,
        PathRole::Vendor => 9,
        PathRole::Hidden => 10,
    }
}
