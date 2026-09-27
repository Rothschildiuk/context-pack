//! Path classification shared by every stage: language detection and the
//! "role" of a path (production source, tests, examples, docs, ...).

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathRole {
    Source,
    Tests,
    Fixtures,
    Examples,
    Docs,
    Benchmarks,
    Scripts,
    Ci,
    Generated,
    Vendor,
    Hidden,
}

impl PathRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Tests => "tests",
            Self::Fixtures => "fixtures",
            Self::Examples => "examples",
            Self::Docs => "docs",
            Self::Benchmarks => "benchmarks",
            Self::Scripts => "scripts",
            Self::Ci => "ci",
            Self::Generated => "generated",
            Self::Vendor => "vendor",
            Self::Hidden => "hidden",
        }
    }

    /// Roles whose code should not be proposed as a starting point.
    pub fn is_secondary(self) -> bool {
        !matches!(self, Self::Source)
    }
}

/// Classify a repo-relative path by its directory components and file name.
/// The first matching component wins, so `tests/fixtures/x` is `Tests`.
pub fn role(path: &Path) -> PathRole {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect::<Vec<_>>();
    let Some((file_name, dirs)) = components.split_last() else {
        return PathRole::Source;
    };

    for (index, dir) in dirs.iter().enumerate() {
        // Maven/Gradle layout: everything under `src/main/<lang>/` is a package
        // path (`org/example/samples/...`), not a directory with a role.
        if dir == "main"
            && dirs
                .get(index + 1)
                .is_some_and(|next| is_jvm_source_root(next))
        {
            return PathRole::Source;
        }
        if let Some(role) = dir_role(dir, index == 0) {
            return role;
        }
    }

    if is_test_file_name(file_name) {
        return PathRole::Tests;
    }
    if is_generated_file_name(file_name) {
        return PathRole::Generated;
    }

    PathRole::Source
}

fn is_jvm_source_root(dir: &str) -> bool {
    matches!(dir, "java" | "kotlin" | "scala" | "groovy" | "resources")
}

fn dir_role(dir: &str, top_level: bool) -> Option<PathRole> {
    let role = match dir {
        "tests" | "test" | "__tests__" | "spec" | "specs" | "e2e" | "integration-tests"
        | "integration_tests" | "testing" | "fuzz" | "fuzzing" => PathRole::Tests,
        "fixtures" | "fixture" | "testdata" | "test-data" | "test_data" | "snapshots"
        | "__snapshots__" | "__fixtures__" | "__mocks__" | "mocks" => PathRole::Fixtures,
        "examples" | "example" | "samples" | "sample" | "demo" | "demos" | "docs_src"
        | "playground" | "playgrounds" | "sandbox" | "starters" | "templates" | "cookbook" => {
            PathRole::Examples
        }
        "docs" | "doc" | "documentation" | "website" | "site" | "book" => PathRole::Docs,
        "bench" | "benches" | "benchmark" | "benchmarks" => PathRole::Benchmarks,
        "scripts" | "script" | "tools" | "tooling" | "hack" | "devtools" => PathRole::Scripts,
        ".github" | ".gitlab" | ".circleci" | ".buildkite" => PathRole::Ci,
        "generated" | "__generated__" | "gen" | "codegen" => PathRole::Generated,
        "vendor" | "vendored" | "third_party" | "third-party" | "thirdparty" | "external"
        | "node_modules" | "bower_components" => PathRole::Vendor,
        // Only treat a bare `tools`/`site` style name as secondary when it is
        // not nested inside a package (handled above), and any other dotted
        // directory as hidden tooling config.
        _ if dir.starts_with('.') && dir.len() > 1 => PathRole::Hidden,
        _ => return None,
    };

    // `docs/` or `scripts/` nested deep inside a package is still secondary,
    // but a nested `site`/`book`/`external` directory is often real code.
    if !top_level && matches!(dir, "site" | "book" | "external" | "templates" | "tools") {
        return None;
    }
    Some(role)
}

