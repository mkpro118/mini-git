use crate::core::objects::index_file::{GitIndex, GitIndexEntry};
use crate::core::repository::{resolve_repository_context, RepositoryContext};
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path::to_posix_path;

/// List files from the index file
/// This handles the subcommand
///
/// ```bash
/// mini_git ls-files
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
pub fn ls_files(args: &Namespace) -> Result<String, String> {
    let RepositoryContext { repo, cwd, .. } = resolve_repository_context()?;

    let cwd: String = to_posix_path(&cwd)?;
    let top_level: String = to_posix_path(repo.worktree())?;

    // This is the directory prefix relative to the top level of the worktree
    let current_dir_prefix = {
        let mut relative_current_dir =
            cwd.strip_prefix(&top_level).unwrap_or(&cwd).to_owned();
        if relative_current_dir.is_empty() {
            // current dir is the top level
            relative_current_dir
        } else {
            relative_current_dir = relative_current_dir
                .strip_prefix("/")
                .unwrap_or(&relative_current_dir)
                .to_owned();
            if !relative_current_dir.ends_with('/') {
                let with_suffix = format!("{relative_current_dir}/");
                relative_current_dir = with_suffix;
            }

            relative_current_dir
        }
    };

    let full_name = args.get("full-path").is_some();
    let debug = args.get("debug").is_some();
    let name_separator = if args.get("use-null").is_some() {
        "\x00"
    } else {
        "\n"
    };

    let index = GitIndex::read_index(&repo)?;
    Ok(index
        .entries()
        .iter()
        .filter(|entry| entry.name.starts_with(&current_dir_prefix))
        .map(|entry| {
            let mut name = entry.name.clone();
            if !full_name {
                name = name
                    .strip_prefix(&current_dir_prefix)
                    .unwrap_or(&name)
                    .to_owned();
            }
            if !debug {
                return name;
            }
            let debug = debug_format(entry);
            format!("{name}{name_separator}{debug}")
        })
        // .filter(predicate)
        .collect::<Vec<_>>()
        .join(name_separator))
}

fn debug_format(entry: &GitIndexEntry) -> String {
    let ctime = format!("  ctime: {}:{}", entry.ctime.0, entry.ctime.1);
    let mtime = format!("mtime: {}:{}", entry.mtime.0, entry.mtime.1);
    let dev = format!("dev: {}", entry.device_id);
    let ino = format!("ino: {}", entry.inode);
    let uid = format!("uid: {}", entry.uid);
    let gid = format!("gid: {}", entry.gid);
    let size = format!("size: {}", entry.size);
    let flags = format!("flags: {}", entry.flag_stage);
    let max_padding = dev.len().max(uid.len()).max(size.len());
    let ino_pad = " ".repeat(max_padding - dev.len());
    let gid_pad = " ".repeat(max_padding - uid.len());
    let flags_pad = " ".repeat(max_padding - size.len());
    let lines = [
        ctime,
        mtime,
        format!("{dev}{ino_pad} {ino}"),
        format!("{uid}{gid_pad} {gid}"),
        format!("{size}{flags_pad} {flags}"),
    ];
    lines.join("\n  ")
}

/// Make `ls-files` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser =
        ArgumentParser::new("Show information about files in the index");

    parser
        .add_argument("debug", ArgumentType::Boolean)
        .optional()
        .add_help("Show debug information");

    parser
        .add_argument("full-path", ArgumentType::Boolean)
        .optional()
        .add_help("Show full path from top level");

    parser
        .add_argument("use-null", ArgumentType::Boolean)
        .short('z')
        .optional()
        .add_help("Separator is NUL, not newline");

    parser
}
