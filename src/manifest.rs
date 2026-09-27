//! Parse project manifests into facts an agent actually needs: what the
//! project is called, what it says it does, which files it declares as entry
//! points, which scripts it exposes, and which packages make up a workspace.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as Json;
use toml::Value as Toml;

use crate::index::RepoIndex;
use crate::paths;

const MAX_MANIFESTS: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ecosystem {
    Cargo,
    Npm,
    Deno,
    Python,
    Go,
    Maven,
    Gradle,
    Dotnet,
    Ruby,
    Php,
    Swift,
    Dart,
    Elixir,
    CMake,
}

impl Ecosystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Npm => "npm",
            Self::Deno => "deno",
            Self::Python => "python",
            Self::Go => "go",
            Self::Maven => "maven",
            Self::Gradle => "gradle",
            Self::Dotnet => "dotnet",
            Self::Ruby => "ruby",
            Self::Php => "composer",
            Self::Swift => "swiftpm",
            Self::Dart => "dart",
            Self::Elixir => "mix",
            Self::CMake => "cmake",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub path: PathBuf,
    pub ecosystem: Ecosystem,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Declared or conventional entry points: (repo-relative path, reason).
    pub entry_points: Vec<(PathBuf, String)>,
    /// Named scripts or tasks (package.json scripts, deno tasks, composer scripts).
    pub scripts: Vec<(String, String)>,
    pub dependencies: Vec<String>,
    pub dev_dependencies: Vec<String>,
    /// Workspace member globs declared by this manifest.
    pub members: Vec<String>,
    /// Tool sections present in the manifest (pytest, ruff, mypy, ...).
    pub tools: Vec<String>,
}

impl Manifest {
    fn new(path: &Path, ecosystem: Ecosystem) -> Self {
        Self {
            path: path.to_path_buf(),
            ecosystem,
            name: None,
            description: None,
            entry_points: Vec::new(),
            scripts: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            members: Vec::new(),
            tools: Vec::new(),
        }
    }

    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new(""))
    }

    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.iter().any(|value| value == tool)
    }

    pub fn has_dependency(&self, name: &str) -> bool {
        self.dependencies
            .iter()
            .chain(self.dev_dependencies.iter())
            .any(|value| value == name)
    }
}

pub fn is_manifest_name(file_name: &str) -> bool {
    matches!(
        file_name,
        "Cargo.toml"
            | "package.json"
            | "deno.json"
            | "deno.jsonc"
            | "pyproject.toml"
            | "setup.py"
            | "requirements.txt"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "Gemfile"
            | "composer.json"
            | "Package.swift"
            | "pubspec.yaml"
            | "mix.exs"
            | "CMakeLists.txt"
    ) || file_name.ends_with(".csproj")
        || file_name.ends_with(".fsproj")
}

pub fn collect(index: &RepoIndex) -> Vec<Manifest> {
    let mut manifests = Vec::new();

    for entry in &index.files {
        if manifests.len() >= MAX_MANIFESTS {
            break;
        }
        let file_name = paths::file_name(&entry.path);
        if !is_manifest_name(file_name) {
            continue;
        }
        // A CMakeLists.txt in every subdirectory is normal; only the top one matters.
        if file_name == "CMakeLists.txt" && paths::depth(&entry.path) > 1 {
            continue;
        }
        let Some(text) = index.read(&entry.path) else {
            continue;
        };
        if let Some(manifest) = parse(index, &entry.path, file_name, &text) {
            manifests.push(manifest);
        }
    }

    // Shallow manifests first: they describe the project as a whole.
    manifests.sort_by_key(|manifest| (paths::depth(&manifest.path), manifest.path.clone()));
    manifests
}

