use crate::core::objects::{self, blob::Blob, tree::Tree};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";

#[derive(Debug)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    content: String,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum Change {
    Same(usize, usize),    // (old_idx, new_idx)
    Delete(usize),         // old_idx
    Insert(usize),         // new_idx
    Replace(usize, usize), // (old_idx, new_idx)
}

/// List differences
/// This handles the subcommand
///
/// ```bash
/// mini_git diff [--name-only] [--files FILES] [tree1] [tree2]
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn diff(args: &Namespace) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|_| {
        "Could not determine current working directory".to_owned()
    })?;

    let repo_path = path::repo_find(&cwd)?
        .canonicalize()
        .map_err(|_| "Could not determine repository path".to_owned())?;
    let repo = GitRepository::new(&repo_path)?;

    // Parse arguments
    let name_only = args.get("name-only").is_some();
    let Some(hunk_context_lines) = args.get("n-context-lines") else {
        unreachable!()
    };
    let Ok(hunk_context_lines) = hunk_context_lines.parse::<usize>() else {
        unreachable!()
    };

    // Resolve the file paths to be relative to the repository root
    let mut resolved_files: Vec<String> = vec![];
    for file in &args
        .get("files")
        .map(|f| f.split(',').collect::<Vec<_>>())
        .unwrap_or_default()
    {
        // Create a path by joining the current working directory with the file path
        let file_path = cwd.join(file);

        // Canonicalize the path to get the absolute path
        let abs_path = file_path
            .canonicalize()
            .map_err(|_| format!("Could not canonicalize path {}", file))?;

        if !abs_path.exists() {
            {}
            return Err(format!(
                "File {} does not exist in the worktree",
                file
            ));
        }

        // Get the relative path from the repository root to the file
        let rel_path = abs_path.strip_prefix(&repo_path).map_err(|_| {
            format!("Could not get path relative to repo root for {}", file)
        })?;

        // Convert the relative path to a string and store it
        resolved_files.push(rel_path.to_string_lossy().to_string());
    }

    // Create a Vec<&str> from the adjusted file paths
    let files: Vec<&str> = resolved_files.iter().map(|s| s.as_str()).collect();

    let (Some(tree1), Some(tree2)) = (args.get("tree1"), args.get("tree2"))
    else {
        return Err("Invalid tree arguments".to_owned());
    };

    let tree1 = if tree1 == "*" {
        None
    } else {
        Some(tree1.as_str())
    };
    let tree2 = if tree2 == "*" {
        None
    } else {
        Some(tree2.as_str())
    };

    _diff(&repo, tree1, tree2, name_only, &files, hunk_context_lines)
}

fn _diff(
    repo: &GitRepository,
    tree1: Option<&str>,
    tree2: Option<&str>,
    name_only: bool,
    files: &[&str],
    hunk_context_lines: usize,
) -> Result<String, String> {
    // Implementation remains the same until the diff generation part
    let (tree1, tree2) = match (tree1, tree2) {
        (None, None) => {
            // Compare working tree with HEAD
            let head = Tree::get_head_tree_sha(repo)?;
            (head, None)
        }
        (Some(tree), None) => {
            // Compare working tree with specified tree
            let tree_sha = objects::find_object(repo, tree, None, true)?;
            (tree_sha, None)
        }
        (Some(tree1), Some(tree2)) => {
            // Compare two trees
            let tree1_sha = objects::find_object(repo, tree1, None, true)?;
            let tree2_sha = objects::find_object(repo, tree2, None, true)?;
            (tree1_sha, Some(tree2_sha))
        }
        _ => return Err("Invalid tree arguments".to_owned()),
    };

    // Get the tree contents
    let tree1_contents = Tree::get_tree_contents(repo, &tree1)?;
    let tree2_contents = if let Some(tree2) = tree2 {
        Tree::get_tree_contents(repo, &tree2)?
    } else {
        Tree::get_working_tree_contents(repo)?
    };

    // Generate diff
    let mut output = String::new();

    // Filter paths if specified
    let paths_to_check: Vec<&str> = if files.is_empty() {
        let mut all_paths: Vec<&str> = tree1_contents
            .keys()
            .chain(tree2_contents.keys())
            .map(String::as_str)
            .collect();
        all_paths.sort_unstable();
        all_paths.dedup();
        all_paths
    } else {
        files.to_vec()
    };

    for path in paths_to_check {
        let content1 = tree1_contents.get(path);
        let content2 = tree2_contents.get(path);

        match (content1, content2) {
            (Some(c1), Some(c2)) if c1 != c2 => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else if Blob::is_binary(c1) || Blob::is_binary(c2) {
                    output.push_str(&format_binary_diff(path));
                } else {
                    output.push_str(&format_diff(
                        path,
                        c1,
                        c2,
                        hunk_context_lines,
                    ));
                }
            }
            (Some(c), None) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else if Blob::is_binary(c) {
                    output.push_str(&format_binary_deletion(path));
                } else {
                    output.push_str(&format_deletion(path, c));
                }
            }
            (None, Some(c)) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else if Blob::is_binary(c) {
                    output.push_str(&format_binary_addition(path));
                } else {
                    output.push_str(&format_addition(path, c));
                }
            }
            _ => {}
        }
    }

    Ok(output)
}

