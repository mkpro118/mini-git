use std::collections::HashMap;
use std::path::Path;

use crate::core::objects::{self, traits::KVLM, GitObject};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const HUNK_CONTEXT_LINES: usize = 5;
const BINARY_CHECK_BYTES: usize = 8000;

#[derive(Debug)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    content: String,
}

#[derive(Debug)]
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

    let repo = path::repo_find(cwd)?;
    let repo = GitRepository::new(&repo)?;

    // Parse arguments
    let name_only = args.get("name-only").is_some();
    let files: Vec<&str> = args
        .get("files")
        .map(|f| f.split(',').collect())
        .unwrap_or_default();

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

    _diff(&repo, tree1, tree2, name_only, &files)
}

fn _diff(
    repo: &GitRepository,
    tree1: Option<&str>,
    tree2: Option<&str>,
    name_only: bool,
    files: &[&str],
) -> Result<String, String> {
    // Implementation remains the same until the diff generation part
    let (tree1, tree2) = match (tree1, tree2) {
        (None, None) => {
            // Compare working tree with HEAD
            let head = get_head_tree(repo)?;
            (head, None)
        }
        (Some(tree), None) => {
            // Compare working tree with specified tree
            let tree_sha =
                objects::find_object(repo, tree, Some("tree"), true)?;
            (tree_sha, None)
        }
        (Some(tree1), Some(tree2)) => {
            // Compare two trees
            let tree1_sha =
                objects::find_object(repo, tree1, Some("tree"), true)?;
            let tree2_sha =
                objects::find_object(repo, tree2, Some("tree"), true)?;
            (tree1_sha, Some(tree2_sha))
        }
        _ => return Err("Invalid tree arguments".to_owned()),
    };

    // Get the tree contents
    let tree1_contents = get_tree_contents(repo, &tree1)?;
    let tree2_contents = if let Some(tree2) = tree2 {
        get_tree_contents(repo, &tree2)?
    } else {
        get_working_tree_contents(repo)?
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
                } else if is_binary(c1) || is_binary(c2) {
                    output.push_str(&format_binary_diff(path));
                } else {
                    output.push_str(&format_diff(path, c1, c2));
                }
            }
            (Some(c), None) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else if is_binary(c) {
                    output.push_str(&format_binary_deletion(path));
                } else {
                    output.push_str(&format_deletion(path, c));
                }
            }
            (None, Some(c)) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else if is_binary(c) {
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

fn get_head_tree(repo: &GitRepository) -> Result<String, String> {
    let head_ref = objects::find_object(repo, "HEAD", Some("commit"), true)?;
    let head_obj = objects::read_object(repo, &head_ref)?;

    if let GitObject::Commit(commit) = head_obj {
        commit
            .kvlm()
            .get_key(b"tree")
            .and_then(|t| t.first())
            .map(|t| String::from_utf8_lossy(t).to_string())
            .ok_or_else(|| "HEAD commit has no tree".to_owned())
    } else {
        Err("HEAD is not a commit".to_owned())
    }
}

fn get_tree_contents(
    repo: &GitRepository,
    tree_sha: &str,
) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut contents = HashMap::new();
    collect_tree_contents(repo, tree_sha, "", &mut contents)?;
    Ok(contents)
}

fn collect_tree_contents(
    repo: &GitRepository,
    tree_sha: &str,
    prefix: &str,
    contents: &mut HashMap<String, Vec<u8>>,
) -> Result<(), String> {
    let tree_obj = objects::read_object(repo, tree_sha)?;

    if let GitObject::Tree(tree) = tree_obj {
        for leaf in tree.leaves() {
            let path = if prefix.is_empty() {
                leaf.path_as_string()
            } else {
                format!("{}/{}", prefix, leaf.path_as_string())
            };

            match leaf.obj_type() {
                Some("blob") => {
                    let blob_obj = objects::read_object(repo, leaf.sha())?;
                    if let GitObject::Blob(blob) = blob_obj {
                        contents.insert(path, blob.data);
                    }
                }
                Some("tree") => {
                    collect_tree_contents(repo, leaf.sha(), &path, contents)?;
                }
                _ => return Err(format!("Unknown object type for {path}")),
            }
        }
    }

    Ok(())
}