fn parse(index: &RepoIndex, path: &Path, file_name: &str, text: &str) -> Option<Manifest> {
    match file_name {
        "Cargo.toml" => parse_cargo(index, path, text),
        "package.json" => parse_package_json(index, path, text),
        "deno.json" | "deno.jsonc" => parse_deno(path, text),
        "pyproject.toml" => parse_pyproject(index, path, text),
        "setup.py" => Some(parse_setup_py(path, text)),
        "requirements.txt" => Some(parse_requirements(path, text)),
        "go.mod" => Some(parse_go_mod(index, path, text)),
        "pom.xml" => Some(parse_pom(index, path, text)),
        "build.gradle" | "build.gradle.kts" => Some(parse_gradle(index, path)),
        "Gemfile" => Some(parse_gemfile(path, text)),
        "composer.json" => parse_composer(path, text),
        "Package.swift" => Some(parse_swift(index, path, text)),
        "pubspec.yaml" => Some(parse_pubspec(index, path, text)),
        "mix.exs" => Some(parse_mix(path, text)),
        "CMakeLists.txt" => Some(parse_cmake(path, text)),
        _ if file_name.ends_with("proj") => Some(parse_dotnet(index, path, text)),
        _ => None,
    }
}

fn parse_cargo(index: &RepoIndex, path: &Path, text: &str) -> Option<Manifest> {
    let value = text.parse::<toml::Table>().ok()?;
    let mut manifest = Manifest::new(path, Ecosystem::Cargo);
    let dir = manifest.dir().to_path_buf();
    let package = value.get("package").and_then(Toml::as_table);

    manifest.name = package
        .and_then(|table| table.get("name"))
        .and_then(Toml::as_str)
        .map(str::to_string);
    manifest.description = package
        .and_then(|table| table.get("description"))
        .and_then(Toml::as_str)
        .map(clean_description);

    let bin_name = manifest.name.clone().unwrap_or_else(|| "main".to_string());
    for bin in value
        .get("bin")
        .and_then(Toml::as_array)
        .into_iter()
        .flatten()
    {
        let name = bin.get("name").and_then(Toml::as_str).unwrap_or(&bin_name);
        let target = bin
            .get("path")
            .and_then(Toml::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("src/bin/{name}.rs")));
        push_entry(
            index,
            &mut manifest,
            dir.join(target),
            format!("cargo bin `{name}`"),
        );
    }
    push_entry(
        index,
        &mut manifest,
        dir.join("src/main.rs"),
        format!("cargo bin `{bin_name}`"),
    );
    let lib_path = value
        .get("lib")
        .and_then(|lib| lib.get("path"))
        .and_then(Toml::as_str)
        .unwrap_or("src/lib.rs");
    push_entry(
        index,
        &mut manifest,
        dir.join(lib_path),
        "cargo library root".to_string(),
    );

    manifest.dependencies = table_keys(value.get("dependencies"));
    manifest.dev_dependencies = table_keys(value.get("dev-dependencies"));
    if let Some(workspace) = value.get("workspace") {
        manifest.members = string_array(workspace.get("members"));
        if manifest.dependencies.is_empty() {
            manifest.dependencies = table_keys(workspace.get("dependencies"));
        }
    }
    Some(manifest)
}

