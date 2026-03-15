use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::cli::{
    normalize_cwd, CliError, DEFAULT_MAX_BYTES, DEFAULT_MAX_DEPTH, DEFAULT_MAX_FILES,
};
use crate::model::{AppConfig, OutputFormat};
use crate::{init_memory_template, refresh_memory_template, render_bundle};

const JSONRPC_VERSION: &str = "2.0";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

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
        "brief_repo" => call_brief_repo(arguments),
        "init_memory" => call_init_memory(arguments),
        "refresh_memory" => call_refresh_memory(arguments),
        _ => {
            return error_response(id, -32602, format!("unknown tool '{name}'"));
        }
    };

    match result {
        Ok(text) => success_response(id, tool_result(text, false)),
        Err(message) => success_response(id, tool_result(message, true)),
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "brief_repo",
            "description": "Generate a compact repository briefing from a target directory using Context Pack.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Repository root to inspect. Defaults to the MCP server working directory."
                    },
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "json"],
                        "description": "Output format. Defaults to markdown."
                    },
                    "changedOnly": {
                        "type": "boolean",
                        "description": "Focus on active work only."
                    },
                    "profile": {
                        "type": "string",
                        "enum": ["onboarding", "review", "incident"],
                        "description": "Preset analysis profile."
                    },
                    "languageAware": {
                        "type": "boolean",
                        "description": "Enable language-aware ranking boosts. Defaults to true."
                    },
                    "noGit": {
                        "type": "boolean",
                        "description": "Disable git collection."
                    },
                    "noTree": {
                        "type": "boolean",
                        "description": "Disable tree output."
                    },
                    "noTests": {
                        "type": "boolean",
                        "description": "Exclude common test directories."
                    },
                    "maxBytes": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Output byte budget."
                    },
                    "maxFiles": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum selected files."
                    },
                    "maxDepth": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum tree depth."
                    },
                    "include": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Extra include globs."
                    },
                    "exclude": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Extra exclude globs."
                    }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "init_memory",
            "description": "Create a .context-pack/memory.md draft for a repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Repository root. Defaults to the MCP server working directory."
                    }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "refresh_memory",
            "description": "Regenerate the .context-pack/memory.md draft for a repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Repository root. Defaults to the MCP server working directory."
                    }
                },
                "additionalProperties": false
            }
        }),
    ]
}

fn call_brief_repo(arguments: Value) -> Result<String, String> {
    let config = config_from_arguments(arguments)?;
    Ok(render_bundle(&config))
}

fn call_init_memory(arguments: Value) -> Result<String, String> {
    let config = config_from_arguments(arguments)?;
    init_memory_template(&config).map_err(|error| error.to_string())
}

fn call_refresh_memory(arguments: Value) -> Result<String, String> {
    let config = config_from_arguments(arguments)?;
    refresh_memory_template(&config).map_err(|error| error.to_string())
}

fn config_from_arguments(arguments: Value) -> Result<AppConfig, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("tool arguments must be an object".to_string());
    };

    let current_dir = std::env::current_dir()
        .map_err(|source| format!("failed to resolve current directory: {source}"))?;
    let cwd = normalize_cwd(
        &current_dir,
        PathBuf::from(optional_string(arguments, "cwd")?.unwrap_or_else(|| ".".to_string())),
    );
    let format = match optional_string(arguments, "format")? {
        Some(value) => OutputFormat::parse(&value).map_err(|error| error.to_string())?,
        None => OutputFormat::Markdown,
    };

    let profile = optional_string(arguments, "profile")?;
    if let Some(value) = profile.as_deref() {
        if !matches!(value, "onboarding" | "review" | "incident") {
            return Err("invalid 'profile', expected onboarding, review, or incident".to_string());
        }
    }

    Ok(AppConfig {
        cwd,
        format,
        profile,
        diff_from: None,
        diff_to: None,
        output: None,
        init_memory: false,
        refresh_memory: false,
        mcp_server: false,
        changed_only: optional_bool(arguments, "changedOnly")?.unwrap_or(false),
        language_aware: optional_bool(arguments, "languageAware")?.unwrap_or(true),
        no_git: optional_bool(arguments, "noGit")?.unwrap_or(false),
        no_tree: optional_bool(arguments, "noTree")?.unwrap_or(false),
        no_tests: optional_bool(arguments, "noTests")?.unwrap_or(false),
        max_bytes: optional_usize(arguments, "maxBytes")?.unwrap_or(DEFAULT_MAX_BYTES),
        max_files: optional_usize(arguments, "maxFiles")?.unwrap_or(DEFAULT_MAX_FILES),
        max_depth: optional_usize(arguments, "maxDepth")?.unwrap_or(DEFAULT_MAX_DEPTH),
        include: optional_string_array(arguments, "include")?.unwrap_or_default(),
        exclude: optional_string_array(arguments, "exclude")?.unwrap_or_default(),
    })
}

fn optional_string(arguments: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("'{key}' must be a string")),
    }
}

fn optional_bool(arguments: &Map<String, Value>, key: &str) -> Result<Option<bool>, String> {
    match arguments.get(key) {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("'{key}' must be a boolean")),
    }
}

fn optional_usize(arguments: &Map<String, Value>, key: &str) -> Result<Option<usize>, String> {
    match arguments.get(key) {
        None => Ok(None),
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
        None => Ok(None),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| format!("'{key}' must contain only strings"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(format!("'{key}' must be an array of strings")),
    }
}

fn tool_result(text: String, is_error: bool) -> Value {
    if is_error {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "isError": true
        })
    } else {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        })
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

    use serde_json::json;

    use super::{handle_line, ServerState};

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
        let mut state = ServerState {
            protocol_version: Some("2025-06-18".to_string()),
        };
        let response = handle_line(
            &mut state,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        )
        .expect("tools/list should return a response");

        let tools = response.result.expect("tools/list should succeed")["tools"]
            .as_array()
            .expect("tools should be an array")
            .clone();

        assert!(tools.iter().any(|tool| tool["name"] == "brief_repo"));
        assert!(tools.iter().any(|tool| tool["name"] == "init_memory"));
        assert!(tools.iter().any(|tool| tool["name"] == "refresh_memory"));
    }

    #[test]
    fn brief_repo_tool_returns_briefing_text() {
        let temp = TempDir::new("mcp-brief");
        write_file(
            temp.path(),
            "README.md",
            "# Demo Repo\n\nProject overview.\n",
        );
        write_file(
            temp.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write_file(temp.path(), "src/main.rs", "fn main() {}\n");

        let mut state = ServerState {
            protocol_version: Some("2025-06-18".to_string()),
        };
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "brief_repo",
                "arguments": {
                    "cwd": temp.path().display().to_string(),
                    "noGit": true,
                    "noTree": true,
                    "noTests": false
                }
            }
        })
        .to_string();
        let response = handle_line(&mut state, &request).expect("tools/call should respond");

        let result = response.result.expect("brief_repo should succeed");
        let text = result["content"][0]["text"]
            .as_str()
            .expect("tool text should be a string");
        assert!(text.contains("# Context Pack"));
        assert!(text.contains("## Agent Briefing"));
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