fn get_working_tree_contents(
    repo: &GitRepository,
) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut contents = HashMap::new();
    let repo_path = repo.gitdir();
    let work_tree = repo_path.parent().ok_or("Invalid repo path")?;

    // This is a simplified version - you might want to add proper .gitignore handling
    collect_working_tree_contents(work_tree, work_tree, &mut contents)?;

    Ok(contents)
}

fn collect_working_tree_contents(
    base: &Path,
    current: &Path,
    contents: &mut HashMap<String, Vec<u8>>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path();

        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == ".git")
        {
            continue;
        }

        if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .map_err(|_| "Failed to get relative path".to_owned())?;
            let content = std::fs::read(&path).map_err(|e| {
                format!("Failed to read file {}: {}", path.display(), e)
            })?;
            contents.insert(relative.to_string_lossy().to_string(), content);
        } else if path.is_dir() {
            collect_working_tree_contents(base, &path, contents)?;
        }
    }

    Ok(())
}

#[allow(clippy::naive_bytecount)]
fn is_binary(content: &[u8]) -> bool {
    let check_len = content.len().min(BINARY_CHECK_BYTES);
    let null_count = content[..check_len]
        .iter()
        .filter(|&&byte| byte == 0)
        .count();

    // If more than 20% of the checked bytes are null, consider it binary
    null_count > check_len / 5
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

fn generate_hunks(
    old_lines: &[&str],
    new_lines: &[&str],
    changes: &[Change],
) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current_hunk = String::new();
    let mut old_start = 1;
    let mut new_start = 1;
    let mut old_count = 0;
    let mut new_count = 0;
    let mut last_change_idx = None;

    for (i, change) in changes.iter().enumerate() {
        match change {
            Change::Same(old_idx, new_idx) => {
                // If we're within HUNK_CONTEXT_LINES of a change, include the line
                if let Some(last_idx) = last_change_idx {
                    if i - last_idx <= HUNK_CONTEXT_LINES {
                        current_hunk
                            .push_str(&format!(" {}\n", old_lines[*old_idx]));
                        old_count += 1;
                        new_count += 1;
                    } else {
                        // Start a new hunk if needed
                        if !current_hunk.is_empty() {
                            hunks.push(Hunk {
                                old_start,
                                old_count,
                                new_start,
                                new_count,
                                content: current_hunk,
                            });
                            current_hunk = String::new();
                        }
                        old_start = old_idx + 1;
                        new_start = new_idx + 1;
                        old_count = 0;
                        new_count = 0;
                    }
                }
            }
            Change::Delete(old_idx) => {
                current_hunk.push_str(&format!(
                    "{RED}-{}{RESET}\n",
                    old_lines[*old_idx]
                ));
                old_count += 1;
                last_change_idx = Some(i);
            }
            Change::Insert(new_idx) => {
                current_hunk.push_str(&format!(
                    "{GREEN}+{}{RESET}\n",
                    new_lines[*new_idx]
                ));
                new_count += 1;
                last_change_idx = Some(i);
            }
            Change::Replace(old_idx, new_idx) => {
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

fn format_diff(path: &str, content1: &[u8], content2: &[u8]) -> String {
    let mut output = String::new();
    output.push_str(&format!("diff --mini-git a/{path} b/{path}\n"));

    let str1 = String::from_utf8_lossy(content1);
    let str2 = String::from_utf8_lossy(content2);

    let lines1: Vec<&str> = str1.lines().collect();
    let lines2: Vec<&str> = str2.lines().collect();

    output.push_str("--- a/\n");
    output.push_str("+++ b/\n");

    let changes = compute_diff(&lines1, &lines2);
    let hunks = generate_hunks(&lines1, &lines2, &changes);

    for hunk in hunks {
        output.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        output.push_str(&hunk.content);
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
