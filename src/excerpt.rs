//! Bounded excerpts: the opening of guidance docs, and an outline of
//! declarations for source files. Secrets are redacted before anything is
//! emitted.

use std::path::Path;

use crate::index::RepoIndex;
use crate::model::Excerpt;
use crate::paths;

pub fn excerpt(index: &RepoIndex, path: &Path, budget: usize) -> Option<Excerpt> {
    if budget < 120 || is_sensitive_file(path) {
        return None;
    }
    let text = index.read(path)?;
    let (text, redacted) = if paths::is_source(path) {
        (text, false)
    } else {
        redact(&text)
    };

    let (content, truncated) = if paths::is_source(path) {
        outline(&text, budget)
    } else {
        leading_text(&text, budget)
    };
    if content.trim().is_empty() {
        return None;
    }
    Some(Excerpt {
        path: paths::display(path),
        content,
        truncated,
        redacted,
    })
}

/// Opening lines of a document, skipping badges, HTML and blank runs.
fn leading_text(text: &str, budget: usize) -> (String, bool) {
    let mut output = String::new();
    let mut previous_blank = true;
    let mut truncated = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        let is_noise = trimmed.starts_with("[![")
            || trimmed.starts_with("![")
            || trimmed.starts_with('<') && !trimmed.starts_with("<!--")
            || trimmed.starts_with("<!--") && trimmed.ends_with("-->");
        if is_noise {
            continue;
        }
        let blank = trimmed.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        if output.len() + trimmed.len() + 1 > budget {
            truncated = true;
            break;
        }
        output.push_str(trimmed);
        output.push('\n');
        previous_blank = blank;
    }
    (output.trim_end().to_string(), truncated)
}

/// Declarations only: an agent can read the body itself once it knows where to look.
fn outline(text: &str, budget: usize) -> (String, bool) {
    let mut output = String::new();
    let mut truncated = false;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !is_declaration(trimmed) {
            continue;
        }
        let rendered = format!(
            "{:>5}: {}\n",
            number + 1,
            line.trim_end().trim_end_matches('{').trim_end()
        );
        if output.len() + rendered.len() > budget {
            truncated = true;
            break;
        }
        output.push_str(&rendered);
    }
    (output.trim_end().to_string(), truncated)
}

fn is_declaration(line: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "pub fn ",
        "pub(crate) fn ",
        "pub async fn ",
        "fn ",
        "async fn ",
        "pub struct ",
        "struct ",
        "pub enum ",
        "enum ",
        "pub trait ",
        "trait ",
        "impl ",
        "impl<",
        "pub mod ",
        "mod ",
        "def ",
        "async def ",
        "class ",
        "func ",
        "type ",
        "interface ",
        "export function ",
        "export async function ",
        "export default function ",
        "export class ",
        "export interface ",
        "export type ",
        "export const ",
        "export default class ",
        "function ",
        "public class ",
        "public interface ",
        "public enum ",
        "public record ",
        "public static ",
        "public abstract class ",
        "public final class ",
        "abstract class ",
        "final class ",
        "data class ",
        "object ",
        "module ",
        "defmodule ",
        "protocol ",
        "extension ",
        "@app.",
        "@router.",
        "router.",
    ];
    PREFIXES.iter().any(|prefix| line.starts_with(prefix)) && !line.ends_with(';')
        || line.starts_with("mod ") && line.ends_with(';')
}

pub fn is_sensitive_file(path: &Path) -> bool {
    let name = paths::file_name(path).to_ascii_lowercase();
    if matches!(
        name.as_str(),
        ".env" | ".npmrc" | ".pypirc" | ".netrc" | "id_rsa" | "id_ed25519" | "credentials"
    ) {
        return true;
    }
    if name.starts_with(".env.")
        && !matches!(
            name.as_str(),
            ".env.example" | ".env.sample" | ".env.template"
        )
    {
        return true;
    }
    name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
        || name.ends_with(".pfx")
        || key_tokens(&name).iter().any(|token| {
            matches!(
                token.as_str(),
                "secret" | "secrets" | "credential" | "credentials" | "token" | "tokens"
            )
        })
}

/// Replace values of secret-looking keys in `key = value` / `key: value` lines.
pub fn redact(text: &str) -> (String, bool) {
    let mut redacted = false;
    let lines = text
        .lines()
        .map(|line| match redact_line(line) {
            Some(replaced) => {
                redacted = true;
                replaced
            }
            None => line.to_string(),
        })
        .collect::<Vec<_>>();
    if redacted {
        (lines.join("\n"), true)
    } else {
        (text.to_string(), false)
    }
}

