use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{AppConfig, ImportantFile, SignalCategory};

pub fn collect(config: &AppConfig, files: &[ImportantFile], budget: usize) -> Vec<String> {
    let mut manifests = files
        .iter()
        .filter(|file| matches!(file.category, SignalCategory::Manifest))
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    manifests.sort();
    manifests.dedup();

    let mut summaries = Vec::new();
    for relative_path in manifests {
        let absolute_path = config.cwd.join(&relative_path);
        let Some(summary) = summarize_manifest(&relative_path, &absolute_path) else {
            continue;
        };
        summaries.push(summary);
        trim_to_budget(&mut summaries, budget);
    }

    summaries
}

fn summarize_manifest(relative_path: &Path, absolute_path: &Path) -> Option<String> {
    let file_name = relative_path.file_name()?.to_str()?;
    let text = fs::read_to_string(absolute_path).ok()?;

    match file_name {
        "package.json" => summarize_package_json(relative_path, &text),
        "Cargo.toml" => summarize_cargo_toml(relative_path, &text),
        "requirements.txt" => summarize_requirements(relative_path, &text),
        "go.mod" => summarize_go_mod(relative_path, &text),
        "pom.xml" => summarize_pom(relative_path, &text),
        "build.gradle" | "build.gradle.kts" => summarize_gradle(relative_path, &text),
        _ => None,
    }
}

fn summarize_package_json(relative_path: &Path, text: &str) -> Option<String> {
    let runtime = extract_json_section_keys(text, "dependencies");
    let dev = extract_json_section_keys(text, "devDependencies");

    let mut parts = Vec::new();
    if !runtime.is_empty() {
        parts.push(format!(
            "runtime dependencies {}",
            render_code_list_limited(&runtime, 4)
        ));
    }
    if !dev.is_empty() {
        parts.push(format!("dev tools {}", render_code_list_limited(&dev, 4)));
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("`{}`: {}.", relative_path.display(), parts.join("; ")))
}

fn summarize_cargo_toml(relative_path: &Path, text: &str) -> Option<String> {
    let runtime = extract_toml_dependencies(text, &["dependencies", "workspace.dependencies"]);
    let dev = extract_toml_dependencies(text, &["dev-dependencies"]);

    let mut parts = Vec::new();
    if !runtime.is_empty() {
        parts.push(format!(
            "dependencies {}",
            render_code_list_limited(&runtime, 4)
        ));
    }
    if !dev.is_empty() {
        parts.push(format!(
            "dev dependencies {}",
            render_code_list_limited(&dev, 4)
        ));
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("`{}`: {}.", relative_path.display(), parts.join("; ")))
}

fn summarize_requirements(relative_path: &Path, text: &str) -> Option<String> {
    let mut deps = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let name = trimmed
            .split(['=', '<', '>', '!', '['])
            .next()
            .unwrap_or_default()
            .trim();
        if !name.is_empty() {
            push_unique_string(&mut deps, name.to_string());
        }
        if deps.len() >= 4 {
            break;
        }
    }

    if deps.is_empty() {
        return None;
    }

    Some(format!(
        "`{}`: dependencies {}.",
        relative_path.display(),
        render_code_list_limited(&deps, 4)
    ))
}

fn summarize_go_mod(relative_path: &Path, text: &str) -> Option<String> {
    let mut deps = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("require (") {
            in_block = true;
            continue;
        }
        if in_block && trimmed == ")" {
            in_block = false;
            continue;
        }

        let candidate = if in_block {
            trimmed
        } else if let Some(rest) = trimmed.strip_prefix("require ") {
            rest
        } else {
            continue;
        };

        let module = candidate.split_whitespace().next().unwrap_or_default().trim();
        if !module.is_empty() {
            push_unique_string(&mut deps, module.to_string());
        }
        if deps.len() >= 4 {
            break;
        }
    }

    if deps.is_empty() {
        return None;
    }

    Some(format!(
        "`{}`: dependencies {}.",
        relative_path.display(),
        render_code_list_limited(&deps, 4)
    ))
}

fn summarize_pom(relative_path: &Path, text: &str) -> Option<String> {
    let mut dependencies = Vec::new();
    let mut offset = 0usize;

    while let Some(start_rel) = text[offset..].find("<dependency") {
        let start = offset + start_rel;
        let Some(end_rel) = text[start..].find("</dependency>") else {
            break;
        };
        let end = start + end_rel + "</dependency>".len();
        let block = &text[start..end];
        let artifact = extract_xml_value(block, "artifactId");
        if let Some(artifact) = artifact {
            let group = extract_xml_value(block, "groupId");
            let value = if let Some(group) = group {
                format!("{group}:{artifact}")
            } else {
                artifact
            };
            push_unique_string(&mut dependencies, value);
        }
        offset = end;
        if dependencies.len() >= 4 {
            break;
        }
    }

    if dependencies.is_empty() {
        return None;
    }

    Some(format!(
        "`{}`: dependencies {}.",
        relative_path.display(),
        render_code_list_limited(&dependencies, 4)
    ))
}

