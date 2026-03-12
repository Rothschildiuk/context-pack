# Context Pack

## Agent Briefing
### What This Repo Is
- Likely a low-level language or formal methods project with C and Coq code.
- Primary languages: c, coq.
- Guidance files available: README.

### Active Work
- Git collection disabled

### Read These First
- `README.md`: project overview
- `C/Makefile`: build or orchestration entrypoint

### Likely Entry Points
- `C/Makefile`: build or orchestration entrypoint

### Caveats
- No AGENTS.md found.
- Git collection disabled.

## Repo
- path: <FIXTURE_ROOT>
- project types: c, coq
- primary languages: c, coq

## Git Changes
Git collection disabled.

## Important Files
### README.md
- reason: project overview
- category: overview
- score: 960
- truncated: false

```text
# C and Coq Fixture

This fixture mixes low-level code and proof-oriented files.
```

### C/Makefile
- reason: build or orchestration entrypoint
- category: build
- score: 795
- truncated: false

```text
all:
	cc main.c -o demo
```

## Tree
c_coq/
  C/
    Makefile
    main.c
  Coq/
    demo.v
  README.md

## Notes
- max bytes: 4000
- max files: 12
- max depth: 4
- budget split: briefing=900, git=500, excerpts=1800, tree=800
- selected files: 2
- files scanned for selection: 4