fn parse_package_json(index: &RepoIndex, path: &Path, text: &str) -> Option<Manifest> {
    let value: Json = serde_json::from_str(text).ok()?;
    let mut manifest = Manifest::new(path, Ecosystem::Npm);
    let dir = manifest.dir().to_path_buf();

    manifest.name = value.get("name").and_then(Json::as_str).map(str::to_string);
    manifest.description = value
        .get("description")
        .and_then(Json::as_str)
        .map(clean_description);

    match value.get("bin") {
        Some(Json::String(target)) => {
            let name = manifest.name.clone().unwrap_or_else(|| "bin".to_string());
            push_entry(
                index,
                &mut manifest,
                dir.join(target),
                format!("npm bin `{name}`"),
            );
        }
        Some(Json::Object(map)) => {
            for (name, target) in map {
                if let Some(target) = target.as_str() {
                    push_entry(
                        index,
                        &mut manifest,
                        dir.join(target),
                        format!("npm bin `{name}`"),
                    );
                }
            }
        }
        _ => {}
    }
    for key in ["main", "module"] {
        if let Some(target) = value.get(key).and_then(Json::as_str) {
            push_entry(
                index,
                &mut manifest,
                dir.join(target),
                format!("package `{key}`"),
            );
        }
    }
    if let Some(target) = export_root(value.get("exports")) {
        push_entry(
            index,
            &mut manifest,
            dir.join(target),
            "package export `.`".to_string(),
        );
    }

    if let Some(scripts) = value.get("scripts").and_then(Json::as_object) {
        manifest.scripts = scripts
            .iter()
            .filter_map(|(name, body)| Some((name.clone(), body.as_str()?.to_string())))
            .collect();
    }
    manifest.dependencies = json_keys(value.get("dependencies"));
    manifest.dev_dependencies = json_keys(value.get("devDependencies"));
    manifest.members = match value.get("workspaces") {
        Some(Json::Array(items)) => json_strings(items),
        Some(Json::Object(map)) => map
            .get("packages")
            .and_then(Json::as_array)
            .map(|items| json_strings(items))
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    Some(manifest)
}

fn export_root(exports: Option<&Json>) -> Option<&str> {
    let root = match exports? {
        Json::String(value) => return Some(value),
        Json::Object(map) => map.get(".").unwrap_or(exports?),
        _ => return None,
    };
    match root {
        Json::String(value) => Some(value),
        Json::Object(map) => ["source", "import", "default", "require"]
            .iter()
            .find_map(|key| match map.get(*key) {
                Some(Json::String(value)) => Some(value.as_str()),
                Some(Json::Object(nested)) => nested.get("default").and_then(Json::as_str),
                _ => None,
            }),
        _ => None,
    }
}

fn parse_deno(path: &Path, text: &str) -> Option<Manifest> {
    let cleaned = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let value: Json = serde_json::from_str(&cleaned).ok()?;
    let mut manifest = Manifest::new(path, Ecosystem::Deno);
    manifest.name = value.get("name").and_then(Json::as_str).map(str::to_string);
    if let Some(tasks) = value.get("tasks").and_then(Json::as_object) {
        manifest.scripts = tasks
            .iter()
            .filter_map(|(name, body)| {
                let body = body
                    .as_str()
                    .or_else(|| body.get("command").and_then(Json::as_str))?;
                Some((name.clone(), body.to_string()))
            })
            .collect();
    }
    Some(manifest)
}

fn parse_pyproject(index: &RepoIndex, path: &Path, text: &str) -> Option<Manifest> {
    let value = text.parse::<toml::Table>().ok()?;
    let mut manifest = Manifest::new(path, Ecosystem::Python);
    let project = value.get("project").and_then(Toml::as_table);
    let tool = value.get("tool").and_then(Toml::as_table);
    let poetry = tool
        .and_then(|table| table.get("poetry"))
        .and_then(Toml::as_table);

    manifest.name = project
        .and_then(|table| table.get("name"))
        .or_else(|| poetry.and_then(|table| table.get("name")))
        .and_then(Toml::as_str)
        .map(str::to_string);
    manifest.description = project
        .and_then(|table| table.get("description"))
        .or_else(|| poetry.and_then(|table| table.get("description")))
        .and_then(Toml::as_str)
        .map(clean_description);

    let scripts = project
        .and_then(|table| table.get("scripts"))
        .or_else(|| poetry.and_then(|table| table.get("scripts")))
        .and_then(Toml::as_table);
    for (name, target) in scripts.into_iter().flatten() {
        let Some(target) = target.as_str() else {
            continue;
        };
        if let Some(module_path) = resolve_python_module(index, manifest.dir(), target) {
            let reason = format!("console script `{name}`");
            push_entry(index, &mut manifest, module_path, reason);
        }
    }

    if let Some(name) = manifest.name.clone() {
        let package = name.to_ascii_lowercase().replace(['-', '.'], "_");
        for root in [
            manifest.dir().join("src").join(&package),
            manifest.dir().join(&package),
        ] {
            push_entry(
                index,
                &mut manifest,
                root.join("__init__.py"),
                format!("python package `{package}`"),
            );
        }
    }

    manifest.dependencies = string_array(project.and_then(|table| table.get("dependencies")))
        .iter()
        .map(|spec| python_requirement_name(spec))
        .filter(|name| !name.is_empty())
        .collect();
    if manifest.dependencies.is_empty() {
        manifest.dependencies = table_keys(poetry.and_then(|table| table.get("dependencies")))
            .into_iter()
            .filter(|name| name != "python")
            .collect();
    }
    if let Some(tool) = tool {
        manifest.tools = tool.keys().cloned().collect();
    }
    if value.contains_key("dependency-groups") {
        manifest.tools.push("dependency-groups".to_string());
    }
    if let Some(workspace) = tool
        .and_then(|table| table.get("uv"))
        .and_then(|uv| uv.get("workspace"))
    {
        manifest.members = string_array(workspace.get("members"));
    }
    Some(manifest)
}

/// `package.module:function` → the file defining `package.module`.
fn resolve_python_module(index: &RepoIndex, dir: &Path, target: &str) -> Option<PathBuf> {
    let module = target.split(':').next()?.trim();
    let relative = module.replace('.', "/");
    for base in [dir.to_path_buf(), dir.join("src")] {
        for candidate in [
            base.join(format!("{relative}.py")),
            base.join(&relative).join("__main__.py"),
            base.join(&relative).join("__init__.py"),
        ] {
            if index.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn python_requirement_name(spec: &str) -> String {
    spec.split(|ch: char| "<>=!~;[ (@".contains(ch))
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn parse_setup_py(path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Python);
    manifest.name = keyword_argument(text, "name");
    manifest.description =
        keyword_argument(text, "description").map(|value| clean_description(&value));
    manifest
}

fn keyword_argument(text: &str, key: &str) -> Option<String> {
    let start = text.find(&format!("{key}="))? + key.len() + 1;
    let rest = text[start..].trim_start();
    let quote = rest.chars().next().filter(|ch| *ch == '"' || *ch == '\'')?;
    let rest = &rest[1..];
    Some(rest[..rest.find(quote)?].to_string())
}

fn parse_requirements(path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Python);
    manifest.dependencies = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('-'))
        .map(python_requirement_name)
        .filter(|name| !name.is_empty())
        .collect();
    manifest
}

fn parse_go_mod(index: &RepoIndex, path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Go);
    let dir = manifest.dir().to_path_buf();
    manifest.name = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("module "))
        .map(|value| value.trim().to_string());

    let mut in_require = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("require (") {
            in_require = true;
            continue;
        }
        if in_require && line == ")" {
            in_require = false;
            continue;
        }
        let spec = if in_require {
            Some(line)
        } else {
            line.strip_prefix("require ")
        };
        if let Some(spec) = spec {
            if spec.contains("// indirect") {
                continue;
            }
            if let Some(module) = spec.split_whitespace().next() {
                manifest.dependencies.push(module.to_string());
            }
        }
    }

    push_entry(
        index,
        &mut manifest,
        dir.join("main.go"),
        "go main package".to_string(),
    );
    let cmd_dir = dir.join("cmd");
    let mut commands = index
        .files
        .iter()
        .filter(|entry| {
            paths::file_name(&entry.path) == "main.go"
                && entry
                    .path
                    .parent()
                    .and_then(Path::parent)
                    .is_some_and(|parent| parent == cmd_dir)
        })
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    commands.sort();
    for command in commands {
        let name = command
            .parent()
            .map(|parent| paths::file_name(parent).to_string())
            .unwrap_or_default();
        push_entry(
            index,
            &mut manifest,
            command,
            format!("go command `{name}`"),
        );
    }
    manifest
}

