use crate::core::GitRepository;
use std::path::Path;

const DEFAULT_PATH: &str = ".";

pub fn cmd_init(
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<String, String> {
    let path = args.into_iter().next();
    let path = match path {
        Some(ref arg) => arg.as_ref(),
        None => DEFAULT_PATH,
    };

    let repo = GitRepository::create(Path::new(&path))?;
    Ok(format!(
        "initialized empty repository in {:?}\n",
        repo.worktree().as_os_str()
    ))
}