fn summarize_gradle(relative_path: &Path, text: &str) -> Option<String> {
    let scopes = [
        "implementation",
        "api",
        "compileOnly",
        "runtimeOnly",
        "testImplementation",
        "testRuntimeOnly",
        "kapt",
    ];

    let mut dependencies = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') {
            continue;
        }
        if !scopes.iter().any(|scope| trimmed.starts_with(scope)) {
            continue;
        }
        if let Some(value) = extract_quoted_value(trimmed) {
            push_unique_string(&mut dependencies, value);
        }
        if dependencies.len() >= 4 {
            break;
        }
    }

    if dependencies.is_empty() {
        return None;
    }

    Some(format!(
        "`{}`: dependencies {}.",
        relative_path.display(),
        render_code_list_limited(&dependencies, 4)
    ))
}

fn extract_json_section_keys(text: &str, section_name: &str) -> Vec<String> {
    let pattern = format!("\"{section_name}\"");
    let Some(section_start) = text.find(&pattern) else {
        return Vec::new();
    };
    let after_section = &text[section_start + pattern.len()..];
    let Some(open_rel) = after_section.find('{') else {
        return Vec::new();
    };
    let open_index = section_start + pattern.len() + open_rel;
    let Some(close_index) = find_matching_brace(text, open_index) else {
        return Vec::new();
    };
    extract_json_object_keys(&text[open_index + 1..close_index])
}

fn extract_json_object_keys(section: &str) -> Vec<String> {
    let bytes = section.as_bytes();
    let mut keys = Vec::new();
    let mut index = 0usize;
    let mut depth = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' if depth == 0 => {
                if let Some((value, next_index)) = parse_json_string(section, index) {
                    index = skip_whitespace_bytes(bytes, next_index);
                    if index < bytes.len() && bytes[index] == b':' {
                        push_unique_string(&mut keys, value);
                    }
                    index += 1;
                } else {
                    break;
                }
            }
            _ => index += 1,
        }
    }

    keys
}

fn parse_json_string(section: &str, start: usize) -> Option<(String, usize)> {
    let bytes = section.as_bytes();
    if bytes.get(start) != Some(&b'"') {
        return None;
    }

    let mut value = String::new();
    let mut index = start + 1;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            value.push(byte as char);
            escaped = false;
            index += 1;
            continue;
        }
        match byte {
            b'\\' => {
                escaped = true;
                index += 1;
            }
            b'"' => return Some((value, index + 1)),
            _ => {
                value.push(byte as char);
                index += 1;
            }
        }
    }

    None
}

fn skip_whitespace_bytes(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn find_matching_brace(text: &str, open_index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open_index) != Some(&b'{') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, byte) in bytes.iter().enumerate().skip(open_index) {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_toml_dependencies(text: &str, sections: &[&str]) -> Vec<String> {
    let mut dependencies = Vec::new();
    let mut active = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            active = sections.iter().any(|candidate| candidate == &section);
            continue;
        }

        if !active || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let dependency = name.trim();
            if !dependency.is_empty() {
                push_unique_string(&mut dependencies, dependency.to_string());
            }
        }
        if dependencies.len() >= 4 {
            break;
        }
    }

    dependencies
}

fn extract_xml_value(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = block.find(&open)? + open.len();
    let end_rel = block[start..].find(&close)?;
    let value = block[start..start + end_rel].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn extract_quoted_value(line: &str) -> Option<String> {
    for quote in ['"', '\''] {
        if let Some(start) = line.find(quote) {
            let rest = &line[start + 1..];
            if let Some(end) = rest.find(quote) {
                let value = rest[..end].trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

fn render_code_list_limited(values: &[String], limit: usize) -> String {
    values
        .iter()
        .take(limit)
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn trim_to_budget(items: &mut Vec<String>, budget: usize) {
    while items.iter().map(|item| item.len()).sum::<usize>() > budget {
        if items.pop().is_none() {
            break;
        }
    }
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if value.is_empty() || values.iter().any(|existing| existing == &value) {
        return;
    }

    values.push(value);
}
