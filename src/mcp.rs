use std::io::{self, BufRead, Write};
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::cli::{default_config, normalize_cwd, CliError};
use crate::model::{AppConfig, Command, OutputFormat, Profile};
use crate::{init_memory, memory, refresh_memory, render};

const JSONRPC_VERSION: &str = "2.0";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
const DEFAULT_EXCERPT_MAX_LINES: usize = 200;
const MAX_NOTE_CHARS: usize = 500;

#[derive(Default)]
struct ServerState {
    protocol_version: Option<String>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

pub fn serve() -> Result<(), CliError> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut state = ServerState::default();

    for line in stdin.lock().lines() {
        let line =
            line.map_err(|source| CliError::Mcp(format!("failed to read MCP stdin: {source}")))?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_line(&mut state, &line) {
            serde_json::to_writer(&mut stdout, &response).map_err(|source| {
                CliError::Mcp(format!("failed to serialize MCP response: {source}"))
            })?;
            stdout
                .write_all(b"\n")
                .and_then(|_| stdout.flush())
                .map_err(|source| {
                    CliError::Mcp(format!("failed to write MCP response: {source}"))
                })?;
        }
    }

    Ok(())
}

fn handle_line(state: &mut ServerState, line: &str) -> Option<JsonRpcResponse> {
    let payload: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(error) => {
            return Some(error_response(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            ))
        }
    };

    let object = match payload.as_object() {
        Some(object) => object,
        None => {
            return Some(error_response(
                Value::Null,
                -32600,
                "invalid request: expected object",
            ))
        }
    };

    let method = match object.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => {
            return Some(error_response(
                object.get("id").cloned().unwrap_or(Value::Null),
                -32600,
                "invalid request: missing method",
            ))
        }
    };

    let id = object.get("id").cloned();
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => Some(handle_initialize(state, id, params)),
        "notifications/initialized" => {
            if state.protocol_version.is_none() {
                state.protocol_version = Some(SUPPORTED_PROTOCOL_VERSIONS[0].to_string());
            }
            None
        }
        "ping" => id.map(|id| success_response(id, json!({}))),
        "tools/list" => id.map(|id| success_response(id, json!({ "tools": tool_definitions() }))),
        "tools/call" => id.map(|id| handle_tool_call(id, params)),
        _ => id.map(|id| error_response(id, -32601, format!("method not found: {method}"))),
    }
}