fn redact_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        return None;
    }
    let delimiter_index = line.find(['=', ':'])?;
    let (key, rest) = line.split_at(delimiter_index);
    let value = rest[1..].trim();
    let bare_key = key.trim().trim_start_matches("- ").trim();
    if value.is_empty() || bare_key.contains(['(', ')', ' ']) {
        return None;
    }
    if !is_secret_key(bare_key) || looks_like_placeholder(value) {
        return None;
    }
    let delimiter = &rest[..1];
    let spacing = if rest[1..].starts_with(' ') { " " } else { "" };
    let quote = value
        .chars()
        .next()
        .filter(|ch| *ch == '"' || *ch == '\'')
        .map(String::from)
        .unwrap_or_default();
    Some(format!("{key}{delimiter}{spacing}{quote}[REDACTED]{quote}"))
}

fn is_secret_key(key: &str) -> bool {
    let tokens = key_tokens(key.trim().trim_start_matches('-').trim_matches(['"', '\'']));
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    has("password")
        || has("passwd")
        || has("pwd")
        || has("secret")
        || has("token")
        || has("apikey")
        || has("credential")
        || has("credentials")
        || (has("api")
            || has("private")
            || has("access")
            || has("secret")
            || has("signing")
            || has("encryption")
            || has("client"))
            && has("key")
        || tokens.last().is_some_and(|last| last == "key") && tokens.len() > 1
}

/// Split `DB_PASSWORD`, `apiKey`, `client-secret` into lowercase words.
fn key_tokens(key: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for ch in key.chars() {
        if !ch.is_alphanumeric() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            previous_lower = false;
            continue;
        }
        if ch.is_uppercase() && previous_lower && !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
        previous_lower = ch.is_lowercase() || ch.is_ascii_digit();
        current.push(ch.to_ascii_lowercase());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn looks_like_placeholder(value: &str) -> bool {
    let value = value.trim_matches(['"', '\'', ',']).trim();
    value.is_empty()
        || value.starts_with("${")
        || value.starts_with('<')
        || value.starts_with('[')
        || value.starts_with('{')
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "changeme"
                | "change-me"
                | "xxx"
                | "todo"
                | "null"
                | "none"
                | "true"
                | "false"
                | "your-key-here"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_and_monkeys_are_not_secrets() {
        let (text, redacted) = redact("\"keywords\": [\"cli\"],\nmonkey: banana\ntokenizer: bpe\n");
        assert!(!redacted, "{text}");
    }

    #[test]
    fn real_secret_keys_are_redacted() {
        let (text, redacted) = redact("DB_PASSWORD=hunter2\napiKey: \"abc\"\nSTRIPE_SECRET_KEY=sk_live\nPUBLIC_URL=https://x\n");
        assert!(redacted);
        assert!(text.contains("DB_PASSWORD=[REDACTED]"));
        assert!(text.contains("apiKey: \"[REDACTED]\""));
        assert!(text.contains("STRIPE_SECRET_KEY=[REDACTED]"));
        let (yaml, _) = redact("services:\n  db:\n    environment:\n      - POSTGRES_PASSWORD=pw\n      password: pw\n");
        assert!(
            yaml.contains("      - POSTGRES_PASSWORD=[REDACTED]"),
            "{yaml}"
        );
        assert!(yaml.contains("      password: [REDACTED]"), "{yaml}");
        assert!(text.contains("PUBLIC_URL=https://x"));
    }

    #[test]
    fn placeholders_stay_readable() {
        let (_, redacted) = redact("API_KEY=${API_KEY}\nTOKEN=<your token>\n");
        assert!(!redacted);
    }

    #[test]
    fn env_files_are_sensitive_but_templates_are_not() {
        assert!(is_sensitive_file(Path::new(".env")));
        assert!(is_sensitive_file(Path::new("config/.env.production")));
        assert!(is_sensitive_file(Path::new("secrets.yaml")));
        assert!(!is_sensitive_file(Path::new(".env.example")));
        assert!(!is_sensitive_file(Path::new("src/tokenizer.rs")));
    }

    #[test]
    fn outline_keeps_declarations_with_line_numbers() {
        let (outline, _) = outline("use x;\n\npub fn run() {\n    body();\n}\nstruct A;\n", 400);
        assert_eq!(outline, "    3: pub fn run()");
    }
}
