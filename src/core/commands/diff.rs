use crate::core::objects::{self, blob::Blob, tree::Tree};
const STAT_WIDTH: usize = 80;
    Same(usize, usize),    // (old_idx, new_idx)
    let old_len = old_lines.len();
    let new_len = new_lines.len();

    // Create a matrix for the shortest edit sequence
    let mut dp = vec![vec![0; new_len + 1]; old_len + 1];
    let mut backtrace = vec![vec![(0, 0); new_len + 1]; old_len + 1];

    // Initialize first row and column
    for i in 0..=old_len {
        dp[i][0] = i;
        if i > 0 {
            backtrace[i][0] = (i - 1, 0);
    for j in 0..=new_len {
        dp[0][j] = j;
        if j > 0 {
            backtrace[0][j] = (0, j - 1);
        }
    // Fill the matrices
    for i in 1..=old_len {
        for j in 1..=new_len {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
                backtrace[i][j] = (i - 1, j - 1);
                let delete_cost = dp[i - 1][j] + 1;
                let insert_cost = dp[i][j - 1] + 1;
                let replace_cost = dp[i - 1][j - 1] + 1;

                dp[i][j] = delete_cost.min(insert_cost.min(replace_cost));

                if dp[i][j] == delete_cost {
                    backtrace[i][j] = (i - 1, j);
                } else if dp[i][j] == insert_cost {
                    backtrace[i][j] = (i, j - 1);
                } else {
                    backtrace[i][j] = (i - 1, j - 1);
                }
    // Reconstruct the changes
    let mut changes = Vec::new();
    let mut i = old_len;
    let mut j = new_len;

    while i > 0 || j > 0 {
        let (prev_i, prev_j) = backtrace[i][j];

        if i == prev_i {
            changes.push(Change::Insert(j - 1));
        } else if j == prev_j {
            changes.push(Change::Delete(i - 1));
        } else if old_lines[i - 1] == new_lines[j - 1] {
            changes.push(Change::Same(i - 1, j - 1));
            changes.push(Change::Replace(i - 1, j - 1));

        i = prev_i;
        j = prev_j;
    changes.reverse();
    changes
#[allow(clippy::too_many_lines)]
    let mut current_hunk = String::new();
    let mut old_start = 1;
    let mut new_start = 1;
    let mut old_count = 0;
    let mut new_count = 0;
    let mut last_change_idx = None;
    // Keep track of context lines before changes
    let mut context_buffer = Vec::new();
    for (i, change) in changes.iter().enumerate() {
        match change {
            Change::Same(old_idx, new_idx) => {
                let line = old_lines[*old_idx];

                if last_change_idx.is_none() {
                    // Before any changes, store in context buffer
                    if context_buffer.len() < hunk_context_lines {
                        context_buffer.push(line);
                    } else {
                        context_buffer.remove(0);
                        context_buffer.push(line);
                    }
                } else if let Some(last_idx) = last_change_idx {
                    if i - last_idx <= hunk_context_lines {
                        // Within range of last change
                        current_hunk.push_str(&format!(" {line}\n"));
                        old_count += 1;
                        new_count += 1;
                    } else {
                        // End current hunk if exists
                        if !current_hunk.is_empty() {
                            hunks.push(Hunk {
                                old_start,
                                old_count,
                                new_start,
                                new_count,
                                content: current_hunk,
                            });
                            current_hunk = String::new();
                            context_buffer.clear();
                        }
                        // Start storing context for next potential hunk
                        context_buffer.push(line);
                        old_start = old_idx + 1 - context_buffer.len();
                        new_start = new_idx + 1 - context_buffer.len();
                        old_count = 0;
                        new_count = 0;
            Change::Delete(old_idx) | Change::Replace(old_idx, _) => {
                // Add context buffer if this is the start of a new hunk
                if last_change_idx.is_none() {
                    for line in &context_buffer {
                        current_hunk.push_str(&format!(" {line}\n"));
                        old_count += 1;
                        new_count += 1;
                    // Adjust start positions
                    old_start = old_idx + 1 - context_buffer.len();
                    new_start = old_start;

                if let Change::Delete(idx) = change {
                    current_hunk.push_str(&format!(
                        "{RED}-{}{RESET}\n",
                        old_lines[*idx]
                    ));
                } else if let Change::Replace(old_idx, new_idx) = change {
                    current_hunk.push_str(&format!(
                        "{RED}-{}{RESET}\n",
                        old_lines[*old_idx]
                    current_hunk.push_str(&format!(
                        "{GREEN}+{}{RESET}\n",
                        new_lines[*new_idx]
                last_change_idx = Some(i);
            Change::Insert(new_idx) => {
                // Add context buffer if this is the start of a new hunk
                if last_change_idx.is_none() {
                    for line in &context_buffer {
                        current_hunk.push_str(&format!(" {line}\n"));
                        old_count += 1;
                        new_count += 1;
                    }
                    old_start = *new_idx + 1 - context_buffer.len();
                    new_start = old_start;
                }
                current_hunk.push_str(&format!(
                    "{GREEN}+{}{RESET}\n",
                    new_lines[*new_idx]
                ));
                new_count += 1;
                last_change_idx = Some(i);
            }
        }
    }
    // Add the last hunk if there is one
    if !current_hunk.is_empty() {
            content: current_hunk,
fn format_binary_diff(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary files differ\n")
}

    if Blob::is_binary(content1) || Blob::is_binary(content2) {
        return format_binary_diff(&src_path, &dst_path);
    }

    let old_str = String::from_utf8_lossy(content1);
    let new_str = String::from_utf8_lossy(content2);

    let old_lines: Vec<&str> = old_str.lines().collect();
    let new_lines: Vec<&str> = new_str.lines().collect();

    let changes = compute_diff(&old_lines, &new_lines);
    let hunks =
        generate_hunks(&old_lines, &new_lines, &changes, hunk_context_lines);

    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
fn format_binary_addition(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary file added\n")
}

    if Blob::is_binary(content) {
        return format_binary_addition(&src_path, &dst_path);
    }

    let new_str = String::from_utf8_lossy(content);
    let new_lines: Vec<&str> = new_str.lines().collect();

    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
    output
        .push_str(&format!("{CYAN}@@ -0,0 +1,{} @@{RESET}\n", new_lines.len()));
fn format_binary_deletion(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary file deleted\n")
}

    if Blob::is_binary(content) {
        return format_binary_deletion(&src_path, &dst_path);
    }

    let old_str = String::from_utf8_lossy(content);
    let old_lines: Vec<&str> = old_str.lines().collect();

    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
    output
        .push_str(&format!("{CYAN}@@ -1,{} +0,0 @@{RESET}\n", old_lines.len()));
    let (mut additions, mut deletions) = changes
        .iter()
        .filter(|x| !matches!(x, Change::Same(..)))
        .fold(
            (0usize, 0usize),
            |(additions, deletions), change| match change {
                Change::Insert(_) => (additions + 1, deletions),
                Change::Delete(_) => (additions, deletions + 1),
                Change::Replace(_, _) => (additions + 1, deletions + 1),
                Change::Same(..) => unreachable!(),
            },
        );
    // +3 for " | "
    let available_columns = STAT_WIDTH - (path.len() + 3);
    let total_changes = additions + deletions;

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    if total_changes > available_columns {
        let p = (additions as f64 / total_changes as f64)
            * available_columns as f64;
        additions = p as usize;
        deletions = available_columns - additions;
        "{path} | {total_changes} {GREEN}{}{RED}{}{RESET}",

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_same_content() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert!(changes
            .iter()
            .all(|change| matches!(change, Change::Same(_, _))));
    }

    #[test]
    fn test_compute_diff_with_deletion() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same(0, 0));
        assert_eq!(changes[1], Change::Delete(1));
        assert_eq!(changes[2], Change::Same(2, 1));
    }

    #[test]
    fn test_compute_diff_with_insertion() {
        let old_lines = ["Line 1", "Line 3"];
        let new_lines = ["Line 1", "Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same(0, 0));
        assert_eq!(changes[1], Change::Insert(1));
        assert_eq!(changes[2], Change::Same(1, 2));
    }

    #[test]
    fn test_compute_diff_with_replacement() {
        let old_lines = ["Line 1", "Old Line 2", "Line 3"];
        let new_lines = ["Line 1", "New Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same(0, 0));
        assert_eq!(changes[1], Change::Replace(1, 1));
        assert_eq!(changes[2], Change::Same(2, 2));
    }

    #[test]
    fn test_generate_hunks_simple_change() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Changed Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        let hunks = generate_hunks(&old_lines, &new_lines, &changes, 3);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 3);
        assert!(hunk.content.contains("-Line 2"));
        assert!(hunk.content.contains("+Changed Line 2"));
    }

    #[test]
    fn test_format_diff_simple_change() {
        let path = "test.txt";
        let content1 = b"Line 1\nLine 2\nLine 3\n";
        let content2 = b"Line 1\nChanged Line 2\nLine 3\n";
        let hunk_context_lines = 3;
        let diff_output = format_diff(
            path,
            content1,
            content2,
            hunk_context_lines,
            "a/",
            "b/",
            false,
        );
        assert!(diff_output.contains("diff --mini-git a/test.txt b/test.txt"));
        assert!(diff_output.contains("--- a/"));
        assert!(diff_output.contains("+++ b/"));
        assert!(diff_output.contains("@@ -1,3 +1,3 @@"));
        assert!(diff_output.contains("-Line 2"));
        assert!(diff_output.contains("+Changed Line 2"));
    }

    #[test]
    fn test_format_binary_diff() {
        let path = "binary_file.bin";
        let output =
            format_binary_diff(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary files differ"));
    }

    #[test]
    fn test_format_binary_addition() {
        let path = "binary_file.bin";
        let output =
            format_binary_addition(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file added"));
    }

    #[test]
    fn test_format_binary_deletion() {
        let path = "binary_file.bin";
        let output =
            format_binary_deletion(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file deleted"));
    }

    #[test]
    fn test_format_addition() {
        let path = "new_file.txt";
        let content = b"New content\nLine 2\n";
        let output = format_addition(path, content, "a/", "b/", false);
        assert!(output.contains("diff --mini-git a/dev/null b/new_file.txt"),);
        assert!(output.contains("new file"));
        assert!(output.contains("+++ b/"));
        assert!(output.contains("+New content"));
        assert!(output.contains("+Line 2"));
    }

    #[test]
    fn test_format_deletion() {
        let path = "old_file.txt";
        let content = b"Old content\nLine 2\n";
        let output = format_deletion(path, content, "a/", "b/", false);
        assert!(output.contains("diff --mini-git a/old_file.txt b/dev/null"),);
        assert!(output.contains("deleted file"));
        assert!(output.contains("--- a/"));
        assert!(output.contains("-Old content"));
        assert!(output.contains("-Line 2"));
    }

    #[test]
    fn test_generate_hunks_with_multiple_changes() {
        let old_lines = ["Line 1", "Line 2", "Line 3", "Line 4"];
        let new_lines = ["Line 1", "Changed Line 2", "Line 3", "New Line 4"];
        let changes = compute_diff(&old_lines, &new_lines);
        let hunks = generate_hunks(&old_lines, &new_lines, &changes, 2);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert!(hunk.content.contains("-Line 2"));
        assert!(hunk.content.contains("+Changed Line 2"));
        assert!(hunk.content.contains("-Line 4"));
        assert!(hunk.content.contains("+New Line 4"));
    }

    #[test]
    fn test_compute_diff_with_empty_old_lines() {
        let old_lines: [&str; 0] = [];
        let new_lines = ["Line 1", "Line 2"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Insert(0));
        assert_eq!(changes[1], Change::Insert(1));
    }

    #[test]
    fn test_compute_diff_with_empty_new_lines() {
        let old_lines = ["Line 1", "Line 2"];
        let new_lines: [&str; 0] = [];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Delete(0));
        assert_eq!(changes[1], Change::Delete(1));
    }

    #[test]
    fn test_format_diff_with_no_changes() {
        let path = "unchanged.txt";
        let content = b"Line 1\nLine 2\n";
        let diff_output =
            format_diff(path, content, content, 3, "a/", "b/", false);
        // Since there are no changes, diff output should be minimal
        assert!(diff_output
            .contains("diff --mini-git a/unchanged.txt b/unchanged.txt"));
        assert!(diff_output.contains("--- a/"));
        assert!(diff_output.contains("+++ b/"));
        // No hunks should be present
        assert!(!diff_output.contains("@@"));
    }
}