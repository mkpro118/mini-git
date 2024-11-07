use std::collections::{HashMap, HashSet};

use crate::core::objects::{self, tree::Tree};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";

#[allow(clippy::struct_excessive_bools)]
struct DiffOpts<'a> {
    files: Vec<&'a str>,
    name_only: bool,
    name_status: bool,
    stat: bool,
    diff_filter: Option<&'a str>,
    hunk_context_lines: usize,
    src_prefix: &'a str,
    dst_prefix: &'a str,
    no_prefix: bool,
}

// Data structures for diff computation
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
    Same(usize),           // old_idx
    Delete(usize),         // old_idx
    Insert(usize),         // new_idx
    Replace(usize, usize), // (old_idx, new_idx)
}

/// List differences
/// This handles the subcommand
///
/// ```bash
/// mini_git diff [options] [ --tree1 TREE1 ] [ --tree2 TREE2 ] [ --files FILE1,FILE2,... ]
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
    let name_status = args.get("name-status").is_some();
    let stat = args.get("stat").is_some();
    let diff_filter = args.get("diff-filter").map(std::string::String::as_str);
    let hunk_context_lines = &args["n-context-lines"];
    let src_prefix = &args["src-prefix"];
    let dst_prefix = &args["dst-prefix"];
    let no_prefix = args.get("no-prefix").is_some();

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
            .map_err(|_| format!("Could not canonicalize path {file}"))?;

        if !abs_path.exists() {
            return Err(format!("File {file} does not exist in the worktree"));
        }

        // Get the relative path from the repository root to the file
        let rel_path = abs_path.strip_prefix(&repo_path).map_err(|_| {
            format!("Could not get path relative to repo root for {file}")
        })?;

        // Convert the relative path to a string and store it
        resolved_files.push(rel_path.to_string_lossy().to_string());
    }

    // Create a Vec<&str> from the adjusted file paths
    let files: Vec<&str> = resolved_files.iter().map(String::as_str).collect();

    let opts = DiffOpts {
        files,
        name_only,
        name_status,
        stat,
        diff_filter,
        hunk_context_lines,
        src_prefix,
        dst_prefix,
        no_prefix,
    };

    // Parse tree1 and tree2
    let tree1 = args
        .get("tree1")
        .filter(|s| *s != "*")
        .map(std::string::String::as_str);
    let tree2 = args
        .get("tree2")
        .filter(|s| *s != "*")
        .map(std::string::String::as_str);

    _diff(&repo, tree1, tree2, &opts)
}

fn _diff(
    repo: &GitRepository,
    tree1: Option<&str>,
    tree2: Option<&str>,
    opts: &DiffOpts,
) -> Result<String, String> {
    let (tree1, tree2) = match (tree1, tree2) {
        (None, None) => {
            // Compare working tree with HEAD
            let head = Tree::get_head_tree_sha(repo)?;
            (Some(head), None)
        }
        (Some(tree), None) => {
            // Compare working tree with specified tree
            let tree_sha = objects::find_object(repo, tree, None, true)?;
            (Some(tree_sha), None)
        }
        (Some(tree1), Some(tree2)) => {
            // Compare two trees
            let tree1_sha = objects::find_object(repo, tree1, None, true)?;
            let tree2_sha = objects::find_object(repo, tree2, None, true)?;
            (Some(tree1_sha), Some(tree2_sha))
        }
        _ => return Err("Invalid tree arguments".to_owned()),
    };

    // Get the files from tree1 and tree2
    let files1 = get_files(repo, tree1.as_deref())?;
    let files2 = get_files(repo, tree2.as_deref())?;

    // Build the set of all files to consider
    let mut all_files = HashSet::new();

    if opts.files.is_empty() {
        all_files.extend(files1.keys().cloned());
        all_files.extend(files2.keys().cloned());
    } else {
        all_files.extend(
            opts.files
                .iter()
                .copied()
                .map(std::string::ToString::to_string),
        );
    }

    let mut results = Vec::new();

    for file in all_files {
        // If files are specified, only consider those files
        if !opts.files.is_empty() && !opts.files.contains(&file.as_str()) {
            continue;
        }

        let content1 = files1.get(&file);
        let content2 = files2.get(&file);

        let status = match (content1, content2) {
            (Some(_), None) => 'D', // Deleted
            (None, Some(_)) => 'A', // Added
            (Some(c1), Some(c2)) => {
                if c1 == c2 {
                    continue; // No change
                }
                'M' // Modified
            }
            (None, None) => continue, // Should not happen
        };

        // Apply diff_filter
        if let Some(filter) = opts.diff_filter {
            if !status_matches_filter(status, filter) {
                continue;
            }
        }

        // Now, depending on options, generate output
        if opts.name_only {
            results.push(file.to_string());
        } else if opts.name_status {
            results.push(format!("{status}\t{file}"));
        } else if opts.stat {
            // Generate diffstat output
            let stat_output = format_diffstat(
                &file,
                content1.unwrap_or(&vec![]),
                content2.unwrap_or(&vec![]),
            );
            results.push(stat_output);
        } else {
            // Generate full diff
            let diff_output = match status {
                'A' => format_addition(
                    &file,
                    content2.unwrap(),
                    opts.src_prefix,
                    opts.dst_prefix,
                    opts.no_prefix,
                ),
                'D' => format_deletion(
                    &file,
                    content1.unwrap(),
                    opts.src_prefix,
                    opts.dst_prefix,
                    opts.no_prefix,
                ),
                'M' => format_diff(
                    &file,
                    content1.unwrap(),
                    content2.unwrap(),
                    opts.hunk_context_lines,
                    opts.src_prefix,
                    opts.dst_prefix,
                    opts.no_prefix,
                ),
                _ => String::new(),
            };
            results.push(diff_output);
        }
    }

    Ok(results.join("\n"))
}

