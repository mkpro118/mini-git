//! # Path Utility Module
//!
//! This module provides utility functions for working with paths in a Git-like
//! repository structure. It offers functionality to manipulate and create
//! paths within a given base directory (typically a `.git` directory).
//!
//! ## Main Functions
//!
//! - [`repo_path`]: Joins paths to a base directory without creating any files or directories.
//! - [`repo_file`]: Returns a file path and optionally creates intermediate directories.
//! - [`repo_dir`]: Returns a directory path and optionally creates the directory structure.
//!
//! ## Usage
//!
//! This module is primarily used for handling paths within a Git-like repository
//! structure. It's useful for operations that involve accessing or creating
//! files and directories within a specific repository layout.
//!
//! ## Error Handling
//!
//! Functions in this module return `Result` types when operations might fail
//! (e.g., due to I/O errors). Users should handle these potential errors
//! appropriately in their code.

use std::fs;
use std::path::{Path, PathBuf};

const POSIX_PATH_SEPARATOR: char = '/';
const CURRENT_DIR_STR: &str = ".";
const PARENT_DIR_STR: &str = "..";

/// Determines the current working directory.
///
/// # Errors
///
/// This function only fails if `std::env::current_dir` fails.
pub fn current_dir() -> Result<std::path::PathBuf, String> {
    std::env::current_dir()
        .map_err(cwd_err)?
        .canonicalize()
        .map_err(cwd_err)
}

#[inline]
fn cwd_err(_: std::io::Error) -> String {
    "Could not determine current working directory".to_owned()
}

/// Converts a filesystem path to a POSIX-compliant path string representation.
///
/// This function takes a `Path` and converts it into a POSIX-style path string,
/// where components are separated by forward slashes ('/'). The resulting string
/// will not have a trailing slash.
///
/// # Arguments
///
/// * `path` - A reference to a Path object to be converted
///
/// # Returns
///
/// Returns a `Result` containing either:
/// * `Ok(String)` - A POSIX-compliant path string
/// * `Err(String)` - An error message if conversion fails
///
/// # Examples
///
/// #### Windows
///
/// ```rust
/// # use std::path::Path;
/// # use mini_git::utils::path::to_posix_path;
///
/// # #[cfg(target_family="windows")]
/// # fn win() {
/// let windows_path = Path::new("C:\\Users\\Documents\\file.txt");
/// assert_eq!(
///     to_posix_path(windows_path).unwrap(),
///     "C:/Users/Documents/file.txt"
/// );
/// # }
/// ```
///
/// #### Unix
/// ```rust
/// # use std::path::Path;
/// # use mini_git::utils::path::to_posix_path;
///
/// # #[cfg(target_family="unix")]
/// # fn unix() {
/// let unix_path = Path::new("/home/user/documents/file.txt");
/// assert_eq!(
///     to_posix_path(unix_path).unwrap(),
///     "/home/user/documents/file.txt"
/// );
/// # }
/// ```
///
/// # Errors
///
/// Returns an error in the following cases:
/// * Any path component contains invalid Unicode characters
/// * The path cannot be successfully processed or converted
#[expect(clippy::module_name_repetitions)]
pub fn to_posix_path(path: &Path) -> Result<String, String> {
    use std::path::{Component, Prefix};

    let mut posix_path = String::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix_comp) => {
                match prefix_comp.kind() {
                    Prefix::Disk(drive_letter)
                    | Prefix::VerbatimDisk(drive_letter) => {
                        // Handle drive letters (e.g., "C:")
                        posix_path.push(drive_letter as char);
                        posix_path.push(':');
                    }
                    Prefix::UNC(server, share)
                    | Prefix::VerbatimUNC(server, share) => {
                        // Handle UNC paths (e.g., "\\server\share")
                        posix_path.push(POSIX_PATH_SEPARATOR);
                        posix_path.push(POSIX_PATH_SEPARATOR);
                        posix_path.push_str(&server.to_string_lossy());
                        posix_path.push(POSIX_PATH_SEPARATOR);
                        posix_path.push_str(&share.to_string_lossy());
                    }
                    Prefix::Verbatim(_) => {
                        // Ignore the "\\?\" prefix for extended-length paths
                        continue;
                    }
                    Prefix::DeviceNS(_) => {
                        return Err(format!(
                            "Unsupported prefix in path {path:?}"
                        ));
                    }
                }
            }
            Component::RootDir => {
                // Optionally handle root directory, but for your cases, we can ignore it
                if posix_path.is_empty() {
                    posix_path.push(POSIX_PATH_SEPARATOR);
                }
            }
            Component::Normal(os_str) => {
                if !posix_path.is_empty()
                    && !posix_path.ends_with(POSIX_PATH_SEPARATOR)
                {
                    posix_path.push(POSIX_PATH_SEPARATOR);
                }
                posix_path.push_str(&os_str.to_string_lossy());
            }
            Component::CurDir => {
                if !posix_path.is_empty()
                    && !posix_path.ends_with(POSIX_PATH_SEPARATOR)
                {
                    posix_path.push(POSIX_PATH_SEPARATOR);
                }
                posix_path.push_str(CURRENT_DIR_STR);
            }
            Component::ParentDir => {
                if !posix_path.is_empty()
                    && !posix_path.ends_with(POSIX_PATH_SEPARATOR)
                {
                    posix_path.push(POSIX_PATH_SEPARATOR);
                }
                posix_path.push_str(PARENT_DIR_STR);
            }
        }
    }

    Ok(posix_path)
}

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
#[expect(clippy::module_name_repetitions)]
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

