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
const MCP_TOOL_SCHEMA_VERSION: &str = "1.0";
const DEFAULT_EXCERPT_MAX_LINES: usize = 200;

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
        "get_context" | "brief_repo" => call_get_context(arguments),
        "get_changed_context" => call_get_changed_context(arguments),
        "get_file_excerpt" => call_get_file_excerpt(arguments),
        "init_memory" => call_init_memory(arguments),
        "refresh_memory" => call_refresh_memory(arguments),
        _ => {
            return error_response(id, -32602, format!("unknown tool '{name}'"));
        }
    };

    match result {
        Ok(output) => success_response(id, tool_result(name, output, false)),
        Err(message) => success_response(id, tool_result(name, ToolOutput::error(message), true)),
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "get_context",
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
                        "enum": ["markdown", "json", "viking"],
                        "description": "Output format. Defaults to markdown."
                    },
                    "changedOnly": {
                        "type": "boolean",
                        "description": "Focus on active work only."
                    },
                    "profile": {
                        "type": "string",
                        "enum": ["compact", "deep", "onboarding", "review", "incident"],
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
                    "quiet": {
                        "type": "boolean",
                        "description": "Briefing-only output (no excerpts, tree, or git details)."
                    },
                    "minify": {
                        "type": "boolean",
                        "description": "Smart minification for code excerpts (remove indent/comments)."
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
            "name": "get_changed_context",
            "description": "Generate a compact briefing focused on active repository changes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Repository root to inspect. Defaults to the MCP server working directory."
                    },
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "json", "viking"],
                        "description": "Output format. Defaults to markdown."
                    },
                    "profile": {
                        "type": "string",
                        "enum": ["compact", "deep", "onboarding", "review", "incident"],
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
                    "quiet": {
                        "type": "boolean",
                        "description": "Briefing-only output (no excerpts, tree, or git details)."
                    },
                    "minify": {
                        "type": "boolean",
                        "description": "Smart minification for code excerpts (remove indent/comments)."
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
            "name": "get_file_excerpt",
            "description": "Return a bounded line-range excerpt from a repository file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Repository root. Defaults to the MCP server working directory."
                    },
                    "path": {
                        "type": "string",
                        "description": "Relative path to the file in the repository."
                    },
                    "startLine": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "1-based inclusive start line. Defaults to 1."
                    },
                    "endLine": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "1-based inclusive end line. If omitted, maxLines is applied."
                    },
                    "maxLines": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum number of lines when endLine is omitted. Defaults to 200."
                    }
                },
                "required": ["path"],
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

#[derive(Default)]
struct ToolOutput {
    text: String,
    data: Value,
}

impl ToolOutput {
    fn success(text: String, data: Value) -> Self {
        Self { text, data }
    }

    fn error(message: String) -> Self {
        Self {
            text: message.clone(),
            data: json!({
                "error": {
                    "message": message
                }
            }),
        }
    }
}

fn call_get_context(arguments: Value) -> Result<ToolOutput, String> {
    let config = config_from_arguments(arguments, false)?;
    let rendered = render_bundle(&config);
    let data = context_result_data(&config, &rendered);
    Ok(ToolOutput::success(rendered, data))
}

fn call_get_changed_context(arguments: Value) -> Result<ToolOutput, String> {
    let config = config_from_arguments(arguments, true)?;
    let rendered = render_bundle(&config);
    let data = context_result_data(&config, &rendered);
    Ok(ToolOutput::success(rendered, data))
}

fn call_get_file_excerpt(arguments: Value) -> Result<ToolOutput, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("tool arguments must be an object".to_string());
    };
    validate_allowed_keys(
        arguments,
        &["cwd", "path", "startLine", "endLine", "maxLines"],
    )?;
    let current_dir = std::env::current_dir()
        .map_err(|source| format!("failed to resolve current directory: {source}"))?;
    let cwd = normalize_cwd(
        &current_dir,
        PathBuf::from(optional_string(arguments, "cwd")?.unwrap_or_else(|| ".".to_string())),
    );
    let relative_path = required_string(arguments, "path")?;
    let start_line = optional_usize(arguments, "startLine")?.unwrap_or(1);
    if start_line == 0 {
        return Err("'startLine' must be >= 1".to_string());
    }
    let max_lines = optional_usize(arguments, "maxLines")?.unwrap_or(DEFAULT_EXCERPT_MAX_LINES);
    if max_lines == 0 {
        return Err("'maxLines' must be >= 1".to_string());
    }
    let requested_end = optional_usize(arguments, "endLine")?;
    if let Some(end_line) = requested_end {
        if end_line == 0 {
            return Err("'endLine' must be >= 1".to_string());
        }
        if end_line < start_line {
            return Err("'endLine' must be greater than or equal to 'startLine'".to_string());
        }
    }

    let absolute_path = cwd.join(&relative_path);
    let text = std::fs::read_to_string(&absolute_path).map_err(|source| {
        format!(
            "failed to read '{}': {source}",
            absolute_path.to_string_lossy()
        )
    })?;
    let all_lines: Vec<&str> = text.lines().collect();
    let total_lines = all_lines.len();
    let desired_end =
        requested_end.unwrap_or_else(|| start_line.saturating_add(max_lines.saturating_sub(1)));
    let bounded_end = desired_end.min(total_lines.max(1));
    let slice_start = start_line.saturating_sub(1).min(total_lines);
    let slice_end = bounded_end.min(total_lines);
    let excerpt_lines = if slice_start < slice_end {
        all_lines[slice_start..slice_end].join("\n")
    } else {
        String::new()
    };
    let truncated = total_lines > slice_end;
    let data = json!({
        "cwd": cwd.to_string_lossy(),
        "path": relative_path,
        "absolutePath": absolute_path.to_string_lossy(),
        "startLine": start_line,
        "endLine": bounded_end,
        "totalLines": total_lines,
        "truncated": truncated,
        "content": excerpt_lines
    });
    Ok(ToolOutput::success(excerpt_lines, data))
}

