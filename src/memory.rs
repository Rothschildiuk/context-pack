//! `.context-pack/memory.md`: durable notes written by agents and humans.
//! The tool only ever owns the metadata block; everything else is preserved
//! byte for byte and surfaced in every briefing.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{GitInfo, MemoryInfo};

pub const MEMORY_PATH: &str = ".context-pack/memory.md";
const STALE_MEMORY_AFTER_SECS: u64 = 7 * 24 * 60 * 60;
const METADATA_HEADING: &str = "## Memory Metadata";
const CREATED_AT_UNIX_PREFIX: &str = "- created_at_unix: ";
const CREATED_AT_UTC_PREFIX: &str = "- created_at_utc: ";
const REFRESHED_AT_UNIX_PREFIX: &str = "- refreshed_at_unix: ";
const REFRESHED_AT_UTC_PREFIX: &str = "- refreshed_at_utc: ";

pub const REQUIRED_METADATA: &[&str] = &[
    METADATA_HEADING,
    CREATED_AT_UNIX_PREFIX,
    CREATED_AT_UTC_PREFIX,
    REFRESHED_AT_UNIX_PREFIX,
    REFRESHED_AT_UTC_PREFIX,
];

#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub created_at_unix: u64,
    pub created_at_utc: String,
    pub refreshed_at_unix: u64,
    pub refreshed_at_utc: String,
}

impl MemoryMetadata {
    pub fn now() -> Self {
        let now_unix = current_unix_seconds();
        let now_utc = format_unix_timestamp(now_unix);
        Self {
            created_at_unix: now_unix,
            created_at_utc: now_utc.clone(),
            refreshed_at_unix: now_unix,
            refreshed_at_utc: now_utc,
        }
    }

    fn refreshed(&self) -> Self {
        let now = Self::now();
        Self {
            created_at_unix: self.created_at_unix,
            created_at_utc: self.created_at_utc.clone(),
            ..now
        }
    }

    fn render(&self) -> String {
        format!(
            "{METADATA_HEADING}\n{CREATED_AT_UNIX_PREFIX}{}\n{CREATED_AT_UTC_PREFIX}{}\n{REFRESHED_AT_UNIX_PREFIX}{}\n{REFRESHED_AT_UTC_PREFIX}{}\n",
            self.created_at_unix, self.created_at_utc, self.refreshed_at_unix, self.refreshed_at_utc
        )
    }
}

pub fn template(repo_name: &str) -> String {
    format!(
        concat!(
            "# Repo Memory: {}\n\n",
            "<!-- Durable knowledge that is not obvious from the code: rules, pitfalls,\n",
            "invariants, decisions, and commands that need special setup. One bullet per\n",
            "fact. context-pack shows this file in every briefing and only ever rewrites\n",
            "the metadata block below. -->\n\n",
            "{}\n",
            "## Notes\n"
        ),
        repo_name,
        MemoryMetadata::now().render()
    )
}

/// Bump `refreshed_at` (the notes were reviewed) while preserving every other byte.
pub fn refresh(content: &str) -> String {
    let metadata = parse_metadata(content)
        .map(|existing| existing.refreshed())
        .unwrap_or_else(MemoryMetadata::now);
    let rendered = metadata.render();

    let lines = content.lines().collect::<Vec<_>>();
    if let Some(start) = lines
        .iter()
        .position(|line| line.trim() == METADATA_HEADING)
    {
        let end = lines[start + 1..]
            .iter()
            .position(|line| {
                !line.trim_start().starts_with("- ") && !line.trim().is_empty()
                    || line.starts_with("## ")
            })
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());
        let mut output = lines[..start].join("\n");
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&rendered);
        let rest = lines[end..].join("\n");
        if !rest.is_empty() {
            output.push('\n');
            output.push_str(&rest);
        }
        output.push('\n');
        return output;
    }

    // No metadata yet: insert it after the first heading (or at the top).
    let insert_at = lines
        .iter()
        .position(|line| line.starts_with("# "))
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut output = lines[..insert_at].join("\n");
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(&rendered);
    output.push('\n');
    output.push_str(&lines[insert_at..].join("\n"));
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

/// Append a bullet at the end of the `## Notes` section (created if missing).
pub fn add_note(content: &str, note: &str) -> String {
    let bullet = format!("- {}", note.trim().trim_start_matches("- "));
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    match lines.iter().position(|line| line.trim() == "## Notes") {
        Some(start) => {
            let section_end = lines[start + 1..]
                .iter()
                .position(|line| line.starts_with("## "))
                .map(|offset| start + 1 + offset)
                .unwrap_or(lines.len());
            // Insert after the last non-blank line of the section.
            let mut insert_at = section_end;
            while insert_at > start + 1 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            lines.insert(insert_at, bullet);
        }
        None => {
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            lines.push(String::new());
            lines.push("## Notes".to_string());
            lines.push(bullet);
        }
    }
    let mut output = lines.join("\n");
    output.push('\n');
    output
}