fn status_matches_filter(status: char, filter: &str) -> bool {
    // Uppercase letters include, lowercase letters exclude
    let mut include = HashSet::new();
    let mut exclude = HashSet::new();
    for c in filter.chars() {
        if c.is_uppercase() {
            include.insert(c);
        } else if c.is_lowercase() {
            exclude.insert(c.to_ascii_uppercase());
        }
    }
    if !include.is_empty() && !include.contains(&status) {
        return false;
    }
    if exclude.contains(&status) {
        return false;
    }
    true
}

fn get_files(
    repo: &GitRepository,
    tree: Option<&str>,
) -> Result<HashMap<String, Vec<u8>>, String> {
    match tree {
        Some(treeish) => {
            // Resolve the tree-ish to a tree SHA
            let tree_sha =
                objects::find_object(repo, treeish, Some("tree"), true)?;
            Tree::get_tree_contents(repo, &tree_sha)
        }
        None => {
            // Get contents from the working directory
            Tree::get_working_tree_contents(repo)
        }
    }
}

fn compute_diff(old_lines: &[&str], new_lines: &[&str]) -> Vec<Change> {
    let mut changes = Vec::new();
    let lcs = longest_common_subsequence(old_lines, new_lines);

    let mut old_index = 0;
    let mut new_index = 0;

    for &(i, j) in &lcs {
        while old_index < i && new_index < j {
            // Lines are different; treat as replacement
            changes.push(Change::Replace(old_index, new_index));
            old_index += 1;
            new_index += 1;
        }

        while old_index < i {
            changes.push(Change::Delete(old_index));
            old_index += 1;
        }
        while new_index < j {
            changes.push(Change::Insert(new_index));
            new_index += 1;
        }

        // Lines are the same
        changes.push(Change::Same(old_index));
        old_index += 1;
        new_index += 1;
    }

    // Handle remaining lines
    while old_index < old_lines.len() && new_index < new_lines.len() {
        changes.push(Change::Replace(old_index, new_index));
        old_index += 1;
        new_index += 1;
    }
    while old_index < old_lines.len() {
        changes.push(Change::Delete(old_index));
        old_index += 1;
    }
    while new_index < new_lines.len() {
        changes.push(Change::Insert(new_index));
        new_index += 1;
    }

    changes
}

