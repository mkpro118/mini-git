use std::fs;
use std::path::{Path, PathBuf};

/// Joins the given `paths` to the base `gitdir`
/// This function does NOT create any files or directories
///
/// # Example
/// ```
/// use mini_git::utils::path::repo_path;
/// use std::path::Path;
///
/// let base = Path::new(".git");
/// let head_path = repo_path(base, &["refs", "heads"]);
/// assert_eq!(head_path, base.join("refs").join("heads"));
/// ```
pub fn repo_path<P>(gitdir: &Path, paths: &[P]) -> PathBuf
where
    P: AsRef<Path>,
{
    paths
        .iter()
        .fold(gitdir.to_path_buf(), |dir, path| dir.join(path))
}

/// Returns the file after joining `gitdir` with the given `paths`.
/// Optionally, if `create = true`, then it creates any missing directories
/// in the path. It does NOT create the file itself.
///
/// Use [`repo_path`] directly if you are not interested in creating missing
/// directories.
///
/// # Errors
///
/// If an I/O error occurs while creating missing intermediate directories
/// or if the path is invalid (this may be OS dependent).
/// Returns a [`String`] message describing the error.
///
/// # Example
/// ```no_run
/// use mini_git::utils::path::repo_file;
/// use std::path::Path;
///
/// let base = Path::new(".git");
/// let head_path = repo_file(base, &["hooks", "pre-commit"], true)?;
/// assert!(base.join("hooks").exists());
/// assert!(base.join("hooks").is_dir());
/// assert_eq!(head_path, Some(base.join("hooks").join("pre-commit")));
/// # Ok::<(), String>(())
/// ```
pub fn repo_file<P>(
    gitdir: &Path,
    paths: &[P],
    create: bool,
) -> Result<Option<PathBuf>, String>
where
    P: AsRef<Path>,
{
    let Some(_) = repo_dir(gitdir, &paths[..(paths.len() - 1)], create)? else {
        return Ok(None);
    };
    Ok(Some(repo_path(gitdir, paths)))
}

pub fn repo_dir<P>(
    gitdir: &Path,
    paths: &[P],
    create: bool,
) -> Result<Option<PathBuf>, String>
where
    P: AsRef<Path>,
{
    let path = repo_path(gitdir, paths);

    if path.exists() {
        if path.is_dir() {
            Ok(Some(path))
        } else {
            Err(format!("not a directory {:?}", path.as_os_str()))
        }
    } else if create {
        match fs::create_dir_all(&path) {
            Ok(()) => Ok(Some(path)),
            Err(_) => Err("error in making directories".to_string()),
        }
    } else {
        Ok(None)
    }
}
