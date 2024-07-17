use std::fs;
use std::path::{Path, PathBuf};

pub fn repo_path<P>(gitdir: &Path, paths: &[P]) -> PathBuf
where
    P: AsRef<Path>,
{
    paths
        .iter()
        .fold(gitdir.to_path_buf(), |dir, path| dir.join(path))
}

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
