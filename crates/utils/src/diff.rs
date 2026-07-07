use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use ts_rs::TS;

// Structs compatable with props: https://github.com/MrWangJustToDo/git-diff-view

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffDetails {
    pub file_name: Option<String>,
    pub content: Option<String>,
}

// Worktree diffs for the diffs tab: minimal, no hunks, optional full contents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub change: DiffChangeKind,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    /// True when file contents are intentionally omitted (e.g., too large)
    pub content_omitted: bool,
    /// Optional precomputed stats for omitted content
    pub additions: Option<usize>,
    pub deletions: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum DiffChangeKind {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    PermissionChange,
}

// ==============================
// Unified diff utility functions
// ==============================

/// Converts a replace diff to a list of unified diff hunks.
/// Uses a context limit of 3 lines.
fn create_unified_diff_hunks(old: &str, new: &str) -> Vec<String> {
    let old = ensure_newline(old);
    let new = ensure_newline(new);

    let diff = TextDiff::from_lines(&old, &new);

    // Generate unified diff with context
    let unified_diff = diff
        .unified_diff()
        .context_radius(3)
        .header("a", "b")
        .to_string();

    extract_unified_diff_hunks(&unified_diff)
}

/// Creates a full unified diff with the file path in the header.
pub fn create_unified_diff(file_path: &str, old: &str, new: &str) -> String {
    let hunks = create_unified_diff_hunks(old, new);
    concatenate_diff_hunks(file_path, &hunks)
}

/// Compute addition/deletion counts between two text snapshots.
pub fn compute_line_change_counts(old: &str, new: &str) -> (usize, usize) {
    let old = ensure_newline(old);
    let new = ensure_newline(new);

    let diff = TextDiff::from_lines(&old, &new);

    let mut additions = 0usize;
    let mut deletions = 0usize;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            ChangeTag::Equal => {}
        }
    }

    (additions, deletions)
}

// ensure a line ends with a newline character
fn ensure_newline(line: &str) -> Cow<'_, str> {
    if line.ends_with('\n') {
        Cow::Borrowed(line)
    } else {
        let mut owned = line.to_owned();
        owned.push('\n');
        Cow::Owned(owned)
    }
}

/// Extracts unified diff hunks from a string containing a full unified diff.
/// Tolerates non-diff lines and missing `@@`` hunk headers.
pub fn extract_unified_diff_hunks(unified_diff: &str) -> Vec<String> {
    let lines = unified_diff.split_inclusive('\n').collect::<Vec<_>>();

    if !lines.iter().any(|l| l.starts_with("@@")) {
        // No @@ hunk headers: treat as a single hunk
        let hunk = lines
            .iter()
            .copied()
            .filter(|line| line.starts_with([' ', '+', '-']))
            .collect::<String>();

        let old_count = lines
            .iter()
            .filter(|line| line.starts_with(['-', ' ']))
            .count();
        let new_count = lines
            .iter()
            .filter(|line| line.starts_with(['+', ' ']))
            .count();

        return if hunk.is_empty() {
            vec![]
        } else {
            vec![format!("@@ -1,{old_count} +1,{new_count} @@\n{hunk}")]
        };
    }

    let mut hunks = vec![];
    let mut current_hunk: Option<String> = None;

    // Collect hunks starting with @@ headers
    for line in lines {
        if line.starts_with("@@") {
            // new hunk starts
            if let Some(hunk) = current_hunk.take() {
                // flush current hunk
                if !hunk.is_empty() {
                    hunks.push(hunk);
                }
            }
            current_hunk = Some(line.to_string());
        } else if let Some(ref mut hunk) = current_hunk {
            if line.starts_with([' ', '+', '-']) {
                // hunk content
                hunk.push_str(line);
            } else {
                // unkown line, flush current hunk
                if !hunk.is_empty() {
                    hunks.push(hunk.clone());
                }
                current_hunk = None;
            }
        }
    }
    // we have reached the end. flush the last hunk if it exists
    if let Some(hunk) = current_hunk
        && !hunk.is_empty()
    {
        hunks.push(hunk);
    }

    // Fix hunk headers if they are empty @@\n
    hunks = fix_hunk_headers(hunks);

    hunks
}