fn compute_diff(old_lines: &[&str], new_lines: &[&str]) -> Vec<Change> {
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
        }
    }
    for j in 0..=new_len {
        dp[0][j] = j;
        if j > 0 {
            backtrace[0][j] = (0, j - 1);
        }
    }

    // Fill the matrices
    for i in 1..=old_len {
        for j in 1..=new_len {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
                backtrace[i][j] = (i - 1, j - 1);
            } else {
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
            }
        }
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
        } else {
            changes.push(Change::Replace(i - 1, j - 1));
        }

        i = prev_i;
        j = prev_j;
    }

    changes.reverse();
    changes
}

#[allow(clippy::too_many_lines)]
fn generate_hunks(
    old_lines: &[&str],
    new_lines: &[&str],
    changes: &[Change],
    hunk_context_lines: usize,
) -> Vec<Hunk> {
    let mut hunks = Vec::new();
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
                    }
                }
            }
            Change::Delete(old_idx) | Change::Replace(old_idx, _) => {
                // Add context buffer if this is the start of a new hunk
                if last_change_idx.is_none() {
                    for line in &context_buffer {
                        current_hunk.push_str(&format!(" {line}\n"));
                        old_count += 1;
                        new_count += 1;
                    }
                    // Adjust start positions
                    old_start = old_idx + 1 - context_buffer.len();
                    new_start = old_start;
                }

                if let Change::Delete(idx) = change {
                    current_hunk.push_str(&format!(
                        "{RED}-{}{RESET}\n",
                        old_lines[*idx]
                    ));
                    old_count += 1;
                } else if let Change::Replace(old_idx, new_idx) = change {
                    current_hunk.push_str(&format!(
                        "{RED}-{}{RESET}\n",
                        old_lines[*old_idx]
                    ));
                    current_hunk.push_str(&format!(
                        "{GREEN}+{}{RESET}\n",
                        new_lines[*new_idx]
                    ));
                    old_count += 1;
                    new_count += 1;
                }
                last_change_idx = Some(i);
            }
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
        hunks.push(Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            content: current_hunk,
        });
    }

    hunks
}