fn handle_initialize(state: &mut ServerState, id: Option<Value>, params: Value) -> JsonRpcResponse {
    let id = id.unwrap_or(Value::Null);
    let Some(params) = params.as_object() else {
        return error_response(id, -32602, "initialize params must be an object");
    };

    let Some(requested_version) = params.get("protocolVersion").and_then(Value::as_str) else {
        return error_response(id, -32602, "initialize params must include protocolVersion");
    };

    if !SUPPORTED_PROTOCOL_VERSIONS.contains(&requested_version) {
        return error_response(
            id,
            -32602,
            format!(
                "unsupported protocol version '{requested_version}', supported versions: {}",
                SUPPORTED_PROTOCOL_VERSIONS.join(", ")
            ),
        );
    }

    state.protocol_version = Some(requested_version.to_string());

    success_response(
        id,
        json!({
            "protocolVersion": requested_version,
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tool_call(id: Value, params: Value) -> JsonRpcResponse {
    let Some(params) = params.as_object() else {
        return error_response(id, -32602, "tool call params must be an object");
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "tool call params must include name");
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let result = match name {
        "get_context" => call_get_context(arguments, false),
        "get_changed_context" => call_get_context(arguments, true),
        "get_file_excerpt" => call_get_file_excerpt(arguments),
        "add_memory_note" => call_add_memory_note(arguments),
        "init_memory" => call_memory(arguments, init_memory),
        "refresh_memory" => call_memory(arguments, refresh_memory),
        _ => return error_response(id, -32602, format!("unknown tool '{name}'")),
    };

    success_response(
        id,
        match result {
            Ok(output) => output.into_result(false),
            Err(message) => ToolOutput::text(message).into_result(true),
        },
    )
}

fn context_properties() -> Value {
    json!({
        "cwd": {"type": "string", "description": "Repository root. Defaults to the server working directory."},
        "format": {"type": "string", "enum": ["markdown", "json"], "description": "markdown (default) is best for reading; json for programmatic use."},
        "profile": {"type": "string", "enum": ["compact", "deep", "review"], "description": "compact ≈2 KB, deep ≈16 KB, review focuses on changes."},
        "maxBytes": {"type": "integer", "minimum": 500, "description": "Size budget for the markdown briefing (default 6000)."},
        "maxFiles": {"type": "integer", "minimum": 1, "description": "Maximum key files (default 8)."},
        "include": {"type": "array", "items": {"type": "string"}, "description": "Globs to always surface as key files."},
        "exclude": {"type": "array", "items": {"type": "string"}, "description": "Globs to ignore entirely."},
        "noGit": {"type": "boolean"},
        "noLayout": {"type": "boolean"},
        "excerpts": {"type": "boolean", "description": "Fill leftover budget with file excerpts (default true)."}
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "get_context",
            "description": "First-pass briefing for a repository: agent instruction files to follow, build/test/lint commands, entry points, key source files, workspace packages, directory layout, active git work, and durable repo memory. Call this before exploring an unfamiliar repo.",
            "inputSchema": {"type": "object", "properties": context_properties(), "additionalProperties": false}
        }),
        json!({
            "name": "get_changed_context",
            "description": "Briefing focused on active work: uncommitted changes, commits on the current branch versus its base, and the files they touch. Use for code review or resuming work.",
            "inputSchema": {"type": "object", "properties": context_properties(), "additionalProperties": false}
        }),
        json!({
            "name": "get_file_excerpt",
            "description": "Return a line range from a file inside the repository (paths outside the repository are rejected).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {"type": "string", "description": "Repository root. Defaults to the server working directory."},
                    "path": {"type": "string", "description": "File path relative to the repository root."},
                    "startLine": {"type": "integer", "minimum": 1},
                    "endLine": {"type": "integer", "minimum": 1},
                    "maxLines": {"type": "integer", "minimum": 1, "description": "Used when endLine is omitted (default 200)."}
                },
                "required": ["path"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "add_memory_note",
            "description": "Append one durable fact to .context-pack/memory.md (a rule, pitfall, invariant, or non-obvious command). It will appear in every future briefing. Do not store secrets or transient task state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {"type": "string"},
                    "note": {"type": "string", "description": "One concise sentence."}
                },
                "required": ["note"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "init_memory",
            "description": "Create .context-pack/memory.md if it does not exist.",
            "inputSchema": {"type": "object", "properties": {"cwd": {"type": "string"}}, "additionalProperties": false}
        }),
        json!({
            "name": "refresh_memory",
            "description": "Mark the memory notes as reviewed (updates the timestamp only; notes are never rewritten).",
            "inputSchema": {"type": "object", "properties": {"cwd": {"type": "string"}}, "additionalProperties": false}
        }),
    ]
}

struct ToolOutput {
    text: String,
    structured: Option<Value>,
}

impl ToolOutput {
    fn text(text: String) -> Self {
        Self {
            text,
            structured: None,
        }
    }

    fn into_result(self, is_error: bool) -> Value {
        let mut result = json!({
            "content": [{"type": "text", "text": self.text}],
            "isError": is_error
        });
        if let Some(structured) = self.structured {
            result["structuredContent"] = structured;
        }
        result
    }
}

fn call_get_context(arguments: Value, changed_only: bool) -> Result<ToolOutput, String> {
    let config = context_config(arguments, changed_only)?;
    let rendered = render(&config);
    let structured = match config.format {
        OutputFormat::Json => serde_json::from_str(&rendered).ok(),
        OutputFormat::Markdown => None,
    };
    Ok(ToolOutput {
        text: rendered,
        structured,
    })
}

fn call_get_file_excerpt(arguments: Value) -> Result<ToolOutput, String> {
    let arguments = object(&arguments)?;
    validate_allowed_keys(
        arguments,
        &["cwd", "path", "startLine", "endLine", "maxLines"],
    )?;
    let cwd = cwd_argument(arguments)?;
    let relative_path = required_string(arguments, "path")?;
    let path = resolve_inside(&cwd, &relative_path)?;

    let start_line = optional_usize(arguments, "startLine")?.unwrap_or(1).max(1);
    let max_lines = optional_usize(arguments, "maxLines")?
        .unwrap_or(DEFAULT_EXCERPT_MAX_LINES)
        .max(1);
    let end_line = match optional_usize(arguments, "endLine")? {
        Some(end) if end < start_line => return Err("'endLine' must be >= 'startLine'".to_string()),
        Some(end) => end,
        None => start_line + max_lines - 1,
    };

    let text = crate::index::read_text(&path).ok_or_else(|| {
        format!("cannot read '{relative_path}' (missing, binary, or larger than 1 MB)")
    })?;
    let lines = text.lines().collect::<Vec<_>>();
    let total = lines.len();
    let from = (start_line - 1).min(total);
    let to = end_line.min(total);
    let mut output = lines[from..to]
        .iter()
        .enumerate()
        .map(|(offset, line)| format!("{:>5}: {line}", from + offset + 1))
        .collect::<Vec<_>>()
        .join("\n");
    if to < total {
        output.push_str(&format!("\n… {} more line(s); {total} total", total - to));
    }
    Ok(ToolOutput::text(output))
}

/// Resolve a user-supplied path and refuse anything that escapes the repo root.
fn resolve_inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(relative);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "'{relative}' must be a relative path inside the repository"
        ));
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("invalid cwd: {error}"))?;
    let resolved = root
        .join(candidate)
        .canonicalize()
        .map_err(|error| format!("cannot open '{relative}': {error}"))?;
    if !resolved.starts_with(&root) {
        return Err(format!("'{relative}' resolves outside the repository"));
    }
    Ok(resolved)
}

