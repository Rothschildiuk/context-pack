use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{AppConfig, ImportantFile, SignalCategory};

pub fn collect(config: &AppConfig, files: &[ImportantFile], budget: usize) -> Vec<String> {
    let mut candidates = root_docker_candidates(config);

    for file in files {
        if matches!(file.category, SignalCategory::Build)
            && file
                .file_name()
                .map(is_docker_relevant_file)
                .unwrap_or(false)
        {
            push_unique_path(&mut candidates, file.path.clone());
        }
    }

    candidates.sort();

    let mut summaries = Vec::new();
    for relative_path in candidates {
        let Some(file_name) = relative_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let absolute_path = config.cwd.join(&relative_path);

        let summary = if is_compose_file(file_name) {
            summarize_compose_file(&relative_path, &absolute_path)
        } else if is_dockerfile(file_name) {
            summarize_dockerfile(&relative_path, &absolute_path)
        } else {
            None
        };

        let Some(summary) = summary else {
            continue;
        };
        summaries.push(summary);
        trim_to_budget(&mut summaries, budget);
    }

    summaries
}

fn root_docker_candidates(config: &AppConfig) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(&config.cwd) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_docker_relevant_file(file_name) {
            continue;
        }

        push_unique_path(&mut candidates, PathBuf::from(file_name));
    }

    candidates
}

fn summarize_compose_file(relative_path: &Path, absolute_path: &Path) -> Option<String> {
    let text = fs::read_to_string(absolute_path).ok()?;
    let services = parse_compose_services(&text);
    if services.is_empty() {
        return None;
    }

    let service_names = services
        .iter()
        .map(|service| format!("`{}`", service.name))
        .take(4)
        .collect::<Vec<_>>();
    let images = collect_unique(
        services.iter().filter_map(|service| service.image.clone()),
        3,
    );
    let build_contexts = collect_unique(
        services.iter().filter_map(|service| service.build.clone()),
        3,
    );
    let ports = collect_unique(
        services
            .iter()
            .flat_map(|service| service.ports.iter().cloned()),
        4,
    );
    let env_files = collect_unique(
        services
            .iter()
            .flat_map(|service| service.env_files.iter().cloned()),
        2,
    );

    let mut parts = Vec::new();
    let more_services = services.len().saturating_sub(service_names.len());
    if more_services > 0 {
        parts.push(format!(
            "services {} (+{} more)",
            service_names.join(", "),
            more_services
        ));
    } else {
        parts.push(format!("services {}", service_names.join(", ")));
    }
    if !images.is_empty() {
        parts.push(format!("images {}", render_code_list(&images)));
    }
    if !build_contexts.is_empty() {
        parts.push(format!(
            "build contexts {}",
            render_code_list(&build_contexts)
        ));
    }
    if !ports.is_empty() {
        parts.push(format!("ports {}", render_code_list(&ports)));
    }
    if !env_files.is_empty() {
        parts.push(format!("env files {}", render_code_list(&env_files)));
    }

    Some(format!(
        "`{}`: {}.",
        relative_path.display(),
        parts.join("; ")
    ))
}

fn summarize_dockerfile(relative_path: &Path, absolute_path: &Path) -> Option<String> {
    let text = fs::read_to_string(absolute_path).ok()?;
    let mut base_images = Vec::new();
    let mut stage_names = Vec::new();
    let mut exposed_ports = Vec::new();
    let mut has_entrypoint = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("FROM ") {
            let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
            if let Some(image) = tokens.get(1) {
                push_unique_string(&mut base_images, (*image).to_string());
            }
            if tokens.len() >= 4 && tokens[2].eq_ignore_ascii_case("AS") {
                push_unique_string(&mut stage_names, tokens[3].to_string());
            }
        } else if upper.starts_with("EXPOSE ") {
            for token in trimmed["EXPOSE ".len()..].split_whitespace() {
                push_unique_string(&mut exposed_ports, token.to_string());
            }
        } else if upper.starts_with("ENTRYPOINT ") || upper.starts_with("CMD ") {
            has_entrypoint = true;
        }
    }

    if base_images.is_empty()
        && stage_names.is_empty()
        && exposed_ports.is_empty()
        && !has_entrypoint
    {
        return None;
    }

    let mut parts = Vec::new();
    if !base_images.is_empty() {
        parts.push(format!(
            "base images {}",
            render_code_list_limited(&base_images, 3)
        ));
    }
    if !stage_names.is_empty() {
        parts.push(format!(
            "stages {}",
            render_code_list_limited(&stage_names, 3)
        ));
    }
    if !exposed_ports.is_empty() {
        parts.push(format!(
            "exposes {}",
            render_code_list_limited(&exposed_ports, 4)
        ));
    }
    if has_entrypoint {
        parts.push("defines container startup commands".to_string());
    }

    Some(format!(
        "`{}`: {}.",
        relative_path.display(),
        parts.join("; ")
    ))
}