fn call_init_memory(arguments: Value) -> Result<ToolOutput, String> {
    let config = config_from_cwd_argument(arguments)?;
    let message = init_memory_template(&config).map_err(|error| error.to_string())?;
    Ok(ToolOutput::success(
        message.clone(),
        json!({
            "cwd": config.cwd.to_string_lossy(),
            "message": message
        }),
    ))
}

fn call_refresh_memory(arguments: Value) -> Result<ToolOutput, String> {
    let config = config_from_cwd_argument(arguments)?;
    let message = refresh_memory_template(&config).map_err(|error| error.to_string())?;
    Ok(ToolOutput::success(
        message.clone(),
        json!({
            "cwd": config.cwd.to_string_lossy(),
            "message": message
        }),
    ))
}

fn config_from_arguments(
    arguments: Value,
    changed_only_default: bool,
) -> Result<AppConfig, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("tool arguments must be an object".to_string());
    };
    validate_allowed_keys(
        arguments,
        &[
            "cwd",
            "format",
            "profile",
            "languageAware",
            "noGit",
            "noTree",
            "noTests",
            "quiet",
            "minify",
            "maxBytes",
            "maxFiles",
            "maxDepth",
            "include",
            "exclude",
        ],
    )?;

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
        if !matches!(
            value,
            "compact" | "deep" | "onboarding" | "review" | "incident"
        ) {
            return Err(
                "invalid 'profile', expected compact, deep, onboarding, review, or incident"
                    .to_string(),
            );
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
        refresh_context: false,
        check_context: false,
        mcp_server: false,
        changed_only: changed_only_default,
        language_aware: optional_bool(arguments, "languageAware")?.unwrap_or(true),
        no_git: optional_bool(arguments, "noGit")?.unwrap_or(false),
        no_tree: optional_bool(arguments, "noTree")?.unwrap_or(false),
        no_tests: optional_bool(arguments, "noTests")?.unwrap_or(false),
        quiet: optional_bool(arguments, "quiet")?.unwrap_or(false),
        minify: optional_bool(arguments, "minify")?.unwrap_or(false),
        max_bytes: optional_usize(arguments, "maxBytes")?.unwrap_or(DEFAULT_MAX_BYTES),
        max_files: optional_usize(arguments, "maxFiles")?.unwrap_or(DEFAULT_MAX_FILES),
        max_depth: optional_usize(arguments, "maxDepth")?.unwrap_or(DEFAULT_MAX_DEPTH),
        include: optional_string_array(arguments, "include")?.unwrap_or_default(),
        exclude: optional_string_array(arguments, "exclude")?.unwrap_or_default(),
    })
}

fn config_from_cwd_argument(arguments: Value) -> Result<AppConfig, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("tool arguments must be an object".to_string());
    };
    validate_allowed_keys(arguments, &["cwd"])?;
    let current_dir = std::env::current_dir()
        .map_err(|source| format!("failed to resolve current directory: {source}"))?;
    let cwd = normalize_cwd(
        &current_dir,
        PathBuf::from(optional_string(arguments, "cwd")?.unwrap_or_else(|| ".".to_string())),
    );

    Ok(AppConfig {
        cwd,
        format: OutputFormat::Markdown,
        profile: None,
        diff_from: None,
        diff_to: None,
        output: None,
        init_memory: false,
        refresh_memory: false,
        refresh_context: false,
        check_context: false,
        mcp_server: false,
        changed_only: false,
        language_aware: true,
        no_git: false,
        no_tree: false,
        no_tests: false,
        quiet: false,
        minify: false,
        max_bytes: DEFAULT_MAX_BYTES,
        max_files: DEFAULT_MAX_FILES,
        max_depth: DEFAULT_MAX_DEPTH,
        include: Vec::new(),
        exclude: Vec::new(),
    })
}

fn validate_allowed_keys(arguments: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    for key in arguments.keys() {
        if !allowed.iter().any(|allowed_key| key == allowed_key) {
            return Err(format!("unknown argument '{key}'"));
        }
    }
    Ok(())
}