fn call_add_memory_note(arguments: Value) -> Result<ToolOutput, String> {
    let arguments = object(&arguments)?;
    validate_allowed_keys(arguments, &["cwd", "note"])?;
    let cwd = cwd_argument(arguments)?;
    let note = required_string(arguments, "note")?;
    let note = note.split_whitespace().collect::<Vec<_>>().join(" ");
    if note.is_empty() {
        return Err("'note' must not be empty".to_string());
    }
    if note.chars().count() > MAX_NOTE_CHARS {
        return Err(format!(
            "'note' must be at most {MAX_NOTE_CHARS} characters; store one fact per note"
        ));
    }

    let path = cwd.join(memory::MEMORY_PATH);
    let mut config = default_config(cwd.clone());
    config.command = Command::InitMemory;
    if !path.exists() {
        init_memory(&config).map_err(|error| error.to_string())?;
    }
    let content =
        std::fs::read_to_string(&path).map_err(|error| format!("cannot read memory: {error}"))?;
    std::fs::write(&path, memory::add_note(&content, &note))
        .map_err(|error| format!("cannot write memory: {error}"))?;
    Ok(ToolOutput::text(format!(
        "Added note to {}",
        path.display()
    )))
}

fn call_memory(
    arguments: Value,
    action: fn(&AppConfig) -> Result<String, CliError>,
) -> Result<ToolOutput, String> {
    let arguments = object(&arguments)?;
    validate_allowed_keys(arguments, &["cwd"])?;
    let config = default_config(cwd_argument(arguments)?);
    action(&config)
        .map(ToolOutput::text)
        .map_err(|error| error.to_string())
}

