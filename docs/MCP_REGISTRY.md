# Publishing to the Official MCP Registry

To get `context-pack` discovered globally by AI agents like Cursor and Claude Desktop, you should submit it to the official [Model Context Protocol Servers Repository](https://github.com/modelcontextprotocol/servers).

## Steps to submit:
1. Fork the `modelcontextprotocol/servers` repository.
2. In your fork, navigate to the `src` folder. Our tool is a CLI utility that reads local file systems, so adding it alongside standard developer tools is preferred.
3. You need to write a short Node/Python wrapper or simply document how to run the `stdio` server in the global list, but because `context-pack` is a compiled Rust binary, you simply add a JSON/Typescript entry to their aggregate list pointing users to:
```json
{
  "name": "context-pack",
  "command": "context-pack",
  "args": ["--mcp-server"]
}
```
4. Open a Pull Request titled `Add context-pack server`.

Because `mcp.json`, `smithery.yaml`, and `tool.json` are now in your repo root, automated indexers like `smithery.ai` will also pick it up automatically!
