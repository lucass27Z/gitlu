use crate::models::stash::{StashEntry, StashQuickStat};
use crate::models::status::FileStatus;
use crate::parsers::status::parse_name_status_z;

/// Parse the output of `git stash list --format=%gd%x1f%gs`.
///
/// Each line: `stash@{N}<US>message`
pub fn parse_stash_list(output: &str) -> Result<Vec<StashEntry>, String> {
    let mut result = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(2, '\x1f');
        let reference = parts.next().ok_or("missing stash reference")?.to_string();
        let message = parts.next().unwrap_or("").to_string();

        let index = parse_stash_index(&reference)?;
        let is_gitlu = parse_gitlu_stash_message(&message).is_some();
        let branch = parse_branch_from_message(&message);

        result.push(StashEntry {
            index,
            reference,
            message,
            branch,
            is_gitlu,
        });
    }

    Ok(result)
}

/// Parse the summary line from `git stash show --stat` output.
///
/// Example last line: ` 3 files changed, 10 insertions(+), 2 deletions(-)`
pub fn parse_stash_stat(output: &str, reference: &str) -> Result<StashQuickStat, String> {
    // The stat summary is the last non-empty line
    let summary_line = output
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");

    let mut files_changed = 0usize;
    let mut insertions = 0usize;
    let mut deletions = 0usize;

    if summary_line.eq_ignore_ascii_case("no local changes to save") {
        return Ok(StashQuickStat {
            reference: reference.to_string(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        });
    }

    // Parse numbers from the summary line
    // Format: " N file(s) changed, N insertion(s)(+), N deletion(s)(-)"
    for part in summary_line.split(',') {
        let part = part.trim();
        if part.contains("file") && part.contains("changed") {
            if let Some(n) = extract_leading_number(part) {
                files_changed = n;
            }
        } else if part.contains("insertion")
            && let Some(n) = extract_leading_number(part)
        {
            insertions = n;
        } else if part.contains("deletion")
            && let Some(n) = extract_leading_number(part)
        {
            deletions = n;
        }
    }

    Ok(StashQuickStat {
        reference: reference.to_string(),
        files_changed,
        insertions,
        deletions,
    })
}

/// Parse the output of `git stash show --name-status -z <ref>`.
///
/// With `-z`, git outputs NUL-separated alternating fields:
///   `<status>\0<path>\0` for regular changes
///   `<status>\0<new_path>\0<old_path>\0` for renames/copies
pub fn parse_stash_file_status(output: &[u8]) -> Result<Vec<FileStatus>, String> {
    parse_name_status_z(output)
}

/// Parse a `!!Gitlu<from> -> <to>` stash message.
///
/// Returns `(from_branch, to_branch)` if the message matches the pattern.
pub fn parse_gitlu_stash_message(message: &str) -> Option<(String, String)> {
    let marker = "!!Gitlu<";
    let lower = message.to_ascii_lowercase();
    let lower_marker = marker.to_ascii_lowercase();
    let marker_start = lower.find(&lower_marker)?;
    let after_prefix = &message[marker_start + marker.len()..];
    let from_end = after_prefix.find('>')?;
    let from_branch = after_prefix[..from_end].trim();

    let after_from = &after_prefix[from_end + 1..];
    let after_arrow = after_from.strip_prefix(" -> <")?;
    let to_end = after_arrow.find('>')?;
    let to_branch = after_arrow[..to_end].trim();

    if from_branch.is_empty() || to_branch.is_empty() {
        return None;
    }

    Some((from_branch.to_string(), to_branch.to_string()))
}

/// Check whether a stash's target branch matches a given branch name,
/// accounting for `origin/` prefix differences.
pub fn branch_name_matches(stash_target: &str, branch: &str) -> bool {
    stash_target == branch
        || stash_target
            .strip_prefix("origin/")
            .is_some_and(|s| s == branch)
        || branch
            .strip_prefix("origin/")
            .is_some_and(|s| s == stash_target)
}

/// Validate that a stash reference matches the `stash@{N}` pattern.
pub fn validate_stash_ref(reference: &str) -> Result<(), String> {
    if reference.starts_with("stash@{") && reference.ends_with('}') {
        let inner = &reference[7..reference.len() - 1];
        if inner.parse::<usize>().is_ok() {
            return Ok(());
        }
    }
    Err(format!(
        "Invalid stash reference '{reference}': expected format stash@{{N}}"
    ))
}

// ── helpers ──────────────────────────────────────────────────────────

/// Extract the stash index from a reference like `stash@{3}`.
fn parse_stash_index(reference: &str) -> Result<usize, String> {
    if reference.starts_with("stash@{") && reference.ends_with('}') {
        let inner = &reference[7..reference.len() - 1];
        inner
            .parse::<usize>()
            .map_err(|_| format!("invalid stash index in '{reference}'"))
    } else {
        Err(format!("invalid stash reference '{reference}'"))
    }
}

/// Extract branch name from a standard git stash message like "On main: WIP on …".
fn parse_branch_from_message(message: &str) -> Option<String> {
    let stripped = message.strip_prefix("On ")?;
    let colon_idx = stripped.find(':')?;
    let branch = stripped[..colon_idx].trim();
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

/// Extract the first number from a string.
fn extract_leading_number(s: &str) -> Option<usize> {
    let num_str: String = s
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Pure function tests (no git data needed) ─────────────────────

    #[test]
    fn test_parse_gitlu_stash_message() {
        let msg = "!!Gitlu<feature-branch> -> <main>";
        let result = parse_gitlu_stash_message(msg).unwrap();
        assert_eq!(result.0, "feature-branch");
        assert_eq!(result.1, "main");
    }

    #[test]
    fn test_parse_gitlu_stash_message_case_insensitive() {
        // Also matches legacy !!Gitlu pattern (case-insensitive)
        let msg = "!!Gitlu<old-branch> -> <new-branch>";
        let result = parse_gitlu_stash_message(msg).unwrap();
        assert_eq!(result.0, "old-branch");
        assert_eq!(result.1, "new-branch");
    }

    #[test]
    fn test_parse_gitlu_stash_message_invalid() {
        assert!(parse_gitlu_stash_message("Regular stash message").is_none());
        assert!(parse_gitlu_stash_message("!!Gitlu<> -> <main>").is_none());
        assert!(parse_gitlu_stash_message("!!Gitlu<feature> -> <>").is_none());
    }

    #[test]
    fn test_validate_stash_ref() {
        assert!(validate_stash_ref("stash@{0}").is_ok());
        assert!(validate_stash_ref("stash@{42}").is_ok());
        assert!(validate_stash_ref("stash@{abc}").is_err());
        assert!(validate_stash_ref("refs/stash").is_err());
        assert!(validate_stash_ref("stash@{-1}").is_err());
    }

    #[test]
    fn test_branch_name_matches() {
        assert!(branch_name_matches("main", "main"));
        assert!(branch_name_matches("origin/main", "main"));
        assert!(branch_name_matches("main", "origin/main"));
        assert!(!branch_name_matches("feature", "main"));
    }

    #[test]
    fn test_parse_stash_index() {
        assert_eq!(parse_stash_index("stash@{0}").unwrap(), 0);
        assert_eq!(parse_stash_index("stash@{5}").unwrap(), 5);
        assert_eq!(parse_stash_index("stash@{99}").unwrap(), 99);
        assert!(parse_stash_index("invalid").is_err());
        assert!(parse_stash_index("stash@{abc}").is_err());
    }

    #[test]
    fn test_parse_branch_from_message() {
        assert_eq!(
            parse_branch_from_message("On main: WIP changes"),
            Some("main".to_string())
        );
        assert_eq!(
            parse_branch_from_message("On feature/test: some work"),
            Some("feature/test".to_string())
        );
        assert!(parse_branch_from_message("Random message").is_none());
    }

    #[test]
    fn test_extract_leading_number() {
        assert_eq!(extract_leading_number("3 files changed"), Some(3));
        assert_eq!(extract_leading_number("  42 insertions"), Some(42));
        assert_eq!(extract_leading_number("no numbers"), None);
    }

    // ── Edge case tests ──────────────────────────────────────────────

    #[test]
    fn parse_empty_stash_list() {
        let entries = parse_stash_list("").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_empty_stash_file_status() {
        let files = parse_stash_file_status(b"").unwrap();
        assert!(files.is_empty());
    }

    // ── is_gitlu field population tests ─────────────────────────────

    #[test]
    fn test_parse_stash_list_is_gitlu_true_for_gitlu_message() {
        // A stash with the !!Gitlu marker should have is_gitlu=true
        let output = "stash@{0}\x1f!!Gitlu<feature-branch> -> <main>";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_gitlu);
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[0].reference, "stash@{0}");
    }

    #[test]
    fn test_parse_stash_list_is_gitlu_false_for_regular_message() {
        // A regular stash message should have is_gitlu=false
        let output = "stash@{0}\x1fOn main: WIP work";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].is_gitlu);
    }

    #[test]
    fn test_parse_stash_list_mixed_gitlu_and_regular() {
        // Mix of gitlu and regular stashes - flags should be set correctly
        let output = "stash@{0}\x1f!!Gitlu<feature> -> <main>\nstash@{1}\x1fOn main: WIP";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_gitlu);
        assert!(!entries[1].is_gitlu);
    }

    #[test]
    fn test_parse_stash_list_is_gitlu_with_new_branch_suffix() {
        // The "(new)" suffix is added for new-branch stashes - still gitlu
        let output = "stash@{0}\x1f!!Gitlu<main> -> <feature/new-branch> (new)";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_gitlu);
    }

    #[test]
    fn test_parse_stash_list_skips_empty_lines() {
        let output = "stash@{0}\x1fOn main: changes\n\nstash@{1}\x1fOn feature: work";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
    }

    #[test]
    fn test_parse_stash_list_gitlu_case_insensitive() {
        // The marker matching is case-insensitive
        let output = "stash@{0}\x1f!!gitlu<main> -> <feature>";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_gitlu);
    }

    #[test]
    fn test_parse_gitlu_stash_message_preserves_branch_names() {
        // Ensure branch names with slashes and dashes are preserved exactly
        let msg = "!!Gitlu<feature/my-branch> -> <release/1.0>";
        let result = parse_gitlu_stash_message(msg).unwrap();
        assert_eq!(result.0, "feature/my-branch");
        assert_eq!(result.1, "release/1.0");
    }

    #[test]
    fn test_parse_gitlu_stash_message_with_new_suffix_in_full_message() {
        // The " (new)" suffix after the closing > should not break parsing
        let msg = "!!Gitlu<main> -> <feature/branch> (new)";
        let result = parse_gitlu_stash_message(msg).unwrap();
        assert_eq!(result.0, "main");
        assert_eq!(result.1, "feature/branch");
    }

    #[test]
    fn test_parse_gitlu_stash_message_missing_arrow() {
        // Missing " -> " arrow should not match
        assert!(parse_gitlu_stash_message("!!Gitlu<main> <feature>").is_none());
    }

    #[test]
    fn test_parse_stash_list_gitlu_is_not_set_for_partial_marker() {
        // A message with only part of the marker pattern should not be flagged
        let output = "stash@{0}\x1f!!Gitlu<> -> <main>";
        let entries = parse_stash_list(output).unwrap();
        assert_eq!(entries.len(), 1);
        // Empty from_branch → parse_gitlu_stash_message returns None → is_gitlu=false
        assert!(!entries[0].is_gitlu);
    }
}
