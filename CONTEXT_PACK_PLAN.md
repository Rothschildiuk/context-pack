# context-pack Plan

## Goal

Build an open-source CLI in Zig that turns a repository into a compact, high-signal context bundle for coding agents.

Proposed GitHub repository:

- `oleh/context-pack`

## Product Definition

`context-pack` should scan a repository and produce a bundle that helps an agent understand:

- repository structure
- key docs and manifest files
- current git changes
- relevant source layout
- high-value context with low noise

Primary output target:

- plain text / Markdown bundle for copy-paste into agent prompts

Later output target:

- JSON for editor integrations and automation

## Core Use Cases

1. Run inside an unfamiliar repo and get a compact overview in seconds.
2. Run before asking an AI agent to modify code.
3. Run with `--changed-only` to focus on active work.
4. Generate consistent repository context across repeated runs.

## MVP Scope

The first usable version should support:

- file tree summary
- detection of important files
- extraction of trimmed content from important files
- git status summary
- sane ignore rules
- output to stdout

Suggested initial commands:

```bash
context-pack
context-pack --changed-only
context-pack --format markdown
context-pack --max-bytes 4000
```

## MVP Success Criteria

The MVP is good enough when:

- it produces useful output on small and medium repositories without manual tuning
- changed files are easy to spot in the bundle
- `README.md`, `AGENTS.md`, and core manifest files are included when present
- output is deterministic across repeated runs on the same repository state
- truncation is explicit rather than silent
- failure to read git metadata does not break the main output

## Non-Goals

Not part of the MVP:

- AST parsing or semantic code analysis
- exact token counting
- editor plugins or IDE integrations
- config file support
- language-specific heuristics beyond simple detection and ranking
- perfect representation of every repository type

## High-Priority Inputs

Always look for these first if present:

- `AGENTS.md`
- `README.md`
- `README`
- `package.json`
- `pyproject.toml`
- `requirements.txt`
- `Cargo.toml`
- `go.mod`
- `pom.xml`
- `build.gradle`
- `Makefile`
- `docker-compose.yml`
- `.env.example`

Nice-to-have later:

- `ARCHITECTURE.md`
- `CONTRIBUTING.md`
- `Justfile`
- `Taskfile.yml`
- `turbo.json`
- workspace config files

## Selection and Ranking Rules

When the repository is larger than the output budget, rank context in this order:

1. user guidance files such as `AGENTS.md`
2. top-level project docs such as `README.md`
3. core manifests and build files
4. changed files
5. small entrypoint or root-level source files
6. additional supporting docs

Selection rules:

- prefer files near the repository root over deep files
- prefer changed files over unchanged files when scores are close
- prefer smaller files that fit the budget cleanly
- include file path, reason for inclusion, and truncation marker when applicable
- stop adding files when the byte budget is exhausted

## Output Structure

Recommended Markdown bundle layout:

```text
# Context Pack

## Repo
- path
- detected project types
- primary languages

## Tree
... trimmed tree ...

## Important Files
### README.md
... excerpt ...

### package.json
... excerpt ...

## Git Changes
... git status --short ...

## Notes
- omitted directories
- truncation summary
```

Recommended JSON top-level fields for later:

- `repo`
- `project_types`
- `languages`
- `tree`
- `important_files`
- `git`
- `omissions`
- `limits`

## Noise Reduction Rules

Ignore by default:

- `.git`
- `node_modules`
- `dist`
- `build`
- `.next`
- `.turbo`
- `.venv`
- `venv`
- `coverage`
- `target`
- `out`
- `.idea`
- `.vscode`
- binary assets larger than threshold

Honor:

- `.gitignore`
- `.ignore` if present
- user-provided include/exclude globs

Trim aggressively:

- lockfiles unless explicitly requested
- generated files
- minified files
- huge vendored directories

## Failure Modes and Fallbacks

The tool should degrade predictably:

- if git is unavailable, omit git sections and continue
- if a file is binary, mark it as omitted instead of trying to render it
- if a file is too large, include a short excerpt with a truncation marker
- if the repository is huge, stop traversal early based on limits and report omissions
- if decoding fails, note the file and continue

## CLI Design

Suggested flags for MVP:

- `--format markdown|json`
- `--output <path>`
- `--changed-only`
- `--max-bytes <n>`
- `--max-files <n>`
- `--max-depth <n>`
- `--include <glob>`
- `--exclude <glob>`
- `--no-git`
- `--no-tree`
- `--cwd <path>`

Possible later flags:

- `--summary`
- `--token-budget <n>`
- `--priority <file>`
- `--config <path>`

## Zig Architecture

Keep the implementation simple and explicit:

- `main.zig`
- `cli.zig` for argument parsing
- `walk.zig` for filesystem traversal
- `ignore.zig` for ignore matching
- `detect.zig` for project/manifests/language detection
- `extract.zig` for file reading and trimming
- `git.zig` for shelling out to git
- `render_markdown.zig`
- `render_json.zig`
- `budget.zig` for truncation and selection budgeting

Implementation principles:

- single static binary
- low memory use
- deterministic output ordering
- explicit truncation markers
- graceful fallback if git is unavailable

## Testing Strategy

Use fixture-based tests first.

Minimum test coverage:

- tree generation
- ignore handling
- manifest detection
- ranking behavior
- trimming behavior
- stable output ordering
- git status parsing
- changed-only mode
- empty repo behavior
- non-git repo behavior

Golden tests:

- snapshot Markdown output
- snapshot JSON output later

## First Vertical Slice

Build one thin end-to-end path first:

1. Scaffold the Zig CLI and argument parsing.
2. Walk the repository with default ignore rules.
3. Detect top-priority files.
4. Extract trimmed content from selected files.
5. Collect git status when available.
6. Render one Markdown bundle to stdout.
7. Test on 2 to 3 real repositories.

Do not start with release automation or editor integration.

## Phased Roadmap

### Phase 1: MVP

- initialize Zig CLI project
- implement directory walk
- implement important file detection
- implement ranking and budgeting
- implement trimmed file extraction
- implement git status collection
- render Markdown output

### Phase 2: Quality

- improve ignore rules
- add fixture repos and golden tests
- benchmark on medium and large repos
- refine selection heuristics on real repositories

### Phase 3: Distribution

- GitHub Actions build workflow
- tagged releases
- prebuilt binaries
- install docs

### Phase 4: Ecosystem

- JSON output
- config file support
- shell completions
- editor integration
- token-budget mode

## Open Questions

Default decisions unless proven wrong:

- optimize default output for agents first, but keep it readable for humans
- use byte-based limits in the MVP
- postpone exact token estimation
- boost changed files in ranking by default

Still open:

- should `.context-pack.toml` exist at all, or only after the CLI stabilizes?
- should changed-only mode also shrink the tree output automatically?

## Recommended Positioning

Short pitch:

> `context-pack` is a fast CLI that turns a repository into a compact, high-signal context bundle for coding agents.

One-line value proposition:

> Less prompt assembly, less repo spelunking, better agent context.

## Release Notes for Later

Once the core behavior is stable:

- create the GitHub repo
- add `README.md`
- add `LICENSE`
- add `.gitignore`
- add CI for builds and releases
- publish macOS and Linux binaries
