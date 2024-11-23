use std::path::Path;

use crate::core::repository::{resolve_repository_context, RepositoryContext};
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

use crate::core::objects::ignore::GitIgnore;

const COMMA: char = ',';

/// Check whether the file is excluded by .gitignore
/// This handles the subcommand
///
/// ```bash
/// mini_git check-ignore [options] PATHS
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn check_ignore(args: &Namespace) -> Result<String, String> {
    let RepositoryContext { repo_path, .. } = resolve_repository_context()?;

    let quiet: bool = args.get("quiet").is_some();
    let verbose: bool = args.get("verbose").is_some();
    let Some(files) = args.get("paths") else {
        unreachable!("Should be validated by argparse");
    };
    let paths: Vec<&str> = files.split(COMMA).collect();

    let gitignore = GitIgnore::new();

    let mut output = String::new();
    let mut any_ignored = false;

    for path_str in paths {
        let path = Path::new(&path_str);

        if let Some((file, line_no, line)) = gitignore.is_ignored(path) {
            any_ignored = true;

            if !quiet {
                if verbose {
                    // Output in verbose format
                    output.push_str(&format!(
                        "{}:{line_no}:{line}\t\"{}\"\n",
                        Path::new(&file)
                            .strip_prefix(&repo_path)
                            .map_err(|e| format!(
                                "'{path:?}' is not in the worktree. Error: {e}"
                            ))?
                            .display(),
                        path.display()
                    ));
                } else {
                    // Output just the path
                    output.push_str(&format!("{}\n", path.display()));
                }
            }
        }
    }

    if quiet {
        if any_ignored {
            // If any path is ignored, exit code should be 0 (success)
            Ok(String::new())
        } else {
            // If none are ignored, exit code should be 1 (failure)
            Err("No paths ignored".to_string())
        }
    } else {
        Ok(output)
    }
}

/// Make `check-ignore` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser =
        ArgumentParser::new("Check whether the file is excluded by .gitignore");
    parser
        .add_argument("quiet", ArgumentType::Boolean)
        .optional()
        .short('q')
        .add_help("Do not output anything, just set exit code");

    parser
        .add_argument("verbose", ArgumentType::Boolean)
        .optional()
        .short('v')
        .add_help("Display output in verbose format");

    parser
        .add_argument("paths", ArgumentType::String)
        .required()
        .add_help("Paths to check, comma separated list");

    parser
}
