# Roadmap

`context-pack` gives a coding agent a map of a repository before it starts exploring: instructions to follow, commands to run, entry points, key files, layout, and active work. The measure of success is whether an agent that reads the briefing reaches the right file and the right verification command in fewer steps than one that does not.

## Next

1. **Benchmark.** A reproducible comparison on 8–10 public repositories of different shapes (monorepo, library, application, JVM, Python, Go):
   - offline: does the briefing contain the files touched by the next real commits, the entry point a maintainer would name, and the command CI runs?
   - with an agent: tool calls, tokens, and correctness for "where would you change X" tasks, with and without the briefing.
   The results decide which sections earn their bytes.
2. **Task-aware ranking.** An optional `--task "<description>"` that boosts files whose paths and symbols match the task, so the key-file list fits the question being asked.
3. **More declared entry points and commands**: Nx/Turborepo project graphs, Bazel/Buck targets, `Rakefile`, `noxfile.py`, `tox` environments.

## Principles

- Every line of output must change what an agent does next.
- Deterministic and local: no index, no network, no API keys.
- Heuristics are backed by an end-to-end test reproducing the repository shape they fix.
