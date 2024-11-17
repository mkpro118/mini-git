pub mod cat_file;
pub mod diff;
pub mod hash_object;
pub mod init;
pub mod log;
pub mod ls_tree;
pub mod rev_parse;
pub mod show_ref;

use std::path::Path;

use crate::core::objects::worktree;
use crate::core::GitRepository;

use crate::utils::path;

#[macro_export]
macro_rules! parse_arg_as_int {
    ($value:expr, $err_msg:literal) => {
        match $value {
            Some(count) if let Ok(x) = count.parse::<usize>() => x,
            _ => return Err(format!("{} is not a number", $err_msg)),
        }
    };
    ($value:expr, $default:expr, $err_msg:literal) => {
        match $value {
            None => $default,
            Some(count) => {
                let Ok(x) = count.parse::<usize>() else {
                    return Err(format!("{} is not a number", $err_msg));
                };
                x
            }
        }
    };
}

/// Resolves files specified on the command line to paths relative to the repository root.
///
/// # Parameters
/// - `repo`: A reference to the `GitRepository`.
/// - `cwd`: The current working directory.
/// - `files`: A comma-separated list of file paths to resolve.
///
/// # Returns
/// - `Ok(Vec<String>)` with paths relative to the repository root.
/// - `Err(String)` if any file cannot be resolved.
///
/// # Errors
/// - Returns an error if:
///   - Any file cannot be canonicalized.
///   - Any specified file does not exist.
///   - A directory could not be processed correctly.
///
/// # Examples
/// ```
/// use mini_git::core::{RepositoryContext, resolve_repository_context};
/// use mini_git::core::commands::resolve_cla_files;
/// let RepositoryContext {cwd, repo, ..} = resolve_repository_context()?;
///
/// let resolved_files = resolve_cla_files(&repo, &cwd, "src/main.rs,src/lib.rs")?;
///
/// # Ok::<(), String>(())
/// ```
pub fn resolve_cla_files(
    repo: &GitRepository,
    cwd: &Path,
    files: &str,
) -> Result<Vec<String>, String> {
    let mut resolved_files = vec![];
    for file in files.split(',') {
        // Create a path by joining the current working directory with the file path
        let file_path = cwd.join(file);

        if !file_path.exists() {
            return Err(format!("path '{file}' is not in the working tree"));
        }

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
            resolved_files.push(path::to_posix_path(rel_path)?);
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

                resolved_files.push(path::to_posix_path(rel_path)?);
            }
        } else {
            return Err(format!("{file} is neither a file nor a directory"));
        }
    }

    Ok(resolved_files)
}
