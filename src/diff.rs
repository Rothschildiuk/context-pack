use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

use crate::cli::CliError;

pub fn render_diff_from_files(from: &Path, to: &Path) -> Result<String, CliError> {
    let left = std::fs::read_to_string(from).map_err(|source| CliError::Io {
        action: "read diff input",
        path: from.to_path_buf(),
        source,
    })?;
    let right = std::fs::read_to_string(to).map_err(|source| CliError::Io {
        action: "read diff input",
        path: to.to_path_buf(),
        source,
    })?;

    Ok(render_text_diff(from, to, &left, &right))
}

fn render_text_diff(from: &Path, to: &Path, left: &str, right: &str) -> String {
    let left_lines = left.lines().collect::<BTreeSet<_>>();
    let right_lines = right.lines().collect::<BTreeSet<_>>();

    let added = right_lines.difference(&left_lines).count();
    let removed = left_lines.difference(&right_lines).count();
    let common = left_lines.intersection(&right_lines).count();

    let mut output = String::new();
    output.push_str("# Context Pack Diff\n\n");
    output.push_str(&format!("- from: {}\n", from.display()));
    output.push_str(&format!("- to: {}\n", to.display()));
    output.push_str(&format!("- lines added: {added}\n"));
    output.push_str(&format!("- lines removed: {removed}\n"));
    output.push_str(&format!("- lines unchanged: {common}\n"));

    if let Some((keys_added, keys_removed)) = json_key_diff(left, right) {
        output.push_str(&format!("- json keys added: {}\n", keys_added.join(", ")));
        output.push_str(&format!("- json keys removed: {}\n", keys_removed.join(", ")));
    }

    output.push('\n');
    output.push_str("## Added Lines (Top 20)\n");
    for line in right_lines.difference(&left_lines).take(20) {
        output.push_str(&format!("- + {}\n", line));
    }
    if added == 0 {
        output.push_str("- none\n");
    }

    output.push('\n');
    output.push_str("## Removed Lines (Top 20)\n");
    for line in left_lines.difference(&right_lines).take(20) {
        output.push_str(&format!("- - {}\n", line));
    }
    if removed == 0 {
        output.push_str("- none\n");
    }

    output
}

fn json_key_diff(left: &str, right: &str) -> Option<(Vec<String>, Vec<String>)> {
    let left = serde_json::from_str::<Value>(left).ok()?;
    let right = serde_json::from_str::<Value>(right).ok()?;
    let left_keys = left.as_object()?.keys().cloned().collect::<BTreeSet<_>>();
    let right_keys = right.as_object()?.keys().cloned().collect::<BTreeSet<_>>();

    let added = right_keys
        .difference(&left_keys)
        .cloned()
        .collect::<Vec<_>>();
    let removed = left_keys
        .difference(&right_keys)
        .cloned()
        .collect::<Vec<_>>();

    Some((added, removed))
}
