use crate::core::GitRepository;
use std::path::Path;

const DEFAULT_PATH: &str = ".";

fn cmd_init(
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<(), String> {
    let path = args.into_iter().next();
    let path = match path {
        Some(ref arg) => arg.as_ref(),
        None => DEFAULT_PATH,
    };

    let _ = GitRepository::create(&Path::new(&path))?;
    Ok(())
}
