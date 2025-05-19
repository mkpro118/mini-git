#![forbid(clippy::complexity)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::configparser::ConfigParser;
use crate::utils::path;

/// A struct representing a Git repository.
#[expect(clippy::module_name_repetitions, dead_code)]
#[derive(Debug)]
pub struct GitRepository {
    /// The working tree of the repository.
    worktree: PathBuf,
    /// The `.git` directory of the repository.
    gitdir: PathBuf,
    /// The configuration of the repository.
    config: ConfigParser,
}

impl GitRepository {
    /// Creates a new `GitRepository` instance.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path where the repository is located.
    ///
    /// # Errors
    ///
    /// Returns a `String` error if the repository could not be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::core::GitRepository;
    /// let repo = GitRepository::new(Path::new("/path/to/repo"))?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn new(path: &Path) -> Result<Self, String> {
        Self::new_repo(path, false)
    }

    /// Returns the working tree path of the repository.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::core::GitRepository;
    /// let repo = GitRepository::create(Path::new("."))?;
    /// let worktree = repo.worktree();
    /// println!("{worktree:?}");
    /// # Ok::<(), String>(())
    /// ```
    #[must_use]
    pub fn worktree(&self) -> &Path {
        &self.worktree
    }

    /// Returns the `.git` directory path of the repository.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::core::GitRepository;
    /// let repo = GitRepository::create(Path::new("."))?;
    /// let gitdir = repo.gitdir();
    /// println!("{gitdir:?}");
    /// # Ok::<(), String>(())
    /// ```
    #[must_use]
    pub fn gitdir(&self) -> &Path {
        &self.gitdir
    }

    /// Creates a new repository object at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path where the repository should be created.
    /// * `forced` - A boolean indicating whether the creation should be forced.
    ///
    /// # Errors
    ///
    /// Returns a `String` error if the repository could not be created.
    fn new_repo(path: &Path, forced: bool) -> Result<Self, String> {
        let not_forced = !forced;

        let path = if path.is_relative() && !path.starts_with(".") {
            &Path::new(".").join(path)
        } else {
            path
        };

        let Some(parent) = path.parent() else {
            return Err(format!("{:?} is not a valid path!", path.as_os_str()));
        };

        let Ok(parent) = parent.canonicalize() else {
            return Err(format!("{:?} is not a valid path!", path.as_os_str()));
        };

        let worktree = parent.join(
            path.file_name()
                .expect("Should be a valid path unless it ends with .."),
        );

        let gitdir = path.join(".git");

        if not_forced && !gitdir.is_dir() {
            return Err(format!("not a git repository {:?}", path.as_os_str()));
        }

        let config;
        let config_file = path::repo_file(&gitdir, &["config"], false)?;
        if let Some(config_file) = config_file {
            config = ConfigParser::from(config_file.as_path());
        } else if not_forced {
            return Err("missing configuration file!".to_string());
        } else {
            config = ConfigParser::default();
        }

        if not_forced {
            let Some(core) = config.get("core") else {
                return Err("section \"core\" is missing!".to_string());
            };
            match core.get_int("repositoryformatversion") {
                Some(0) => {}
                Some(version) => {
                    return Err(format!(
                        "unsupported repositoryformatversion {version}"
                    ))
                }
                None => {
                    return Err("key \"repositoryformatversion\" is missing"
                        .to_string())
                }
            }
        }

        Ok(Self {
            worktree,
            gitdir,
            config,
        })
    }

    /// Initializes and creates a new Git repository at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path where the repository should be created.
    ///
    /// # Errors
    ///
    /// Returns a `String` error if the repository could not be created.
    ///
    /// # Panics
    ///
    /// If an I/O error occurs while creating a repository
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::core::GitRepository;
    /// let repo = GitRepository::create(Path::new("/path/to/repo"))?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn create(path: &Path) -> Result<Self, String> {
        let repo = Self::new_repo(path, true)?;

        if repo.worktree.exists() {
            if !repo.worktree.is_dir() {
                return Err(format!("not a directory {:?}", path.as_os_str()));
            }

            if repo.gitdir.read_dir().is_ok_and(|mut e| e.next().is_some()) {
                return Err(format!("{:?} is not empty", path.as_os_str()));
            }
        } else if fs::create_dir_all(&repo.worktree).is_err() {
            return Err("error in making directories".to_string());
        }

        path::repo_dir(&repo.gitdir, &["branches"], true)?;
        path::repo_dir(&repo.gitdir, &["objects"], true)?;
        path::repo_dir(&repo.gitdir, &["objects", "pack"], true)?;
        path::repo_dir(&repo.gitdir, &["refs", "tags"], true)?;
        path::repo_dir(&repo.gitdir, &["refs", "heads"], true)?;

        if let Some(file) =
            path::repo_file(&repo.gitdir, &["description"], false)?
        {
            fs::write(
                file,
                "Unnamed repository; edit this file 'description' to name the \
                repository.\n",
            )
            .expect("Should write to file!");
        }

        if let Some(file) = path::repo_file(&repo.gitdir, &["HEAD"], false)? {
            fs::write(file, "ref: refs/heads/main\n")
                .expect("Should write to file!");
        }

        if let Some(file) = path::repo_file(&repo.gitdir, &["config"], false)? {
            let default_config = Self::default_config();
            if default_config.write_to_file(&file).is_err() {
                return Err("error occurred while writing \
                            configuration file"
                    .to_string());
            }
        }

        Ok(repo)
    }

    /// Creates the default configuration for a Git repository.
    fn default_config() -> ConfigParser {
        let mut config = ConfigParser::new();
        config["core"]["repositoryformatversion"] = String::from("0");
        config["core"]["filemode"] = String::from("false");
        config["core"]["bare"] = String::from("false");

        config
    }
}

// Holds the context of a Git repository, including the current working directory,
/// repository path, and a reference to the Git repository.
#[expect(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct RepositoryContext {
    /// The current working directory, resolved when `resolve_repository_context` is called.
    pub cwd: PathBuf,

    /// The absolute path to the root of the repository's worktree.
    pub repo_path: PathBuf,

    /// The `GitRepository` representing the current repository.
    pub repo: GitRepository,
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
pub fn resolve_repository_context() -> Result<RepositoryContext, String> {
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