fn longest_common_subsequence<'a>(
    old_lines: &[&'a str],
    new_lines: &[&'a str],
) -> Vec<(usize, usize)> {
    let m = old_lines.len();
    let n = new_lines.len();
    let mut lcs_lengths = vec![vec![0; n + 1]; m + 1];

    for (i, &old_line) in old_lines.iter().enumerate() {
        for (j, &new_line) in new_lines.iter().enumerate() {
            if old_line == new_line {
                lcs_lengths[i + 1][j + 1] = lcs_lengths[i][j] + 1;
            } else {
                lcs_lengths[i + 1][j + 1] =
                    std::cmp::max(lcs_lengths[i + 1][j], lcs_lengths[i][j + 1]);
            }
        }
    }

    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if old_lines[i - 1] == new_lines[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if lcs_lengths[i - 1][j] >= lcs_lengths[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

fn generate_hunks(
    old_lines: &[&str],
    new_lines: &[&str],
    changes: &[Change],
    hunk_context_lines: usize,
) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut i = 0;
    while i < changes.len() {
        // Skip unchanged lines
        while i < changes.len() {
            match &changes[i] {
                Change::Same(_) => i += 1,
                _ => break,
            }
        }
        if i >= changes.len() {
            break;
        }

        let hunk_start = if i >= hunk_context_lines {
            i - hunk_context_lines
        } else {
            0
        };

        let mut hunk_end = i;
        let mut context_after = 0;

        while hunk_end < changes.len() {
            match &changes[hunk_end] {
                Change::Same(_) => {
                    context_after += 1;
                    if context_after >= hunk_context_lines {
                        hunk_end += 1; // Include enough context lines
                        break;
                    }
                }
                _ => {
                    context_after = 0;
                }
            }
            hunk_end += 1;
        }

        // Calculate hunk header positions
        let mut old_start = 0;
        let mut new_start = 0;
        let mut old_count = 0;
        let mut new_count = 0;
        let mut hunk_content = String::new();

        for change in changes.iter().take(hunk_end).skip(hunk_start) {
            match change {
                Change::Same(old_idx) => {
                    if old_start == 0 {
                        old_start = old_idx + 1;
                        new_start = old_idx + 1;
                    }
                    old_count += 1;
                    new_count += 1;
                    hunk_content
                        .push_str(&format!(" {}\n", old_lines[*old_idx]));
                }
                Change::Delete(old_idx) => {
                    if old_start == 0 {
                        old_start = old_idx + 1;
                        new_start = old_idx + 1;
                    }
                    old_count += 1;
                    hunk_content.push_str(&format!(
                        "{}-{}\n",
                        RED, old_lines[*old_idx]
                    ));
                }
                Change::Insert(new_idx) => {
                    if old_start == 0 {
                        old_start = new_idx + 1;
                        new_start = new_idx + 1;
                    }
                    new_count += 1;
                    hunk_content.push_str(&format!(
                        "{}+{}\n",
                        GREEN, new_lines[*new_idx]
                    ));
                }
                Change::Replace(old_idx, new_idx) => {
                    if old_start == 0 {
                        old_start = old_idx + 1;
                        new_start = new_idx + 1;
                    }
                    old_count += 1;
                    new_count += 1;
                    hunk_content.push_str(&format!(
                        "{}-{}\n",
                        RED, old_lines[*old_idx]
                    ));
                    hunk_content.push_str(&format!(
                        "{}+{}\n",
                        GREEN, new_lines[*new_idx]
                    ));
                }
            }
        }

        hunk_content.push_str(RESET);

        hunks.push(Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            content: hunk_content,
        });

        i = hunk_end;
    }

    hunks
}

fn format_diff(
    path: &str,
    content1: &[u8],
    content2: &[u8],
    hunk_context_lines: usize,
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let old_str = String::from_utf8_lossy(content1);
    let new_str = String::from_utf8_lossy(content2);

    let old_lines: Vec<&str> = old_str.lines().collect();
    let new_lines: Vec<&str> = new_str.lines().collect();

    let changes = compute_diff(&old_lines, &new_lines);
    let hunks =
        generate_hunks(&old_lines, &new_lines, &changes, hunk_context_lines);

    let src_path = if no_prefix {
        path.to_string()
    } else {
        format!("{src_prefix}{path}")
    };
    let dst_path = if no_prefix {
        path.to_string()
    } else {
        format!("{dst_prefix}{path}")
    };

    let mut output = String::new();
    output.push_str(&format!("diff --mini-git {src_path} {dst_path}\n"));
    output.push_str("index ....\n"); // Simplified index line
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    for hunk in hunks {
        output.push_str(&format!(
            "{CYAN}@@ -{},{} +{},{} @@{RESET}\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        output.push_str(&hunk.content);
    }

    output.push_str(RESET);

    output
}

fn format_addition(
    path: &str,
    content: &[u8],
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let new_str = String::from_utf8_lossy(content);
    let new_lines: Vec<&str> = new_str.lines().collect();

    let src_path = if no_prefix {
        "/dev/null".to_string()
    } else {
        format!(
            "{}{}",
            src_prefix,
            if src_prefix.ends_with('/') {
                "dev/null"
            } else {
                "/dev/null"
            }
        )
    };
    let dst_path = if no_prefix {
        path.to_string()
    } else {
        format!("{dst_prefix}{path}")
    };

    let mut output = String::new();
    output.push_str(&format!("diff --mini-git {src_path} {dst_path}\n"));
    output.push_str("new file mode 100644\n");
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    output.push_str(&format!("@@ -0,0 +1,{} @@\n", new_lines.len()));
    for line in new_lines {
        output.push_str(&format!("{GREEN}+{line}\n"));
    }

    output.push_str(RESET);

    output
}

fn format_deletion(
    path: &str,
    content: &[u8],
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let old_str = String::from_utf8_lossy(content);
    let old_lines: Vec<&str> = old_str.lines().collect();

    let src_path = if no_prefix {
        path.to_string()
    } else {
        format!("{src_prefix}{path}")
    };
    let dst_path = if no_prefix {
        "/dev/null".to_string()
    } else {
        format!(
            "{}{}",
            dst_prefix,
            if dst_prefix.ends_with('/') {
                "dev/null"
            } else {
                "/dev/null"
            }
        )
    };

    let mut output = String::new();
    output.push_str(&format!("diff --mini-git {src_path} {dst_path}\n"));
    output.push_str("deleted file mode 100644\n");
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    output.push_str(&format!("@@ -1,{} +0,0 @@\n", old_lines.len()));
    for line in old_lines {
        output.push_str(&format!("{RED}-{line}\n"));
    }

    output.push_str(RESET);

    output
}

fn format_diffstat(path: &str, content1: &[u8], content2: &[u8]) -> String {
    // Generate a simple diffstat output
    let old_lines = String::from_utf8_lossy(content1);
    let old_lines: Vec<&str> = old_lines.lines().collect();
    let new_lines = String::from_utf8_lossy(content2);
    let new_lines: Vec<&str> = new_lines.lines().collect();

    let changes = compute_diff(&old_lines, &new_lines);

    let mut additions = 0;
    let mut deletions = 0;

    for change in changes {
        match change {
            Change::Insert(_) => additions += 1,
            Change::Delete(_) => deletions += 1,
            Change::Replace(_, _) => {
                deletions += 1;
                additions += 1;
            }
            Change::Same(_) => {}
        }
    }

    format!(
        "{:<40} | {} {}\n",
        path,
        "+".repeat(additions),
        "-".repeat(deletions)
    )
}

// DO NOT CHANGE THIS FUNCTION
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
        .add_argument("name-status", ArgumentType::Boolean)
        .optional()
        .add_help("Show only the name(s) and status of each changed file.");

    parser
        .add_argument("stat", ArgumentType::Boolean)
        .optional()
        .add_help("Generate a diffstat, instead of patch, using 80 columns.");

    parser
        .add_argument("diff-filter", ArgumentType::String)
        .optional()
        .add_help("Select only files that are Added (A), Deleted (D), Modified (M) or Renamed (R). Also, these upper-case letters can be downcased to exclude");

    parser
        .add_argument("files", ArgumentType::String)
        .optional()
        .add_help("Comma-separated list of files to diff");

    parser
        .add_argument("n-context-lines", ArgumentType::Integer)
        .short('l')
        .optional()
        .default("3")
        .add_help("Number of context lines around a diff hunk");

    parser
        .add_argument("src-prefix", ArgumentType::String)
        .optional()
        .default("a/")
        .add_help("Show the given source prefix instead of \"a/\"");

    parser
        .add_argument("dst-prefix", ArgumentType::String)
        .optional()
        .default("b/")
        .add_help("Show the given destination prefix instead of \"b/\"");

    parser
        .add_argument("no-prefix", ArgumentType::Boolean)
        .optional()
        .add_help("Do not show any source or destination prefix");

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