pub fn is_test_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    let stem = lower.split('.').next().unwrap_or(&lower);
    lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains("_test.")
        || lower.contains("_spec.")
        || (stem.starts_with("test_") && lower.ends_with(".py"))
        || stem == "conftest"
        || file_name.ends_with("Test.java")
        || file_name.ends_with("Tests.java")
        || file_name.ends_with("Test.kt")
        || file_name.ends_with("Tests.cs")
        || file_name.ends_with("Tests.swift")
}

fn is_generated_file_name(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".min.js")
        || lower.ends_with(".min.css")
        || lower.ends_with(".pb.go")
        || lower.ends_with("_pb2.py")
        || lower.ends_with("_pb2_grpc.py")
        || lower.contains(".generated.")
        || lower.ends_with(".g.dart")
        || lower.ends_with(".d.ts")
        || lower.ends_with(".map")
}

/// Map a path to a programming language name. Only languages that make sense
/// as "source code" are returned; markup and data files return `None`.
pub fn language(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let language = match ext.as_str() {
        "rs" => "rust",
        "go" => "go",
        "py" | "pyi" => "python",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "scala" | "sc" => "scala",
        "groovy" => "groovy",
        "cs" => "csharp",
        "fs" | "fsx" => "fsharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "m" | "mm" => "objective-c",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "zig" => "zig",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "clj" | "cljs" | "cljc" => "clojure",
        "dart" => "dart",
        "lua" => "lua",
        "r" => "r",
        "jl" => "julia",
        "nim" => "nim",
        "cr" => "crystal",
        "sol" => "solidity",
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" => "powershell",
        "sql" => "sql",
        "tf" => "terraform",
        "nix" => "nix",
        "xq" | "xqy" | "xql" | "xqm" => "xquery",
        "xsl" | "xslt" => "xslt",
        _ => return None,
    };
    Some(language)
}

/// Languages that describe the product itself rather than glue around it.
pub fn is_primary_language(language: &str) -> bool {
    !matches!(
        language,
        "shell" | "powershell" | "sql" | "terraform" | "nix"
    )
}

pub fn is_source(path: &Path) -> bool {
    language(path).is_some_and(|language| !matches!(language, "sql"))
}

pub fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
}

pub fn depth(path: &Path) -> usize {
    path.components().count().saturating_sub(1)
}

/// Render a relative path with forward slashes for stable output everywhere.
pub fn display(path: &Path) -> String {
    let mut output = String::new();
    for (index, component) in path.components().enumerate() {
        if index > 0 {
            output.push('/');
        }
        output.push_str(&component.as_os_str().to_string_lossy());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_follow_first_meaningful_component() {
        assert_eq!(role(Path::new("src/main.rs")), PathRole::Source);
        assert_eq!(role(Path::new("tests/fixtures/a.rs")), PathRole::Tests);
        assert_eq!(role(Path::new("docs_src/app/main.py")), PathRole::Examples);
        assert_eq!(
            role(Path::new("examples/basics/package.json")),
            PathRole::Examples
        );
        assert_eq!(role(Path::new("pkg/api/handler_test.go")), PathRole::Tests);
        assert_eq!(role(Path::new("web/src/App.test.tsx")), PathRole::Tests);
        assert_eq!(role(Path::new(".github/workflows/ci.yml")), PathRole::Ci);
        assert_eq!(role(Path::new("third_party/lib.c")), PathRole::Vendor);
        assert_eq!(role(Path::new("api/gen/types.pb.go")), PathRole::Generated);
        assert_eq!(
            role(Path::new("packages/site/src/index.ts")),
            PathRole::Source
        );
        assert_eq!(
            role(Path::new(
                "src/main/java/org/springframework/samples/petclinic/App.java"
            )),
            PathRole::Source
        );
        assert_eq!(
            role(Path::new("src/test/java/org/example/AppTest.java")),
            PathRole::Tests
        );
        assert_eq!(
            role(Path::new("fuzz/fuzz_targets/glob.rs")),
            PathRole::Tests
        );
    }

    #[test]
    fn languages_cover_common_ecosystems() {
        assert_eq!(language(Path::new("a/b.rs")), Some("rust"));
        assert_eq!(language(Path::new("x.tsx")), Some("typescript"));
        assert_eq!(language(Path::new("Program.cs")), Some("csharp"));
        assert_eq!(language(Path::new("README.md")), None);
    }
}