fn required_string(arguments: &Map<String, Value>, key: &str) -> Result<String, String> {
    optional_string(arguments, key)?.ok_or_else(|| format!("'{key}' is required"))
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

fn context_result_data(config: &AppConfig, rendered: &str) -> Value {
    let format = output_format_label(config.format);
    let payload = if matches!(config.format, OutputFormat::Json | OutputFormat::Viking) {
        serde_json::from_str::<Value>(rendered)
            .unwrap_or_else(|_| Value::String(rendered.to_string()))
    } else {
        Value::String(rendered.to_string())
    };
    json!({
        "cwd": config.cwd.to_string_lossy(),
        "format": format,
        "changedOnly": config.changed_only,
        "payload": payload
    })
}

fn output_format_label(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Json => "json",
        OutputFormat::Viking => "viking",
    }
}

fn tool_result(tool_name: &str, output: ToolOutput, is_error: bool) -> Value {
    let structured = if is_error {
        json!({
            "schemaVersion": MCP_TOOL_SCHEMA_VERSION,
            "tool": tool_name,
            "status": "error",
            "data": output.data
        })
    } else {
        json!({
            "schemaVersion": MCP_TOOL_SCHEMA_VERSION,
            "tool": tool_name,
            "status": "ok",
            "data": output.data
        })
    };
    let text = serde_json::to_string_pretty(&structured).unwrap_or_else(|_| output.text.clone());

    if is_error {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "structuredContent": structured,
            "isError": true
        })
    } else {
        json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ],
            "structuredContent": structured
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

    use super::{handle_line, ServerState, MCP_TOOL_SCHEMA_VERSION};

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

        assert!(tools.iter().any(|tool| tool["name"] == "get_context"));
        assert!(tools
            .iter()
            .any(|tool| tool["name"] == "get_changed_context"));
        assert!(tools.iter().any(|tool| tool["name"] == "get_file_excerpt"));
        assert!(tools.iter().any(|tool| tool["name"] == "init_memory"));
        assert!(tools.iter().any(|tool| tool["name"] == "refresh_memory"));
    }

    #[test]
    fn get_context_tool_returns_versioned_structured_payload() {
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
                "name": "get_context",
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

        let result = response.result.expect("get_context should succeed");
        let structured = &result["structuredContent"];
        assert_eq!(structured["schemaVersion"], MCP_TOOL_SCHEMA_VERSION);
        assert_eq!(structured["tool"], "get_context");
        assert_eq!(structured["status"], "ok");
        assert_eq!(structured["data"]["format"], "markdown");
        assert_eq!(structured["data"]["changedOnly"], false);
        assert!(structured["data"]["payload"]
            .as_str()
            .expect("payload should be a string")
            .contains("# Context Pack"));
    }

    #[test]
    fn get_changed_context_forces_changed_only() {
        let temp = TempDir::new("mcp-changed");
        write_file(
            temp.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );

        let mut state = ServerState {
            protocol_version: Some("2025-06-18".to_string()),
        };
        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_changed_context",
                "arguments": {
                    "cwd": temp.path().display().to_string(),
                    "noGit": true
                }
            }
        })
        .to_string();
        let response = handle_line(&mut state, &request).expect("tools/call should respond");
        let result = response.result.expect("get_changed_context should succeed");
        assert_eq!(result["structuredContent"]["data"]["changedOnly"], true);
    }

    #[test]
    fn get_context_accepts_viking_format() {
        let temp = TempDir::new("mcp-viking");
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
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "get_context",
                "arguments": {
                    "cwd": temp.path().display().to_string(),
                    "format": "viking",
                    "noGit": true
                }
            }
        })
        .to_string();
        let response = handle_line(&mut state, &request).expect("tools/call should respond");
        let result = response.result.expect("get_context should succeed");
        assert_eq!(result["structuredContent"]["data"]["format"], "viking");
        assert_eq!(
            result["structuredContent"]["data"]["payload"]["tiers"]["L0"]["repo"]["project_types"]
                .is_array(),
            true
        );
    }

    #[test]
    fn get_file_excerpt_returns_line_slice() {
        let temp = TempDir::new("mcp-excerpt");
        write_file(temp.path(), "src/lib.rs", "line1\nline2\nline3\nline4\n");

        let mut state = ServerState {
            protocol_version: Some("2025-06-18".to_string()),
        };
        let request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "get_file_excerpt",
                "arguments": {
                    "cwd": temp.path().display().to_string(),
                    "path": "src/lib.rs",
                    "startLine": 2,
                    "maxLines": 2
                }
            }
        })
        .to_string();
        let response = handle_line(&mut state, &request).expect("tools/call should respond");
        let result = response.result.expect("get_file_excerpt should succeed");
        let data = &result["structuredContent"]["data"];
        assert_eq!(
            result["structuredContent"]["schemaVersion"],
            MCP_TOOL_SCHEMA_VERSION
        );
        assert_eq!(data["startLine"], 2);
        assert_eq!(data["endLine"], 3);
        assert_eq!(data["content"], "line2\nline3");
        assert_eq!(data["truncated"], true);
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
