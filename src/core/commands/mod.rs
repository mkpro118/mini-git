pub mod cat_file;
pub mod diff;
pub mod hash_object;
pub mod init;
pub mod log;
pub mod ls_tree;
pub mod rev_parse;
pub mod show_ref;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::core::objects::{self, tree, worktree};
use crate::core::GitRepository;

#[derive(Debug)]
enum FileSource {
    Blob { path: String, sha: String },
    Worktree { path: String },
}

impl FileSource {
    fn contents(&self, repo: &GitRepository) -> Result<Vec<u8>, String> {
        Ok(match self {
            FileSource::Blob { sha, .. } => {
                match objects::read_object(repo, sha)? {
                    objects::GitObject::Blob(blob) => blob.data,
                    x => {
                        return Err(format!(
                            "Expect object {sha} to be a blob, but was {}",
                            String::from_utf8_lossy(x.format())
                        ))
                    }
                }
            }
            FileSource::Worktree { path } => match fs::read(path) {
                Ok(data) => data,
                Err(e) => {
                    return Err(format!(
                        "Failed to read file {path}! Error: {e}"
                    ))
                }
            },
        })
    }

    fn path(&self) -> String {
        match self {
            FileSource::Blob { path, .. } | FileSource::Worktree { path } => {
                path.clone()
            }
        }
    }
}

// Resolves files passed in on the command line
fn resolve_cla_files(
    repo: &GitRepository,
    cwd: &Path,
    files: &str,
) -> Result<Vec<String>, String> {
    let mut resolved_files = vec![];
    for file in files.split(',') {
        // Create a path by joining the current working directory with the file path
        let file_path = cwd.join(file);

        // Canonicalize the path to get the absolute path
        let abs_path = file_path
            .canonicalize()
            .map_err(|_| format!("Could not canonicalize path {file}"))?;

        if !abs_path.exists() {
            return Err(format!("File {file} does not exist in the worktree"));
        }

        if abs_path.is_file() {
            // Get the relative path from the repository root to the file
            let rel_path =
                abs_path.strip_prefix(repo.worktree()).map_err(|_| {
                    format!(
                        "Could not get path relative to repo root for {file}"
                    )
                })?;

            // Convert the relative path to a string and store it
            resolved_files.push(rel_path.to_string_lossy().to_string());
        } else if abs_path.is_dir() {
            // Get all files under this directory
            let worktree_files =
                worktree::get_worktree_files(repo, Some(&abs_path))?;
            for worktree_file in worktree_files {
                // worktree_file is relative to abs_path, so we need to get the absolute path
                let file_abs_path = abs_path.join(&worktree_file);

                // Get the relative path from the repository root
                let rel_path = file_abs_path.strip_prefix(repo.worktree()).map_err(|_| {
                    format!("Could not get path relative to repo root for {file}")
                })?;

                resolved_files.push(rel_path.to_string_lossy().to_string());
            }
        } else {
            return Err(format!("{file} is neither a file nor a directory"));
        }
    }

    Ok(resolved_files)
}

// Gets file contents from both trees
fn get_file_contents(
    repo: &GitRepository,
    tree1: Option<&str>,
    tree2: Option<&str>,
) -> Result<(Vec<FileSource>, Vec<FileSource>), String> {
    let files1 = get_files(repo, tree1)?;
    let files2 = get_files(repo, tree2)?;
    Ok((files1, files2))
}

fn get_files(
    repo: &GitRepository,
    tree: Option<&str>,
) -> Result<Vec<FileSource>, String> {
    Ok(match tree {
        // Get contents from the specified tree
        Some(treeish) => tree::get_tree_files(repo, treeish)?
            .into_iter()
            .map(|(path, sha)| FileSource::Blob { path, sha })
            .collect(),

        // Get contents from the working directory
        None => worktree::get_worktree_files(repo, None)?
            .into_iter()
            .map(|path| FileSource::Worktree { path })
            .collect(),
    })
}

// Collects all files that need to be processed
fn collect_files_to_process(
    files1: &[FileSource],
    files2: &[FileSource],
    specified_files: &[String],
) -> Vec<String> {
    let mut all_files = HashSet::new();

    if specified_files.is_empty() {
        all_files.extend(files1.iter().map(FileSource::path));
        all_files.extend(files2.iter().map(FileSource::path));
    } else {
        all_files.extend(specified_files.iter().cloned());
    }

    all_files.into_iter().collect()
}
