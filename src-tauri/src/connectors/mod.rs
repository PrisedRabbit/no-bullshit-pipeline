pub mod cli_agent;
pub mod save;
pub mod shell;

/// Strip YAML frontmatter from markdown content if present.
/// Returns the content after the closing `---` delimiter, trimmed of leading newlines.
/// If no frontmatter is found, returns the original content unchanged.
pub fn strip_frontmatter(content: &str) -> &str {
    if let Some(after_open) = content.strip_prefix("---")
        && let Some(end_idx) = after_open.find("---")
    {
        let after = &after_open[end_idx + 3..];
        return after.trim_start_matches('\n').trim_start_matches('\r');
    }
    content
}