fn parse_pom(index: &RepoIndex, path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Maven);
    let without_parent = strip_xml_block(text, "parent");
    let without_deps = strip_xml_block(&strip_xml_block(&without_parent, "dependencies"), "build");
    manifest.name = xml_value(&without_deps, "artifactId");
    manifest.description =
        xml_value(&without_deps, "description").map(|value| clean_description(&value));
    manifest.members = xml_values(&without_deps, "module");
    manifest.dependencies = xml_values(&without_parent, "artifactId")
        .into_iter()
        .skip(1)
        .take(40)
        .collect();
    push_java_mains(index, &mut manifest);
    manifest
}

fn parse_gradle(index: &RepoIndex, path: &Path) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Gradle);
    let dir = manifest.dir().to_path_buf();
    for settings in ["settings.gradle", "settings.gradle.kts"] {
        let Some(text) = index.read(dir.join(settings)) else {
            continue;
        };
        for line in text.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("rootProject.name") {
                manifest.name = quoted_values(rest).into_iter().next();
            } else if line.starts_with("include") {
                manifest.members.extend(
                    quoted_values(line)
                        .into_iter()
                        .map(|value| value.trim_start_matches(':').replace(':', "/")),
                );
            }
        }
    }
    push_java_mains(index, &mut manifest);
    manifest
}

