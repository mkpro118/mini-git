use std::collections::HashMap;
use std::path::Path;

use crate::core::objects::{self, traits::KVLM, GitObject};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";

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
    // Get the two trees to compare
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
                } else {
                    output.push_str(&format_diff(path, c1, c2));
                }
            }
            (Some(c), None) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
                } else {
                    output.push_str(&format_deletion(path, c));
                }
            }
            (None, Some(c)) => {
                if name_only {
                    output.push_str(&format!("{path}\n"));
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

#[allow(clippy::similar_names)]
fn format_diff(path: &str, content1: &[u8], content2: &[u8]) -> String {
    let mut output = String::new();
    output.push_str(&format!("diff --mini-git a/{path} b/{path}\n"));

    let str1 = String::from_utf8_lossy(content1);
    let str2 = String::from_utf8_lossy(content2);

    let lines1: Vec<&str> = str1.lines().collect();
    let lines2: Vec<&str> = str2.lines().collect();

    // Simple diff implementation - you might want to use a proper diff algorithm
    output.push_str("--- a/\n");
    output.push_str("+++ b/\n");

    let mut hunk = String::new();
    let hunk_old_start = 1;
    let hunk_new_start = 1;
    let mut hunk_old_count = 0;
    let mut hunk_new_count = 0;

    for i in 0..lines1.len().max(lines2.len()) {
        let line1 = lines1.get(i);
        let line2 = lines2.get(i);

        match (line1, line2) {
            (Some(l1), Some(l2)) if l1 == l2 => {
                hunk.push_str(&format!(" {l1}\n"));
                hunk_old_count += 1;
                hunk_new_count += 1;
            }
            (Some(l1), None) => {
                hunk.push_str(&format!("{RED}-{l1}{RESET}\n"));
                hunk_old_count += 1;
            }
            (None, Some(l2)) => {
                hunk.push_str(&format!("{GREEN}+{l2}{RESET}\n"));
                hunk_new_count += 1;
            }
            (Some(l1), Some(l2)) => {
                hunk.push_str(&format!("{RED}-{l1}{RESET}\n"));
                hunk.push_str(&format!("{GREEN}+{l2}{RESET}\n"));
                hunk_old_count += 1;
                hunk_new_count += 1;
            }
            (None, None) => break,
        }
    }

    output.push_str(&format!(
        "@@ -{hunk_old_start},{hunk_old_count} +{hunk_new_start},{hunk_new_count} @@\n"
    ));
    output.push_str(&hunk);

    output
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
