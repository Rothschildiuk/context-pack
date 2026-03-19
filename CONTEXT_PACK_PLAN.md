# context-pack Roadmap

## Current Product

`context-pack` is an open-source Rust CLI that generates a compact, high-signal repository briefing for coding agents.

It is designed for the first pass through an unfamiliar codebase, especially when the default agent behavior is too noisy:

- reading too many files too early
- missing repo guidance and entrypoints
- wasting prompt budget on low-signal exploration
- failing to form a good initial working set

The core product idea is no longer "repository summarization" in the abstract. The sharper framing is:

> `context-pack` is the repo briefing layer for coding agents.

## What Is Already True

The project is past the "MVP idea" stage. It already has:

- a working Rust CLI
- Markdown and JSON output modes
- changed-only mode
- output budgeting and selection heuristics
- fixture-based tests and markdown snapshot coverage
- release automation and Homebrew distribution

This means the main question is no longer "can this tool exist?"

The real questions now are:

- is the positioning sharp enough?
- does the output reliably help agents on real repositories?
- which signals matter most in medium and large unfamiliar repos?
- how should the tool prove its value against manual repo exploration, repo instructions, and RAG/indexing approaches?

## Product Thesis

The strongest use case for `context-pack` is:

- medium or large unfamiliar repositories
- older or messier codebases
- agent workflows with weak memory or no persistent repo learning
- fresh-thread workflows that repeatedly repay repo orientation cost
- prompt-sensitive environments where dumping raw repo state is too expensive

The product should be optimized for the first 5 to 10 minutes of repo orientation.

That also means the product should optimize for context density, not maximum context volume.

That means success looks like:

- the agent finds the right entrypoint faster
- the agent sees repo guidance before wandering
- the first prompt is smaller and higher-signal
- fewer wrong-file edits happen during early exploration

## Validated Signals So Far

From direct usage and early external feedback, these pain points appear real:

- noisy first-pass repo exploration wastes tokens
- agents often miss the actual entrypoint on the first attempt
- teams sometimes build manual "compact first-pass summaries" already
- the cost difference between "dump everything" and "structured first pass" can be material on large repos
- fresh threads in agent tools often repay repo orientation cost from scratch

These signals matter more right now than micro-optimizing runtime.

## Current Priorities

Priority order for the next phase:

1. prove value more clearly
2. improve briefing quality on real repositories
3. sharpen product positioning and documentation
4. optimize performance where it affects adoption

## Near-Term Roadmap

### 1. Validation and Proof

Turn anecdotal interest into reusable proof points.

Target outcomes:

- demonstrate that `context-pack` reduces noisy repo exploration
- show that it improves first-pass file selection
- produce concrete before/after examples
- gather evidence around token, cost, or prompt-size savings
- show value in fresh-thread workflows where repo context gets reloaded repeatedly

Planned work:

- create 3 to 5 realistic comparison cases on unfamiliar repos
- document "without context-pack" vs "with context-pack"
- capture where agents chose the wrong files before the briefing
- record approximate prompt-size or token-budget differences
- include at least one Codex/ChatGPT-style fresh-thread case
- turn the best cases into README and launch-post assets

### 2. Briefing Quality

Improve the output where it changes agent behavior, not just where it looks nicer.

Target outcomes:

- better entrypoint detection
- stronger selection of guidance docs and supporting docs
- less low-signal config and workspace noise
- more consistent output on medium and large repos

Planned work:

- refine ranking heuristics using real repos
- keep expanding fixture coverage for selection edge cases
- add more tests around noisy repositories and older layouts
- improve support for high-signal shared config files when they matter
- tighten omission and truncation reporting when budgets get small

### 3. Positioning and Docs

Keep the README and surrounding docs aligned with how the tool is actually being understood.

Target outcomes:

- clearer differentiation from `tree + rg + git diff`
- clearer differentiation from RAG and repo indexers
- stronger "first-pass repo briefing" framing
- better public explanation of where the tool is useful and where it is not

Planned work:

