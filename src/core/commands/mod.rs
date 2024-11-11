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
use std::path::{Path, PathBuf};

use crate::core::objects::{self, tree, worktree};
use crate::core::GitRepository;
use crate::utils::path;

/// Represents the source of a file, either from a Git blob or the working tree.
#[derive(Debug)]
enum FileSource {
    /// A file stored in a Git blob, with a specific path and SHA identifier.
    Blob { path: String, sha: String },

    /// A file located in the working tree with a specified path.
    Worktree { path: String },
}

/// Holds the context of a Git repository, including the current working directory,
/// repository path, and a reference to the Git repository.
#[derive(Debug)]
struct RepositoryContext {
    /// The current working directory, resolved when `resolve_repository_context` is called.
    cwd: PathBuf,

    /// The absolute path to the root of the repository's worktree.
    repo_path: PathBuf,

    /// The `GitRepository` representing the current repository.
    repo: GitRepository,
}

impl FileSource {
    /// Retrieves the contents of the file source.
    ///
    /// # Arguments
    ///
    /// * `repo` - Reference to the Git repository.
    ///
    /// # Returns
    ///
    /// A `Result` containing the file contents as a vector of bytes if successful,
    /// or an error message if reading the contents failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///   - For `Blob` sources, the object is not a blob.
    ///   - For `Worktree` sources, the file could not be read from the filesystem.
    ///
    /// # Examples
    ///
    /// ```
    /// let file_source = FileSource::Blob { path: "file.txt".to_string(), sha: "abc123".to_string() };
    /// let contents = file_source.contents(&repo)?;
    /// ```
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

    /// Returns the path of the file, either from a Git blob or working tree.
    ///
    /// # Returns
    ///
    /// A `String` representing the path to the file.
    ///
    /// # Examples
    ///
    /// ```
    /// let file_source = FileSource::Worktree { path: "file.txt".to_string() };
    /// let path = file_source.path();
    /// ```
    fn path(&self) -> String {
        match self {
            FileSource::Blob { path, .. } | FileSource::Worktree { path } => {
                path.clone()
            }
        }
    }
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
/// let resolved_files = resolve_cla_files(&repo, &cwd, "src/main.rs,src/lib.rs")?;
/// ```
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

/// Retrieves files from a specified tree or the working directory if no tree is specified.
///
/// # Parameters
/// - `repo`: A reference to the `GitRepository`.
/// - `tree`: An optional reference to a tree identifier.
///
/// # Returns
/// - `Ok(Vec<FileSource>)` containing files from the specified tree or working directory.
/// - `Err(String)` if an error occurs while retrieving files.
///
/// # Errors
/// - Returns an error if:
///   - Files cannot be read from the specified tree.
///   - The working directory cannot be accessed.
///
/// # Examples
/// ```
/// let files = get_files(&repo, Some("main"))?;
/// ```
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

/// Collects all files that need to be processed, based on user-specified files or default paths.
///
/// # Parameters
/// - `files1`: A slice of `FileSource` representing files from the first tree.
/// - `files2`: A slice of `FileSource` representing files from the second tree.
/// - `specified_files`: A slice of `String` with specific files to process, if any.
///
/// # Returns
/// A `Vec<String>` containing paths to all files that need processing.
///
/// # Examples
/// ```
/// let files_to_process = collect_files_to_process(&files1, &files2, &["src/lib.rs".to_string()]);
/// ```
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

/// Resolves the repository context, including the current working directory, repository path,
/// and repository object.
///
/// # Returns
/// - `Ok(RepositoryContext)` containing the current working directory, repository path, and Git repository object.
/// - `Err(String)` if the repository context cannot be determined.
///
/// # Errors
/// - Returns an error if:
///   - The current working directory cannot be determined.
///   - The repository path cannot be determined.
///   - The Git repository object cannot be initialized.
///
/// # Examples
/// ```
/// let repo_context = resolve_repository_context()?;
/// ```
fn resolve_repository_context() -> Result<RepositoryContext, String> {
    let cwd = std::env::current_dir().map_err(|_| {
        "Could not determine current working directory".to_owned()
    })?;

    let repo_path = path::repo_find(&cwd)?
        .canonicalize()
        .map_err(|_| "Could not determine repository path".to_owned())?;
    let repo = GitRepository::new(&repo_path)?;

    Ok(RepositoryContext {
        cwd,
        repo_path,
        repo,
    })
}