/// Spring Boot style `*Application.java` / `Main.java` classes under this module.
fn push_java_mains(index: &RepoIndex, manifest: &mut Manifest) {
    let main_dir = manifest.dir().join("src/main");
    let mut mains = index
        .files
        .iter()
        .filter(|entry| entry.path.starts_with(&main_dir))
        .filter(|entry| {
            let name = paths::file_name(&entry.path);
            matches!(name, "Main.java" | "Main.kt" | "Application.kt")
                || name.ends_with("Application.java")
                || name.ends_with("Application.kt")
        })
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    mains.sort_by_key(|path| paths::depth(path));
    for main in mains.into_iter().take(2) {
        push_entry(index, manifest, main, "application main class".to_string());
    }
}

fn parse_gemfile(path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Ruby);
    manifest.dependencies = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("gem "))
        .filter_map(|rest| quoted_values(rest).into_iter().next())
        .collect();
    manifest
}

fn parse_composer(path: &Path, text: &str) -> Option<Manifest> {
    let value: Json = serde_json::from_str(text).ok()?;
    let mut manifest = Manifest::new(path, Ecosystem::Php);
    manifest.name = value.get("name").and_then(Json::as_str).map(str::to_string);
    manifest.description = value
        .get("description")
        .and_then(Json::as_str)
        .map(clean_description);
    if let Some(scripts) = value.get("scripts").and_then(Json::as_object) {
        manifest.scripts = scripts
            .iter()
            .filter_map(|(name, body)| {
                let body = match body {
                    Json::String(value) => value.clone(),
                    Json::Array(items) => items.first()?.as_str()?.to_string(),
                    _ => return None,
                };
                Some((name.clone(), body))
            })
            .collect();
    }
    manifest.dependencies = json_keys(value.get("require"));
    manifest.dev_dependencies = json_keys(value.get("require-dev"));
    Some(manifest)
}

fn parse_swift(index: &RepoIndex, path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Swift);
    manifest.name = text
        .find("name:")
        .and_then(|start| quoted_values(&text[start..]).into_iter().next());
    let sources = manifest.dir().join("Sources");
    let mains = index
        .files
        .iter()
        .filter(|entry| {
            entry.path.starts_with(&sources) && paths::file_name(&entry.path) == "main.swift"
        })
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    for main in mains {
        push_entry(
            index,
            &mut manifest,
            main,
            "swift executable target".to_string(),
        );
    }
    manifest
}

fn parse_pubspec(index: &RepoIndex, path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Dart);
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            manifest.name = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            manifest.description = Some(clean_description(value.trim().trim_matches('"')));
        }
    }
    let main = manifest.dir().join("lib/main.dart");
    push_entry(index, &mut manifest, main, "flutter/dart main".to_string());
    manifest
}

fn parse_mix(path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Elixir);
    manifest.name = text.find("app:").map(|start| {
        text[start + 4..]
            .trim_start()
            .trim_start_matches(':')
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
            .collect()
    });
    manifest
}

fn parse_cmake(path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::CMake);
    let lower = text.to_ascii_lowercase();
    if let Some(start) = lower.find("project(") {
        let rest = &text[start + "project(".len()..];
        manifest.name = rest
            .split(|ch: char| ch.is_whitespace() || ch == ')')
            .find(|part| !part.is_empty())
            .map(str::to_string);
    }
    manifest
}