/// Returns the path to the last directory after joining `gitdir` with the
/// given `paths`.
/// Optionally, if `create = true`, then it creates any missing directories
/// in the path. It DOES create the last directory.
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
/// use mini_git::utils::path::repo_dir;
/// use std::path::Path;
///
/// let base = Path::new(".git");
/// let head_path = repo_dir(base, &["refs", "head"], true)?;
/// assert!(base.join("hooks").exists());
/// assert!(base.join("hooks").is_dir());
/// assert_eq!(head_path, Some(base.join("refs").join("head")));
/// # Ok::<(), String>(())
/// ```
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

/// Returns the path to the root of the repository, traversing from `top` to
/// the root.
///
/// # Errors
///
/// If an I/O error occurs while resolving paths, or if no ancestor of `top`
/// upto the root is a repository root.
/// An error message describing the error is returned
///
/// # Example
/// Note: This example will only work if the VCS is being used in the current
/// crate
///
/// ```
/// use mini_git::utils::path::repo_find;
///
/// let top = env!("CARGO_MANIFEST_DIR");
/// let repo_root = repo_find(&top)?;
/// println!("{:?}", repo_root.as_os_str());
/// # Ok::<(), String>(())
/// ```
pub fn repo_find<P>(top: P) -> Result<PathBuf, String>
where
    P: AsRef<Path>,
{
    const GITDIR: &str = ".git";

    let top = top.as_ref();
    let path = Path::new(top);
    let Ok(path) = path.canonicalize() else {
        return Err(format!("Could not resolve path {:?}", path.as_os_str()));
    };

    for dir in path.ancestors() {
        if dir.join(GITDIR).is_dir() {
            return Ok(dir.to_path_buf());
        }
    }

    Err(format!(
        "neither {top:?} nor any of it's parent directories \
                 is a repository."
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::utils::test::*;

    #[test]
    fn test_repo_path() {
        let base = Path::new(".git");
        let result = repo_path(base, &["refs", "heads"]);
        assert_eq!(result, base.join("refs").join("heads"));

        let result = repo_path(base, &["objects", "pack"]);
        assert_eq!(result, base.join("objects").join("pack"));

        // Test with empty paths
        let result = repo_path::<&str>(base, &[]);
        assert_eq!(result, base.to_path_buf());
    }

    #[test]
    fn test_repo_file() {
        let tmp_dir = TempDir::<()>::create("test_repo_file");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        // Test without creating directories
        let result =
            repo_file(&base, &["refs", "heads", "master"], false).unwrap();
        assert_eq!(result, None);

        // Test with creating directories
        let result =
            repo_file(&base, &["refs", "heads", "master"], true).unwrap();
        assert_eq!(
            result,
            Some(base.join("refs").join("heads").join("master"))
        );
        assert!(base.join("refs").join("heads").is_dir());

        // Test with existing directories
        let result =
            repo_file(&base, &["refs", "heads", "develop"], true).unwrap();
        assert_eq!(
            result,
            Some(base.join("refs").join("heads").join("develop"))
        );

        // Test with invalid path (existing file as directory)
        fs::File::create(base.join("invalid")).unwrap();
        let result = repo_file(&base, &["invalid", "file"], true);
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_dir() {
        let tmp_dir = TempDir::<()>::create("test_repo_dir");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        // Test without creating directories
        let result = repo_dir(&base, &["refs", "heads"], false).unwrap();
        assert_eq!(result, None);

        // Test with creating directories
        let result = repo_dir(&base, &["refs", "heads"], true).unwrap();
        assert_eq!(result, Some(base.join("refs").join("heads")));
        assert!(base.join("refs").join("heads").is_dir());

        // Test with existing directories
        let result = repo_dir(&base, &["refs"], true).unwrap();
        assert_eq!(result, Some(base.join("refs")));

        // Test with invalid path (existing file as directory)
        fs::File::create(base.join("invalid")).unwrap();
        let result = repo_dir(&base, &["invalid"], true);
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_dir_with_existing_file() {
        let tmp_dir = TempDir::<()>::create("test_repo_dir_with_existing_file");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        // Create a file instead of a directory
        fs::File::create(base.join("file")).unwrap();

        // Try to create a directory with the same name
        let result = repo_dir(&base, &["file"], true);
        assert!(result.is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "subtract with overflow")]
    fn test_repo_file_with_empty_paths() {
        let tmp_dir = TempDir::<()>::create("test_repo_file_with_empty_paths");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        let result = repo_file::<&str>(&base, &[], true);
        assert!(result.is_err());
    }

    #[cfg(not(debug_assertions))]
    #[test]
    #[should_panic(expected = "out of range")]
    fn test_repo_file_with_empty_paths() {
        let tmp_dir = TempDir::<()>::create("test_repo_file_with_empty_paths");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        let result = repo_file::<&str>(&base, &[], true);
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_dir_with_empty_paths() {
        let tmp_dir = TempDir::<()>::create("test_repo_dir_with_empty_paths");
        let base = tmp_dir.tmp_dir().join(".git");
        fs::create_dir(&base).unwrap();

        let result = repo_dir::<&str>(&base, &[], true).unwrap();
        assert_eq!(result, Some(base));
    }

    #[test]
    fn test_repo_find_with_manifest() {
        let top = env!("CARGO_MANIFEST_DIR");
        let expected =
            Path::new(top).canonicalize().expect("Should get abspath");
        let repo_root = repo_find(top).unwrap();
        assert_eq!(repo_root, expected);
    }

    #[test]
    fn test_repo_find_with_manifest_subdir_src() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let top = Path::new(manifest).join("src");
        let expected = Path::new(&manifest)
            .canonicalize()
            .expect("Should get abspath");
        let repo_root = repo_find(top).unwrap();
        assert_eq!(repo_root, expected);
    }

    #[test]
    fn test_repo_find_with_manifest_subdir_tests() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let top = Path::new(manifest).join("tests");
        let expected = Path::new(&manifest)
            .canonicalize()
            .expect("Should get abspath");
        let repo_root = repo_find(top).unwrap();
        assert_eq!(repo_root, expected);
    }

    #[test]
    fn test_repo_find_bad_dir() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let top = Path::new(manifest).join("bad_dir");
        let res = repo_find(top);
        assert!(res.is_err());
    }

    #[test]
    fn test_repo_find_no_git() {
        let tmp_dir = TempDir::<()>::create("test_repo_find_no_git");
        let top = tmp_dir.tmp_dir();
        let res = repo_find(top);
        assert!(res.is_err());
    }

    // Helper function to create paths with different separators based on OS
    fn create_path(components: &[&str]) -> String {
        if cfg!(target_family = "windows") {
            components.join("\\")
        } else {
            components.join("/")
        }
    }

    #[test]
    fn test_empty_path() {
        let path = Path::new("");
        let result = to_posix_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_dot_path() {
        let path = Path::new(".");
        let result = to_posix_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ".");
    }

    #[test]
    fn test_double_dot_path() {
        let path = Path::new("..");
        let result = to_posix_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "..");
    }

    // Test valid Unicode paths
    #[test]
    fn test_unicode_valid_paths() {
        let test_cases = vec![
            ("é", "é"),
            ("パス", "パス"),
            ("путь", "путь"),
            ("路径", "路径"),
            ("König", "König"),
        ];

        for (input, expected) in test_cases {
            let path = Path::new(input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    // Test paths with mixed forward and backward slashes
    #[cfg(target_family = "windows")]
    #[test]
    fn test_mixed_slashes_windows() {
        let test_cases = vec![
            (r"path\to/file", "path/to/file"),
            (r"path/to\file", "path/to/file"),
            (r"path\to\file/name", "path/to/file/name"),
        ];

        for (input, expected) in test_cases {
            let path = Path::new(input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    // Test multiple consecutive separators
    #[test]
    fn test_consecutive_separators() {
        let test_cases = vec![
            create_path(&["path", "", "file"]),
            create_path(&["path", "", "", "file"]),
            create_path(&["", "path", "file"]),
            create_path(&["path", "file", ""]),
        ];

        for input in test_cases {
            let path = Path::new(&input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            // All consecutive separators should be normalized to single separators
            assert!(!result.unwrap().contains("//"));
        }
    }

    // Test invalid Unicode paths
    #[test]
    #[expect(invalid_from_utf8_unchecked, unsafe_code)]
    fn test_invalid_unicode() {
        // Create a path with invalid UTF-8 sequences
        let invalid_path = if cfg!(target_family = "windows") {
            Path::new("\u{FFFD}")
        } else {
            Path::new(unsafe {
                std::str::from_utf8_unchecked(&[0xFF, 0xFE, 0xFD])
            })
        };

        let result = to_posix_path(invalid_path);
        assert!(result.is_ok());
    }

    // Test deep nested paths
    #[test]
    fn test_deep_nested_paths() {
        let components = vec!["a"; 100]; // Create a very deep path
        let path_str = create_path(&components);
        let path = Path::new(&path_str);
        let result = to_posix_path(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().matches('/').count(), 99); // Should have 99 separators for 100 components
    }

    // Test paths with spaces and special characters
    #[test]
    fn test_special_characters() {
        let test_cases = vec![
            "path with spaces",
            "path_with!special@chars#",
            "path with (parentheses)",
            "path with [brackets]",
            "path.with.dots",
            "path-with-dashes",
        ];

        for input in test_cases {
            let path = Path::new(input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), input);
        }
    }

    // Test with root paths
    #[test]
    fn test_root_paths() {
        if cfg!(target_family = "windows") {
            let test_cases = vec![
                (r"C:\", "C:"),
                (r"C:\path", "C:/path"),
                (r"\\server\share", "//server/share"),
            ];

            for (input, expected) in test_cases {
                let path = Path::new(input);
                let result = to_posix_path(path);
                assert!(result.is_ok());
                dbg!(&result);
                assert_eq!(result.unwrap(), expected);
            }
        } else {
            let test_cases = vec![
                ("/", "/"),
                ("/path", "/path"),
                ("/path/to/file", "/path/to/file"),
            ];

            for (input, expected) in test_cases {
                let path = Path::new(input);
                let result = to_posix_path(path);
                assert!(result.is_ok());
                assert_eq!(result.unwrap(), expected);
            }
        }
    }

    // Test with environment-specific paths
    #[cfg(target_family = "windows")]
    #[test]
    fn test_windows_specific_paths() {
        let test_cases = vec![
            // UNC paths
            (r"\\server\share\path", "//server/share/path"),
            // Drive letters with different cases
            (r"c:\path", "C:/path"),
            (r"D:\path", "D:/path"),
            // Reserved names
            (r"CON", "CON"),
            (r"PRN\file", "PRN/file"),
            // Extended-length paths
            (r"\\?\C:\very\long\path", "C:/very/long/path"),
        ];

        for (input, expected) in test_cases {
            let path = Path::new(input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn test_unix_specific_paths() {
        let test_cases = vec![
            // Hidden files
            (".hidden", ".hidden"),
            // Absolute paths
            ("/usr/local/bin", "/usr/local/bin"),
            // Current directory references
            ("./path", "./path"),
            // Parent directory references
            ("../path", "../path"),
            // Symbolic links (the function should treat them as regular paths)
            ("link/to/file", "link/to/file"),
        ];

        for (input, expected) in test_cases {
            let path = Path::new(input);
            let result = to_posix_path(path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }
}
