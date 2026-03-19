use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::GitResult;

const STALE_MEMORY_AFTER_SECS: u64 = 7 * 24 * 60 * 60;
const CREATED_AT_UNIX_PREFIX: &str = "- created_at_unix: ";
const CREATED_AT_UTC_PREFIX: &str = "- created_at_utc: ";
const REFRESHED_AT_UNIX_PREFIX: &str = "- refreshed_at_unix: ";
const REFRESHED_AT_UTC_PREFIX: &str = "- refreshed_at_utc: ";

#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub created_at_unix: u64,
    pub created_at_utc: String,
    pub refreshed_at_unix: u64,
    pub refreshed_at_utc: String,
}

#[derive(Debug, Clone)]
pub struct RepoMemoryStatus {
    pub created_at_utc: String,
    pub refreshed_at_utc: String,
    pub stale_reason: Option<String>,
}

impl RepoMemoryStatus {
    pub fn is_stale(&self) -> bool {
        self.stale_reason.is_some()
    }
}

pub fn load_existing_memory_metadata(memory_path: &Path) -> Option<MemoryMetadata> {
    if !memory_path.exists() {
        return None;
    }

    let content = fs::read_to_string(memory_path).ok();
    if let Some(content) = &content {
        if let Some(metadata) = parse_memory_metadata(content) {
            return Some(metadata);
        }
    }

    let modified = fs::metadata(memory_path).ok()?.modified().ok()?;
    let modified_unix = system_time_to_unix_seconds(modified)?;
    let modified_utc = format_unix_timestamp(modified_unix);
    Some(MemoryMetadata {
        created_at_unix: modified_unix,
        created_at_utc: modified_utc.clone(),
        refreshed_at_unix: modified_unix,
        refreshed_at_utc: modified_utc,
    })
}

pub fn next_memory_metadata(existing: Option<&MemoryMetadata>) -> MemoryMetadata {
    let now_unix = current_unix_seconds();
    let now_utc = format_unix_timestamp(now_unix);

    match existing {
        Some(existing) => MemoryMetadata {
            created_at_unix: existing.created_at_unix,
            created_at_utc: existing.created_at_utc.clone(),
            refreshed_at_unix: now_unix,
            refreshed_at_utc: now_utc,
        },
        None => MemoryMetadata {
            created_at_unix: now_unix,
            created_at_utc: now_utc.clone(),
            refreshed_at_unix: now_unix,
            refreshed_at_utc: now_utc,
        },
    }
}

pub fn inspect_repo_memory(memory_path: &Path, git: &GitResult) -> Option<RepoMemoryStatus> {
    let metadata = load_existing_memory_metadata(memory_path)?;

    Some(RepoMemoryStatus {
        created_at_utc: metadata.created_at_utc.clone(),
        refreshed_at_utc: metadata.refreshed_at_utc.clone(),
        stale_reason: stale_reason(&metadata, git),
    })
}

pub fn render_metadata_section(metadata: &MemoryMetadata) -> String {
    format!(
        concat!(
            "## Memory Metadata\n",
            "- created_at_unix: {}\n",
            "- created_at_utc: {}\n",
            "- refreshed_at_unix: {}\n",
            "- refreshed_at_utc: {}\n",
            "- refresh_policy: Refresh this file if it is older than 7 days and repo development has continued.\n\n"
        ),
        metadata.created_at_unix,
        metadata.created_at_utc,
        metadata.refreshed_at_unix,
        metadata.refreshed_at_utc
    )
}

fn parse_memory_metadata(content: &str) -> Option<MemoryMetadata> {
    Some(MemoryMetadata {
        created_at_unix: parse_numeric_value(content, CREATED_AT_UNIX_PREFIX)?,
        created_at_utc: parse_string_value(content, CREATED_AT_UTC_PREFIX)?,
        refreshed_at_unix: parse_numeric_value(content, REFRESHED_AT_UNIX_PREFIX)?,
        refreshed_at_utc: parse_string_value(content, REFRESHED_AT_UTC_PREFIX)?,
    })
}

fn stale_reason(metadata: &MemoryMetadata, git: &GitResult) -> Option<String> {
    let now_unix = current_unix_seconds();
    if now_unix.saturating_sub(metadata.refreshed_at_unix) < STALE_MEMORY_AFTER_SECS {
        return None;
    }

    if !git.changes.is_empty() {
        return Some(format!(
            "Repo memory may be stale: last refreshed {} and the working tree has newer changes.",
            metadata.refreshed_at_utc
        ));
    }

    if git
        .latest_commit_unix
        .is_some_and(|value| value > metadata.refreshed_at_unix)
    {
        return Some(format!(
            "Repo memory may be stale: last refreshed {} and git history has newer commits.",
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