fn parse_dotnet(index: &RepoIndex, path: &Path, text: &str) -> Manifest {
    let mut manifest = Manifest::new(path, Ecosystem::Dotnet);
    manifest.name = xml_value(text, "AssemblyName").or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .map(str::to_string)
    });
    manifest.description = xml_value(text, "Description").map(|value| clean_description(&value));
    manifest.dependencies = text
        .match_indices("PackageReference Include=\"")
        .filter_map(|(start, needle)| {
            let rest = &text[start + needle.len()..];
            Some(rest[..rest.find('"')?].to_string())
        })
        .collect();
    let program = manifest.dir().join("Program.cs");
    push_entry(
        index,
        &mut manifest,
        program,
        ".NET program entry".to_string(),
    );
    manifest
}

fn push_entry(index: &RepoIndex, manifest: &mut Manifest, path: PathBuf, reason: String) {
    let path = normalize(&path);
    if !index.contains(&path)
        || manifest
            .entry_points
            .iter()
            .any(|(known, _)| *known == path)
    {
        return;
    }
    manifest.entry_points.push((path, reason));
}

/// Collapse `./` and `..` segments without touching the file system.
pub fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other.as_os_str().to_os_string()),
        }
    }
    parts.iter().collect()
}

fn clean_description(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn table_keys(value: Option<&Toml>) -> Vec<String> {
    value
        .and_then(Toml::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

fn string_array(value: Option<&Toml>) -> Vec<String> {
    value
        .and_then(Toml::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Toml::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn json_keys(value: Option<&Json>) -> Vec<String> {
    value
        .and_then(Json::as_object)
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default()
}

fn json_strings(items: &[Json]) -> Vec<String> {
    items
        .iter()
        .filter_map(Json::as_str)
        .map(str::to_string)
        .collect()
}

fn quoted_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = text.char_indices();
    while let Some((start, ch)) = chars.next() {
        if ch != '"' && ch != '\'' {
            continue;
        }
        let rest = &text[start + 1..];
        if let Some(end) = rest.find(ch) {
            values.push(rest[..end].to_string());
            for _ in 0..=rest[..end].chars().count() {
                chars.next();
            }
        }
    }
    values
}

fn xml_value(text: &str, tag: &str) -> Option<String> {
    xml_values(text, tag).into_iter().next()
}

fn xml_values(text: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(&close) else {
            break;
        };
        let value = after[..end].trim();
        if !value.is_empty() && !value.contains('<') {
            values.push(value.to_string());
        }
        rest = &after[end + close.len()..];
    }
    values
}

fn strip_xml_block(text: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        output.push_str(&rest[..start]);
        match rest[start..].find(&close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    output.push_str(rest);
    output
}

/// Group nested manifests into workspace patterns like `packages/*`.
pub fn workspace_groups(manifests: &[Manifest]) -> BTreeMap<String, Vec<&Manifest>> {
    let mut groups: BTreeMap<String, Vec<&Manifest>> = BTreeMap::new();
    for manifest in manifests {
        let dir = manifest.dir();
        if dir.as_os_str().is_empty() || manifest.ecosystem == Ecosystem::CMake {
            continue;
        }
        let parent = dir.parent().unwrap_or_else(|| Path::new(""));
        let pattern = if parent.as_os_str().is_empty() {
            paths::display(dir)
        } else {
            format!("{}/*", paths::display(parent))
        };
        let group = groups.entry(pattern).or_default();
        // One package directory can carry several manifests (package.json + deno.json).
        if !group.iter().any(|known| known.dir() == dir) {
            group.push(manifest);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_requirement_names_drop_version_specs() {
        assert_eq!(python_requirement_name("fastapi>=0.100"), "fastapi");
        assert_eq!(
            python_requirement_name("uvicorn[standard] ; python_version>'3'"),
            "uvicorn"
        );
    }

    #[test]
    fn xml_blocks_are_stripped_before_reading_artifact() {
        let pom = "<project><parent><artifactId>boot</artifactId></parent><artifactId>app</artifactId></project>";
        let stripped = strip_xml_block(pom, "parent");
        assert_eq!(xml_value(&stripped, "artifactId").as_deref(), Some("app"));
    }

    #[test]
    fn quoted_values_are_extracted_in_order() {
        assert_eq!(
            quoted_values("include(':app', \"lib:core\")"),
            vec![":app".to_string(), "lib:core".to_string()]
        );
    }
}
