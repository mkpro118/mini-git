use std::path::Path;

use crate::core::GitRepository;

/// Retrieves a list of all file paths in the worktree of a given Git repository,
/// optionally starting from a specified subdirectory.
///
/// This function starts from either the repository's worktree root or the specified
/// `top` path and recursively collects all file paths within the directory structure,
/// excluding any `.git` directory. It returns the file paths relative to the `top`
/// directory or to the worktree root if no `top` is specified.
///
/// # Arguments
///
/// * `repo` - A reference to the [`GitRepository`] from which to retrieve the worktree paths.
/// * `top` - An optional path within the worktree to start collecting files from.
///           If `None`, it defaults to the worktree root.
///
/// # Returns
///
/// A `Result` with:
/// * `Ok(Vec<String>)` - A list of file paths relative to the specified `top` or worktree root.
/// * `Err(String)` - An error message if the function fails to read directories or retrieve paths.
///
/// # Errors
///
/// Returns an error if:
/// * There is an issue reading from the file system, such as lacking permissions
///   or an invalid directory path.
/// * There is an issue stripping the prefix to make paths relative.
///
/// # Examples
///
/// ```no_run
/// use mini_git::core::GitRepository;
/// use mini_git::core::objects::worktree::get_worktree_files;
/// let repo = GitRepository::new(Path::new("path/to/repo")).unwrap();
/// use std::path::Path;
/// // Retrieve all files in the worktree relative to the root.
/// let files = get_worktree_files(&repo, None).unwrap();
///
/// // Retrieve all files in the worktree relative to a specified directory.
/// let files = get_worktree_files(&repo, Some(Path::new("src"))).unwrap();
/// ```
pub fn get_worktree_files(
    repo: &GitRepository,
    top: Option<&Path>,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let work_tree = repo.worktree();
    let base = top.unwrap_or(work_tree);
    collect_worktree_files(base, base, &mut paths)?;
    Ok(paths)
}

fn collect_worktree_files(
    base: &Path,
    current: &Path,
    paths: &mut Vec<String>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path().canonicalize().map_err(|e| {
            format!("Failed to resolve path {:?} {e}", entry.path())
        })?;

        if path.is_dir()
            && path
                .strip_prefix(base)
                .map_err(|e| format!("Failed to get relative path: {e}"))?
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
            paths.push(relative.to_string_lossy().to_string());
        } else if path.is_dir() {
            collect_worktree_files(base, &path, paths)?;
        }
    }
    Ok(())
}
