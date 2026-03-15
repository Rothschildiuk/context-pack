# Context Pack

## Agent Briefing
### What This Repo Is
- Likely a Rust project with Cargo-based entry points.
- Primary languages: rust, python.

### Active Work
- No high-signal changes detected

### Read These First
- `src/diff.rs`: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- `src/model.rs`: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- `src/render_markdown.rs`: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- `src/walk.rs`: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint

### Likely Entry Points
- none

### Large Code Files
- `src/select.rs` (1970 LOC) : large production source file
- `src/select.rs` (1970 LOC) : large explicitly included source file

### Caveats
- No AGENTS.md found.
- README was omitted as low-signal or placeholder-heavy.

## Repo
- path: /Users/olehbaidiuk/MyProjects/context-pack
- project types: rust, python
- primary languages: rust, python

## Git Changes
- current branch: `main`
- local branches: `main`
- upstream branch: `origin/main`
- default branch: `main`
- primary development branch likely `main`
- relative to `origin/main`: ahead 0, behind 1

No high-signal changes detected.

## Important Files
### src/diff.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, compact file bonus, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 875
- truncated: true
- redacted: true
- redaction reason: potential secrets redacted

```text
fn json_key_diff(left: [REDACTED]
    let left = serde_json::from_str::<Value>(left).ok()?;
    let right = serde_json::from_str::<Value>(right).ok()?;
... [truncated]
```

### src/model.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, compact file bonus, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 875
- truncated: true

```text
pub struct AppConfig {
    pub cwd: PathBuf,
    pub format: OutputFormat,
...
pub enum OutputFormat {
    Markdown,
    Json,
... [truncated]
```

### src/render_markdown.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, compact file bonus, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 875
- truncated: true

```text
pub fn render(context: &RenderContext) -> String {
    let mut output = String::new();
... [truncated]
```

### src/walk.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, compact file bonus, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 875
- truncated: true

```text
pub fn build_tree_summary_with_matcher(
    config: &AppConfig,
    matcher: &IgnoreMatcher,
...
fn visit_dir(
    absolute_dir: &Path,
    relative_dir: &Path,
... [truncated]
```

### src/briefing.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true

```text
pub fn build(
    config: &AppConfig,
    repo: &RepoInfo,
...
fn build_read_these_first(files: &[ImportantFile]) -> Vec<BriefingItem> {
    let mut ordered = files
        .iter()
... [truncated]
```

### src/cli.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true

```text
pub fn parse_args<I>(args: I) -> Result<AppConfig, CliError>
...
fn next_value<I>(iter: &mut I, flag: &'static str) -> Result<String, CliError>
... [truncated]
```

### src/dependency_summary.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true
- redacted: true
- redaction reason: potential secrets redacted

```text
pub fn collect(config: &AppConfig, files: &[ImportantFile], budget: usize) -> Vec<String> {
    let mut manifests = files
        .iter()
... [truncated]
```

### src/detect.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true

```text
pub fn detect_repo_info_with_matcher(
    config: &AppConfig,
    files: &[ImportantFile],
...
impl DetectionState {
    fn new() -> Self {
        Self {
... [truncated]
```

### src/docker_summary.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true
- redacted: true
- redaction reason: potential secrets redacted

```text
pub fn collect(config: &AppConfig, files: &[ImportantFile], budget: usize) -> Vec<String> {
    let mut candidates = root_docker_candidates(config);
... [truncated]
```

### src/git.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true

```text
pub fn collect(config: &AppConfig, summary_budget: usize) -> GitResult {
    if config.no_git {
        return GitResult {
... [truncated]
```

### src/ignore.rs
- reason: explicitly included source file, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- why: explicitly included source file, shallow path priority, explicit include, language-aware boost (rust, top-1), referenced by active work or entrypoint
- category: included_source
- score: 855
- truncated: true

```text
pub struct IgnoreMatcher {
    root: PathBuf,
    rules: Vec<Rule>,
...
    fn matches_include_rule(
        &self,
        relative_path: &Path,
... [truncated]
```

## Tree
context-pack/
  .codex-plugin/
    plugin.json
  .github/
    ISSUE_TEMPLATE/
      agent_task.yml
      bug_report.yml
      config.yml
      feature_request.yml
    release.yml
    workflows/
      agent_ci.yml
      release.yml
  .gitignore
  .mcp.json
  AGENTS.md
  CHANGELOG.md
  CONTEXT_PACK_PLAN.md
  CONTRIBUTING.md
  Cargo.lock
  Cargo.toml
  Formula/
    context-pack.rb
  GITHUB_PLAYBOOK.md

## Notes
- max bytes: 4000
- approx tokens: 2121
- elapsed_ms: 171
- max files: 12
- max depth: 4
- budget split: briefing=900, git=500, excerpts=1800, tree=800
- ignored entries: 5
- tree entries omitted by limit: 13
- git changes omitted as low-signal noise
- selected files: 11
- language-aware scoring: top languages = rust, python
- files scanned for selection: 75
