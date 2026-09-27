use crate::model::Brief;

pub fn render(brief: &Brief) -> String {
    let mut output = serde_json::to_string_pretty(brief).unwrap_or_else(|error| {
        serde_json::json!({ "error": format!("failed to serialize briefing: {error}") }).to_string()
    });
    output.push('\n');
    output
}