- keep README centered on pain, contrast, and before/after examples
- add a validation section once stronger proof points exist
- update public examples to show real agent workflows
- make the roadmap and product docs reflect current Rust implementation and shipping scope

### 4. Performance and Scale

Performance matters, but it is not the main product bottleneck unless it blocks use on real repositories.

Target outcomes:

- fast enough runs on medium and large repos
- predictable behavior under tight output budgets
- no obvious traversal or ranking regressions

Planned work:

- benchmark on representative repositories
- identify slow paths in traversal, selection, and rendering
- optimize only after measuring real bottlenecks
- preserve deterministic output while improving speed

### 5. Layered Context and Memory Freshness

Agents do not need one giant blob of repository state. They need the right layers at the right time.

Target outcomes:

- separate stable repo rules from volatile active work
- expose retrieval targets explicitly instead of forcing full-repo injection
- make learned repo memory visibly fresh or stale
- support fresh-thread handoff without pretending to be a full memory runtime

Planned work:

- draft a layered structured schema for `stable`, `active`, `retrieval`, and `decision` context
- preserve creation and refresh timestamps inside `.context-pack/memory.md`
- warn when repo memory is older than 7 days and repository activity continued
- keep the current Markdown/JSON/Viking outputs compatible while testing layered additions
- validate whether layered outputs improve agent behavior more than simply increasing budget

## Product Work That Matters Most

If there is only time for a few things, focus on these:

- before/after evaluation cases
- stronger evidence of token and cost savings
- better heuristics for finding entrypoints and guidance docs
- higher confidence on old, messy, or medium/large repos
- strong support for repeated fresh-thread orientation flows
- explicit separation of stable context from active context where it improves handoff quality

This is more important than adding a long tail of new flags.

## Lower-Priority Work

Useful, but not the best focus right now:

- shell completions
- extensive config file support
- editor plugins
- token estimation accuracy beyond rough budgeting
- broad language-specific logic that is not clearly improving first-pass outcomes

## Non-Goals For Now

Not the main goal in the current phase:

- replacing `rg`, `git`, or editor navigation
- becoming a full semantic code search tool
- competing head-on with RAG systems on deep retrieval
- building a full long-lived repo memory runtime
- perfect coverage for every repository shape

Memory-aware output is in scope. Owning the entire agent memory architecture is not.

## Roadmap Questions

Questions worth answering through real usage:

- which signals are most predictive on old or messy repositories?
- how much briefing detail is needed before output becomes too large to be useful?
- when does changed-only mode help versus over-narrow the briefing?
- which files are currently over-selected or under-selected?
- what is the best benchmark for "agent got oriented faster"?
- how much repeated token spend can be avoided in fresh-thread workflows?

## Success Criteria For The Next Phase

The next phase is successful if:

- the README explains the niche in one pass
- at least a few public examples show clear before/after value
- real users can explain why they would use this instead of manual repo exploration
- output quality improves on medium and large repos without becoming bloated
- performance is good enough that speed is not the main objection
- there is credible evidence that the tool reduces repeated repo-orientation cost in new threads

## Working Positioning

Short version:

> `context-pack` gives coding agents a compact first-pass repo briefing.

Expanded version:

> `context-pack` helps agents stop wandering in unfamiliar repos by surfacing the right docs, entrypoints, manifests, and active changes before deep exploration begins.

Additional framing:

> `context-pack` helps reduce repeated token spend by turning repo orientation into a compact first-pass briefing that can be reused across fresh threads.

Memory-aware framing:

> `context-pack` should emit stable context, active context, and retrieval hints as distinct layers so coding agents can stay oriented without carrying the whole repo at once.

## Immediate Next Steps

- collect and formalize 3 to 5 evaluation examples
- turn user feedback into a small validation section in the docs
- continue improving selection heuristics where they affect first-pass accuracy
- explicitly measure repeated orientation cost in fresh-thread workflows
- keep the roadmap aligned with shipped reality rather than old MVP assumptions
- test whether layered structured output improves handoff and fresh-thread reuse
