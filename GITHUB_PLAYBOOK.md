# GitHub Playbook

This repository can get more leverage from GitHub without adding much process overhead.

## Releases

Recommended release shape:

- tag a semantic version
- publish binaries through the existing GitHub Actions workflow
- keep a short manual changelog entry in `CHANGELOG.md`
- use GitHub release notes for a concise "why it matters" summary

Suggested release note structure:

1. What changed
2. Why it matters
3. How to try it

## Discussions

Suggested categories to enable in the GitHub UI:

- General
- Ideas
- Show and tell
- Q&A

Suggested starter discussion topics:

- How are you using context-pack with coding agents?
- What signals matter most in older or messy repositories?
- Where does first-pass briefing still fail?

## Issues

Use issues for actionable engineering work:

- bugs in selection, ranking, or rendering
- feature requests tied to a concrete workflow problem
- measurable quality improvements

Good issue examples:

- Improve entrypoint detection in Python service repos
- Measure token savings on large unfamiliar repositories
- Add memory bootstrapping for learned repo notes

## Labels

Recommended lightweight label set:

- `bug`
- `enhancement`
- `documentation`
- `briefing`
- `heuristics`
- `memory`
- `workflow`
- `breaking-change`

## Current Recommendation

If only a few GitHub features get active attention, prioritize these:

1. Releases with clear notes
2. Discussions for workflow feedback
3. Issues tied to measurable product improvements