/// Read the memory file for the briefing: notes without metadata and comments.
pub fn inspect(root: &Path, git: Option<&GitInfo>) -> Option<MemoryInfo> {
    let content = fs::read_to_string(root.join(MEMORY_PATH)).ok()?;
    let metadata = parse_metadata(&content);
    Some(MemoryInfo {
        path: MEMORY_PATH.to_string(),
        refreshed_at: metadata
            .as_ref()
            .map(|value| value.refreshed_at_utc.clone()),
        stale_reason: metadata.as_ref().and_then(|value| stale_reason(value, git)),
        notes: notes_body(&content),
        truncated: false,
    })
}

fn notes_body(content: &str) -> String {
    let mut output = Vec::new();
    let mut in_metadata = false;
    let mut in_comment = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if in_comment {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if trimmed.starts_with("<!--") {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if trimmed == METADATA_HEADING {
            in_metadata = true;
            continue;
        }
        if in_metadata {
            if trimmed.starts_with("## ") {
                in_metadata = false;
            } else {
                continue;
            }
        }
        if line.starts_with("# ") || trimmed == "- none yet" {
            continue;
        }
        output.push(line);
    }

    // Drop headings that ended up with no content under them.
    let mut kept: Vec<&str> = Vec::new();
    for (index, line) in output.iter().enumerate() {
        if line.starts_with('#') {
            let has_body = output[index + 1..]
                .iter()
                .take_while(|next| !next.starts_with('#'))
                .any(|next| !next.trim().is_empty());
            if !has_body {
                continue;
            }
        }
        if line.trim().is_empty() && kept.last().is_none_or(|last| last.trim().is_empty()) {
            continue;
        }
        kept.push(line);
    }
    kept.join("\n").trim().to_string()
}

fn parse_metadata(content: &str) -> Option<MemoryMetadata> {
    Some(MemoryMetadata {
        created_at_unix: parse_numeric_value(content, CREATED_AT_UNIX_PREFIX)?,
        created_at_utc: parse_string_value(content, CREATED_AT_UTC_PREFIX)?,
        refreshed_at_unix: parse_numeric_value(content, REFRESHED_AT_UNIX_PREFIX)?,
        refreshed_at_utc: parse_string_value(content, REFRESHED_AT_UTC_PREFIX)?,
    })
}

fn stale_reason(metadata: &MemoryMetadata, git: Option<&GitInfo>) -> Option<String> {
    let now_unix = current_unix_seconds();
    if now_unix.saturating_sub(metadata.refreshed_at_unix) < STALE_MEMORY_AFTER_SECS {
        return None;
    }
    let git = git?;
    let newer_commits = git
        .latest_commit_unix
        .is_some_and(|value| value > metadata.refreshed_at_unix);
    if newer_commits || !git.working_changes.is_empty() {
        return Some(format!(
            "last reviewed {}; the repo changed since. Re-check the notes, then run `context-pack memory refresh`.",
            metadata.refreshed_at_utc
        ));
    }
    None
}

fn parse_numeric_value(content: &str, prefix: &str) -> Option<u64> {
    content
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn parse_string_value(content: &str, prefix: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(|value| value.trim().to_string())
}

fn current_unix_seconds() -> u64 {
    system_time_to_unix_seconds(SystemTime::now()).unwrap_or(0)
}

fn system_time_to_unix_seconds(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_secs())
}

fn format_unix_timestamp(value: u64) -> String {
    let days = (value / 86_400) as i64;
    let seconds_of_day = value % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u8, u8) {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted / 146_097
    } else {
        (shifted - 146_096) / 146_097
    };
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u8, day as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_preserves_notes_and_created_timestamp() {
        let original = "# Repo Memory\n\n## Memory Metadata\n- created_at_unix: 10\n- created_at_utc: 1970-01-01T00:00:10Z\n- refreshed_at_unix: 10\n- refreshed_at_utc: 1970-01-01T00:00:10Z\n\n## Notes\n- keep me\n";
        let refreshed = refresh(original);
        assert!(refreshed.contains("- created_at_unix: 10\n"));
        assert!(!refreshed.contains("- refreshed_at_unix: 10\n"));
        assert!(refreshed.ends_with("## Notes\n- keep me\n"));
    }

    #[test]
    fn refresh_adds_metadata_to_hand_written_file() {
        let refreshed = refresh("# Notes\n- the queue retries forever\n");
        assert!(refreshed.starts_with("# Notes\n\n## Memory Metadata\n"));
        assert!(refreshed.contains("- the queue retries forever"));
    }

    #[test]
    fn add_note_appends_inside_notes_section() {
        let content = "# M\n\n## Notes\n- one\n\n## Later\n- other\n";
        assert_eq!(
            add_note(content, "two"),
            "# M\n\n## Notes\n- one\n- two\n\n## Later\n- other\n"
        );
        assert_eq!(add_note("# M\n", "first"), "# M\n\n## Notes\n- first\n");
    }

    #[test]
    fn notes_body_skips_metadata_comments_and_empty_sections() {
        let content = format!(
            "{}- `make test` needs Docker\n\n## Empty\n",
            template("demo")
        );
        assert_eq!(notes_body(&content), "## Notes\n- `make test` needs Docker");
    }
}