#[derive(Default)]
struct ComposeService {
    name: String,
    image: Option<String>,
    build: Option<String>,
    ports: Vec<String>,
    env_files: Vec<String>,
    depends_on: Vec<String>,
}

fn parse_compose_services(text: &str) -> Vec<ComposeService> {
    let mut services = Vec::new();
    let mut in_services = false;
    let mut current: Option<ComposeService> = None;
    let mut active_list: Option<&str> = None;
    let mut in_build_block = false;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();

        if indent == 0 {
            if let Some(service) = current.take() {
                services.push(service);
            }
            in_services = trimmed == "services:";
            active_list = None;
            in_build_block = false;
            continue;
        }

        if !in_services {
            continue;
        }

        if indent == 2 && trimmed.ends_with(':') {
            if let Some(service) = current.take() {
                services.push(service);
            }
            current = Some(ComposeService {
                name: trimmed.trim_end_matches(':').to_string(),
                ..ComposeService::default()
            });
            active_list = None;
            in_build_block = false;
            continue;
        }

        let Some(service) = current.as_mut() else {
            continue;
        };

        if indent == 4 {
            active_list = None;
            in_build_block = false;

            if let Some(value) = trimmed.strip_prefix("image:") {
                service.image = clean_scalar(value);
            } else if let Some(value) = trimmed.strip_prefix("build:") {
                let value = value.trim();
                if value.is_empty() {
                    in_build_block = true;
                } else {
                    service.build = clean_scalar(value);
                }
            } else if let Some(value) = trimmed.strip_prefix("ports:") {
                active_list = Some("ports");
                extend_list(&mut service.ports, parse_inline_list(value));
            } else if let Some(value) = trimmed.strip_prefix("env_file:") {
                active_list = Some("env_file");
                extend_list(&mut service.env_files, parse_inline_list(value));
            } else if let Some(value) = trimmed.strip_prefix("depends_on:") {
                active_list = Some("depends_on");
                extend_list(&mut service.depends_on, parse_inline_list(value));
            }
            continue;
        }

        if in_build_block && indent >= 6 {
            if let Some(value) = trimmed.strip_prefix("context:") {
                service.build = clean_scalar(value);
            }
            continue;
        }

        if indent >= 6 {
            if let Some(value) = trimmed.strip_prefix("- ") {
                let item = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                match active_list {
                    Some("ports") => push_unique_string(&mut service.ports, item),
                    Some("env_file") => push_unique_string(&mut service.env_files, item),
                    Some("depends_on") => push_unique_string(&mut service.depends_on, item),
                    _ => {}
                }
            }
        }
    }

    if let Some(service) = current {
        services.push(service);
    }

    services
}

fn clean_scalar(value: &str) -> Option<String> {
    let cleaned = value.trim().trim_matches('"').trim_matches('\'');
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn parse_inline_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return clean_scalar(trimmed).into_iter().collect();
    }

    trimmed[1..trimmed.len() - 1]
        .split(',')
        .filter_map(clean_scalar)
        .collect()
}

fn extend_list(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        push_unique_string(target, value);
    }
}

fn collect_unique<I>(values: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut unique = Vec::new();
    for value in values {
        push_unique_string(&mut unique, value);
        if unique.len() >= limit {
            break;
        }
    }
    unique
}

fn render_code_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_code_list_limited(values: &[String], limit: usize) -> String {
    render_code_list(&values.iter().take(limit).cloned().collect::<Vec<_>>())
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

fn push_unique_path(values: &mut Vec<PathBuf>, value: PathBuf) {
    if values.iter().any(|existing| existing == &value) {
        return;
    }

    values.push(value);
}

fn is_docker_relevant_file(file_name: &str) -> bool {
    is_compose_file(file_name) || is_dockerfile(file_name)
}

fn is_compose_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml"
    )
}

fn is_dockerfile(file_name: &str) -> bool {
    file_name == "Dockerfile" || file_name.starts_with("Dockerfile.")
}
