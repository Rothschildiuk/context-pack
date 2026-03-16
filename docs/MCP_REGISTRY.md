# Publishing to the MCP Registry

## npm package

The `npm/` directory contains a thin npm wrapper that downloads the pre-built
Rust binary on `postinstall`. The CI publishes it automatically on each tagged
release.

Users install with:

```bash
npx context-pack --mcp-server
```

Or globally:

```bash
npm install -g context-pack
context-pack --mcp-server
```

## MCP Registry

The `npm/server.json` file follows the official MCP Registry schema. To publish:

```bash
brew install mcp-publisher   # or build from source
mcp-publisher login github
cd npm && mcp-publisher publish
```

## Version sync

Run `scripts/sync-npm-version.sh` after bumping `Cargo.toml` to keep
`npm/package.json` and `npm/server.json` in sync. The release CI also
auto-syncs the version from the git tag.

## Automated discovery

- `smithery.yaml` and `tool.json` in the repo root are picked up by smithery.ai
- `.mcp.json` enables auto-discovery when the repo is cloned and opened in
  Claude Code