fn context_config(arguments: Value, changed_only: bool) -> Result<AppConfig, String> {
    let arguments = object(&arguments)?;
    validate_allowed_keys(
        arguments,
        &[
            "cwd", "format", "profile", "maxBytes", "maxFiles", "include", "exclude", "noGit",
            "noLayout", "excerpts",
        ],
    )?;

    // Build the equivalent CLI invocation so profiles and overrides behave identically.
    let mut args = vec![
        "--cwd".to_string(),
        cwd_argument(arguments)?.display().to_string(),
    ];
    if changed_only {
        args.push("--changed-only".to_string());
    }
    if let Some(format) = optional_string(arguments, "format")? {
        OutputFormat::parse(&format).map_err(|error| error.to_string())?;
        args.extend(["--format".to_string(), format]);
    }
    if let Some(profile) = optional_string(arguments, "profile")? {
        Profile::parse(&profile).map_err(|error| error.to_string())?;
        args.extend(["--profile".to_string(), profile]);
    }
    for (key, flag) in [("maxBytes", "--max-bytes"), ("maxFiles", "--max-files")] {
        if let Some(value) = optional_usize(arguments, key)? {
            args.extend([flag.to_string(), value.to_string()]);
        }
    }
    for (key, flag) in [("include", "--include"), ("exclude", "--exclude")] {
        for value in optional_string_array(arguments, key)?.unwrap_or_default() {
            args.extend([flag.to_string(), value]);
        }
    }
    if optional_bool(arguments, "noGit")? == Some(true) {
        args.push("--no-git".to_string());
    }
    if optional_bool(arguments, "noLayout")? == Some(true) {
        args.push("--no-layout".to_string());
    }
    match optional_bool(arguments, "excerpts")? {
        Some(true) => args.push("--excerpts".to_string()),
        Some(false) => args.push("--no-excerpts".to_string()),
        None => {}
    }
    crate::cli::parse_args(args).map_err(|error| error.to_string())
}

fn object(arguments: &Value) -> Result<&Map<String, Value>, String> {
    arguments
        .as_object()
        .ok_or_else(|| "tool arguments must be an object".to_string())
}

fn cwd_argument(arguments: &Map<String, Value>) -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|source| format!("failed to resolve current directory: {source}"))?;
    let cwd = optional_string(arguments, "cwd")?.unwrap_or_else(|| ".".to_string());
    let cwd = normalize_cwd(&current_dir, PathBuf::from(cwd));
    if !cwd.is_dir() {
        return Err(format!("cwd '{}' is not a directory", cwd.display()));
    }
    Ok(cwd)
}

fn validate_allowed_keys(arguments: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    match arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        Some(key) => Err(format!(
            "unknown argument '{key}' (allowed: {})",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}

fn required_string(arguments: &Map<String, Value>, key: &str) -> Result<String, String> {
    optional_string(arguments, key)?.ok_or_else(|| format!("'{key}' is required"))
}

fn optional_string(arguments: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("'{key}' must be a string")),
    }
}

fn optional_bool(arguments: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("'{key}' must be a boolean")),
    }
}

fn optional_usize(arguments: &Map<String, Value>, key: &str) -> Result<Option<usize>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_u64()
            .map(|value| Some(value as usize))
            .ok_or_else(|| format!("'{key}' must be a non-negative integer")),
        Some(_) => Err(format!("'{key}' must be an integer")),
    }
}

fn optional_string_array(
    arguments: &Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<String>>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("'{key}' must contain only strings"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(format!("'{key}' must be an array of strings")),
    }
}

fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION,
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION,
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::{json, Value};

    use super::{handle_line, ServerState};

    fn call(tool: &str, arguments: Value) -> Value {
        let mut state = ServerState {
            protocol_version: Some("2025-06-18".to_string()),
        };
        let request = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}
        })
        .to_string();
        handle_line(&mut state, &request)
            .expect("tools/call should respond")
            .result
            .expect("tools/call should return a result")
    }

    fn text(result: &Value) -> &str {
        result["content"][0]["text"].as_str().expect("text content")
    }

    #[test]
    fn initialize_negotiates_supported_protocol_version() {
        let mut state = ServerState::default();
        let response = handle_line(
            &mut state,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}"#,
        )
        .expect("initialize should return a response");
        assert_eq!(
            response.result.expect("initialize should succeed")["protocolVersion"],
            "2025-06-18"
        );
    }

    #[test]
    fn tools_list_exposes_context_pack_tools() {
        let mut state = ServerState::default();
        let response = handle_line(
            &mut state,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        )
        .expect("tools/list should return a response");
        let tools = response.result.expect("tools/list should succeed")["tools"].clone();
        for name in [
            "get_context",
            "get_changed_context",
            "get_file_excerpt",
            "add_memory_note",
            "init_memory",
            "refresh_memory",
        ] {
            assert!(
                tools
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["name"] == name),
                "{name}"
            );
        }
    }

    #[test]
    fn get_context_returns_plain_markdown() {
        let temp = TempDir::new("mcp-brief");
        write_file(
            temp.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\ndescription = \"Demo tool\"\n",
        );
        write_file(temp.path(), "src/main.rs", "fn main() {}\n");
        let result = call(
            "get_context",
            json!({"cwd": temp.path().display().to_string(), "noGit": true}),
        );
        assert_eq!(result["isError"], false);
        assert!(text(&result).starts_with("# demo — context pack"));
        assert!(result.get("structuredContent").is_none());
    }

    #[test]
    fn get_context_json_includes_structured_content() {
        let temp = TempDir::new("mcp-json");
        write_file(temp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let result = call(
            "get_changed_context",
            json!({"cwd": temp.path().display().to_string(), "noGit": true, "format": "json"}),
        );
        assert_eq!(result["structuredContent"]["repo"]["name"], "demo");
    }

    #[test]
    fn get_file_excerpt_returns_numbered_lines() {
        let temp = TempDir::new("mcp-excerpt");
        write_file(temp.path(), "src/lib.rs", "line1\nline2\nline3\nline4\n");
        let result = call(
            "get_file_excerpt",
            json!({"cwd": temp.path().display().to_string(), "path": "src/lib.rs", "startLine": 2, "maxLines": 2}),
        );
        assert_eq!(
            text(&result),
            "    2: line2\n    3: line3\n… 1 more line(s); 4 total"
        );
    }

    #[test]
    fn get_file_excerpt_rejects_paths_outside_repo() {
        let temp = TempDir::new("mcp-escape");
        write_file(temp.path(), "a.txt", "x\n");
        for path in ["../secret", "/etc/passwd"] {
            let result = call(
                "get_file_excerpt",
                json!({"cwd": temp.path().display().to_string(), "path": path}),
            );
            assert_eq!(result["isError"], true, "{path}");
        }
    }

    #[test]
    fn add_memory_note_creates_and_appends() {
        let temp = TempDir::new("mcp-note");
        let cwd = temp.path().display().to_string();
        assert_eq!(
            call(
                "add_memory_note",
                json!({"cwd": cwd, "note": "Run `make db` before tests."})
            )["isError"],
            false
        );
        call(
            "add_memory_note",
            json!({"cwd": cwd, "note": "Never edit generated/ by hand."}),
        );
        let memory = fs::read_to_string(temp.path().join(".context-pack/memory.md")).unwrap();
        assert!(
            memory.contains(
                "## Notes\n- Run `make db` before tests.\n- Never edit generated/ by hand.\n"
            ),
            "{memory}"
        );
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("context-pack-{prefix}-{nonce}"));
            fs::create_dir_all(&path).expect("failed to create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directory");
        }
        fs::write(path, content).expect("failed to write file");
    }
}