fn format_diff(
    path: &str,
    content1: &[u8],
    content2: &[u8],
    hunk_context_lines: usize,
) -> String {
    let mut output = String::new();
    output
        .push_str(&format!("{CYAN}diff --mini-git a/{path} b/{path}{RESET}\n"));

    let str1 = String::from_utf8_lossy(content1);
    let str2 = String::from_utf8_lossy(content2);

    let lines1: Vec<&str> = str1.lines().collect();
    let lines2: Vec<&str> = str2.lines().collect();

    output.push_str("--- a/\n");
    output.push_str("+++ b/\n");

    let changes = compute_diff(&lines1, &lines2);
    let hunks = generate_hunks(&lines1, &lines2, &changes, hunk_context_lines);

    for hunk in hunks {
        output.push_str(&format!(
            "{CYAN}@@ -{},{} +{},{} @@{RESET}\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        output.push_str(&hunk.content);
        output.push('\n');
    }

    output
}

fn format_binary_diff(path: &str) -> String {
    format!("diff --mini-git a/{path} b/{path}\nBinary files differ\n")
}

fn format_binary_addition(path: &str) -> String {
    format!("diff --mini-git a/{path} b/{path}\nBinary file added\n")
}

fn format_binary_deletion(path: &str) -> String {
    format!("diff --mini-git a/{path} b/{path}\nBinary file deleted\n")
}

fn format_deletion(path: &str, content: &[u8]) -> String {
    let mut output = String::new();
    output.push_str(&format!("diff --mini-git a/{path} b/{path}\n"));
    output.push_str("deleted file\n");
    output.push_str("--- a/\n");
    output.push_str("+++ /dev/null\n");

    let content_str = String::from_utf8_lossy(content);
    for line in content_str.lines() {
        output.push_str(&format!("{RED}-{line}{RESET}\n"));
    }

    output
}

fn format_addition(path: &str, content: &[u8]) -> String {
    let mut output = String::new();
    output.push_str(&format!("diff --mini-git a/{path} b/{path}\n"));
    output.push_str("new file\n");
    output.push_str("--- /dev/null\n");
    output.push_str("+++ b/\n");

    let content_str = String::from_utf8_lossy(content);
    for line in content_str.lines() {
        output.push_str(&format!("{GREEN}+{line}{RESET}\n"));
    }

    output
}

/// Make parser for the diff command
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new(
        "Show changes between commits, commit and working tree, etc.",
    );

    parser
        .add_argument("name-only", ArgumentType::Boolean)
        .optional()
        .add_help("Show only names of changed files");

    parser
        .add_argument("files", ArgumentType::String)
        .optional()
        .add_help("Comma-separated list of files to diff");

    parser
        .add_argument("n-context-lines", ArgumentType::Integer)
        .short('l')
        .optional()
        .default("5")
        .add_help("Number of context lines around a diff hunk");

    parser
        .add_argument("tree1", ArgumentType::String)
        .required()
        .default("*") // * is not a valid branch name
        .add_help("First tree-ish");

    parser
        .add_argument("tree2", ArgumentType::String)
        .required()
        .default("*") // * is not a valid branch name
        .add_help("Second tree-ish");

    parser
}

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
        let diff_output =
            format_diff(path, content1, content2, hunk_context_lines);
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
        let output = format_binary_diff(path);
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary files differ"));
    }

    #[test]
    fn test_format_binary_addition() {
        let path = "binary_file.bin";
        let output = format_binary_addition(path);
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file added"));
    }

    #[test]
    fn test_format_binary_deletion() {
        let path = "binary_file.bin";
        let output = format_binary_deletion(path);
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file deleted"));
    }

    #[test]
    fn test_format_addition() {
        let path = "new_file.txt";
        let content = b"New content\nLine 2\n";
        let output = format_addition(path, content);
        assert!(
            output.contains("diff --mini-git a/new_file.txt b/new_file.txt")
        );
        assert!(output.contains("new file"));
        assert!(output.contains("+++ b/"));
        assert!(output.contains("+New content"));
    }

    #[test]
    fn test_format_deletion() {
        let path = "old_file.txt";
        let content = b"Old content\nLine 2\n";
        let output = format_deletion(path, content);
        assert!(
            output.contains("diff --mini-git a/old_file.txt b/old_file.txt")
        );
        assert!(output.contains("deleted file"));
        assert!(output.contains("--- a/"));
        assert!(output.contains("-Old content"));
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
        let diff_output = format_diff(path, content, content, 3);
        // Since there are no changes, diff output should be minimal
        assert!(diff_output
            .contains("diff --mini-git a/unchanged.txt b/unchanged.txt"));
        assert!(diff_output.contains("--- a/"));
        assert!(diff_output.contains("+++ b/"));
        // No hunks should be present
        assert!(!diff_output.contains("@@"));
    }
}