// Helper function to ensure valid hunk headers
fn fix_hunk_headers(hunks: Vec<String>) -> Vec<String> {
    if hunks.is_empty() {
        return hunks;
    }

    let mut new_hunks = Vec::new();
    // if hunk header is empty @@\n, ten we need to replace it with a valid header
    for hunk in hunks {
        let mut lines = hunk
            .split_inclusive('\n')
            .map(str::to_string)
            .collect::<Vec<_>>();
        if lines.len() < 2 {
            // empty hunk, skip
            continue;
        }

        let header = &lines[0];
        if !header.starts_with("@@") {
            // no header, skip
            continue;
        }

        if header.trim() == "@@" {
            // empty header, replace with a valid one
            lines.remove(0);
            let old_count = lines
                .iter()
                .filter(|line| line.starts_with(['-', ' ']))
                .count();
            let new_count = lines
                .iter()
                .filter(|line| line.starts_with(['+', ' ']))
                .count();
            let new_header = format!("@@ -1,{old_count} +1,{new_count} @@");
            lines.insert(0, new_header);
            new_hunks.push(lines.join(""));
        } else {
            // valid header, keep as is
            new_hunks.push(hunk);
        }
    }

    new_hunks
}

/// Creates a full unified diff with the file path in the header,
pub fn concatenate_diff_hunks(file_path: &str, hunks: &[String]) -> String {
    let mut unified_diff = String::new();

    let header = format!("--- a/{file_path}\n+++ b/{file_path}\n");

    unified_diff.push_str(&header);

    if !hunks.is_empty() {
        let lines = hunks
            .iter()
            .flat_map(|hunk| hunk.lines())
            .filter(|line| line.starts_with("@@ ") || line.starts_with([' ', '+', '-']))
            .collect::<Vec<_>>();
        unified_diff.push_str(lines.join("\n").as_str());
        if !unified_diff.ends_with('\n') {
            unified_diff.push('\n');
        }
    }

    unified_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── create_unified_diff ────────────────────────────────────────────

    #[test]
    fn create_unified_diff_identical_files() {
        let content = "line1\nline2\nline3\n";
        let result = create_unified_diff("test.txt", content, content);
        assert!(result.contains("--- a/test.txt"));
        assert!(result.contains("+++ b/test.txt"));
    }

    #[test]
    fn create_unified_diff_added_line() {
        let old = "line1\n";
        let new = "line1\nline2\n";
        let result = create_unified_diff("test.txt", old, new);
        assert!(result.contains("+line2"));
    }

    #[test]
    fn create_unified_diff_removed_line() {
        let old = "line1\nline2\n";
        let new = "line1\n";
        let result = create_unified_diff("test.txt", old, new);
        assert!(result.contains("-line2"));
    }

    #[test]
    fn create_unified_diff_modified_line() {
        let old = "hello world\n";
        let new = "hello universe\n";
        let result = create_unified_diff("test.txt", old, new);
        assert!(result.contains("-hello world"));
        assert!(result.contains("+hello universe"));
    }

    #[test]
    fn create_unified_diff_empty_strings() {
        let result = create_unified_diff("empty.txt", "", "");
        assert!(result.contains("--- a/empty.txt"));
        assert!(result.contains("+++ b/empty.txt"));
    }

    #[test]
    fn create_unified_diff_content_without_trailing_newline() {
        let old = "line1";
        let new = "line1\nline2";
        let result = create_unified_diff("test.txt", old, new);
        assert!(result.contains("+line2"));
    }

    // ── compute_line_change_counts ─────────────────────────────────────

    #[test]
    fn compute_line_change_counts_identical() {
        let content = "a\nb\nc\n";
        let (additions, deletions) = compute_line_change_counts(content, content);
        assert_eq!(additions, 0);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn compute_line_change_counts_only_additions() {
        let old = "";
        let new = "line1\nline2\nline3\n";
        let (additions, _) = compute_line_change_counts(old, new);
        assert!(additions > 0);
    }

    #[test]
    fn compute_line_change_counts_only_deletions() {
        let old = "line1\nline2\nline3\n";
        let new = "";
        let (_, deletions) = compute_line_change_counts(old, new);
        assert!(deletions > 0);
    }

    #[test]
    fn compute_line_change_counts_mixed_changes() {
        let old = "keep\nremove\nkeep2\n";
        let new = "keep\nkeep2\nadded\n";
        let (additions, deletions) = compute_line_change_counts(old, new);
        assert_eq!(additions, 1);
        assert_eq!(deletions, 1);
    }

    #[test]
    fn compute_line_change_counts_empty_both() {
        let (additions, deletions) = compute_line_change_counts("", "");
        assert_eq!(additions, 0);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn compute_line_change_counts_no_trailing_newline() {
        let old = "line1";
        let new = "line1\nline2";
        let (additions, deletions) = compute_line_change_counts(old, new);
        assert_eq!(additions, 1);
        assert_eq!(deletions, 0);
    }

    // ── extract_unified_diff_hunks ─────────────────────────────────────

    #[test]
    fn extract_unified_diff_hunks_no_hunks_header() {
        let diff = " line1\n-line2\n+line3\n";
        let hunks = extract_unified_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert!(hunks[0].starts_with("@@"));
    }

    #[test]
    fn extract_unified_diff_hunks_empty_input() {
        let hunks = extract_unified_diff_hunks("");
        assert!(hunks.is_empty());
    }

    #[test]
    fn extract_unified_diff_hunks_with_headers() {
        let diff = "@@ -1,3 +1,4 @@\n line1\n-line2\n+line3\n+line4\n";
        let hunks = extract_unified_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert!(hunks[0].starts_with("@@ -1,3 +1,4 @@"));
        assert!(hunks[0].contains("-line2"));
        assert!(hunks[0].contains("+line3"));
    }

    #[test]
    fn extract_unified_diff_hunks_multiple_hunks() {
        let diff = "@@ -1,3 +1,3 @@\n line1\n-line2\n+line3\n@@ -10,3 +10,4 @@\n line10\n+line11\n";
        let hunks = extract_unified_diff_hunks(diff);
        assert_eq!(hunks.len(), 2);
    }

    #[test]
    fn extract_unified_diff_hunks_non_diff_lines_filtered() {
        let diff = "some header\n line1\n-line2\n+line3\nsome footer\n";
        let hunks = extract_unified_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert!(!hunks[0].contains("header"));
        assert!(!hunks[0].contains("footer"));
    }

    #[test]
    fn extract_unified_diff_hunks_empty_header_fix() {
        let diff = "@@\n -old\n +new\n";
        let hunks = extract_unified_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert!(hunks[0].starts_with("@@ -1,"));
    }

    // ── concatenate_diff_hunks ─────────────────────────────────────────

    #[test]
    fn concatenate_diff_hunks_single_hunk() {
        let hunks = vec!["@@ -1,2 +1,2 @@\n line1\n-line2\n+line3\n".to_string()];
        let result = concatenate_diff_hunks("test.txt", &hunks);
        assert!(result.contains("--- a/test.txt"));
        assert!(result.contains("+++ b/test.txt"));
        assert!(result.contains("@@ -1,2 +1,2 @@"));
        assert!(result.contains("-line2"));
        assert!(result.contains("+line3"));
    }

    #[test]
    fn concatenate_diff_hunks_empty_hunks() {
        let hunks: Vec<String> = vec![];
        let result = concatenate_diff_hunks("test.txt", &hunks);
        assert!(result.contains("--- a/test.txt"));
        assert!(result.contains("+++ b/test.txt"));
    }

    #[test]
    fn concatenate_diff_hunks_multiple_hunks() {
        let hunks = vec![
            "@@ -1,2 +1,2 @@\n line1\n-line2\n+line3\n".to_string(),
            "@@ -10,2 +10,2 @@\n line10\n-line11\n+line12\n".to_string(),
        ];
        let result = concatenate_diff_hunks("test.txt", &hunks);
        assert!(result.contains("@@ -1,2 +1,2 @@"));
        assert!(result.contains("@@ -10,2 +10,2 @@"));
    }

    #[test]
    fn concatenate_diff_hunks_ends_with_newline() {
        let hunks = vec!["@@ -1,1 +1,1 @@\n line1\n".to_string()];
        let result = concatenate_diff_hunks("test.txt", &hunks);
        assert!(result.ends_with('\n'));
    }

    // ── integration: create_unified_diff → extract → concatenate ──────

    #[test]
    fn round_trip_create_and_parse_diff() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline2_modified\nline3\nline4\n";
        let unified = create_unified_diff("roundtrip.txt", old, new);
        assert!(unified.contains("--- a/roundtrip.txt"));
        let hunks = extract_unified_diff_hunks(&unified);
        assert!(!hunks.is_empty());
        let reconstructed = concatenate_diff_hunks("roundtrip.txt", &hunks);
        assert!(reconstructed.contains("--- a/roundtrip.txt"));
    }
}
