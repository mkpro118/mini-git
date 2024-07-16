#![allow(missing_docs, clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![forbid(clippy::complexity)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::configparser::ConfigParser;

/// A git repository
#[allow(clippy::module_name_repetitions, dead_code)]
#[derive(Debug)]
pub struct GitRepository {
    worktree: PathBuf,
    gitdir: PathBuf,
    config: ConfigParser,
}

impl GitRepository {
    pub fn new(path: &Path) -> Result<Self, String> {
        Self::new_repo(path, false)
    }

    #[must_use]
    pub fn worktree(&self) -> &Path {
        &self.worktree
    }

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
        let config_file = path_utils::repo_file(&gitdir, &["config"], false)?;
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

    pub fn create(path: &Path) -> Result<Self, String> {
        let repo = Self::new_repo(path, true)?;

        if repo.worktree.exists() {
            if !repo.worktree.is_dir() {
                return Err(format!("not a directory {:?}", path.as_os_str()));
            }

            if repo
                .gitdir
                .read_dir()
                .map_or(false, |mut e| e.next().is_some())
            {
                return Err(format!("{:?} is not empty", path.as_os_str()));
            }
        } else if fs::create_dir_all(&repo.worktree).is_err() {
            return Err("error in making directories".to_string());
        }

        path_utils::repo_dir(&repo.gitdir, &["branches"], true)?;
        path_utils::repo_dir(&repo.gitdir, &["objects"], true)?;
        path_utils::repo_dir(&repo.gitdir, &["refs", "tags"], true)?;
        path_utils::repo_dir(&repo.gitdir, &["refs", "heads"], true)?;

        if let Some(file) =
            path_utils::repo_file(&repo.gitdir, &["description"], false)?
        {
            fs::write(
                file,
                "Unnamed repository; edit this file 'description' to name the \
                repository.\n",
            )
            .expect("Should write to file!");
        }

        if let Some(file) =
            path_utils::repo_file(&repo.gitdir, &["HEAD"], false)?
        {
            fs::write(file, "ref: refs/heads/main\n")
                .expect("Should write to file!");
        }

        if let Some(file) =
            path_utils::repo_file(&repo.gitdir, &["config"], false)?
        {
            let default_config = Self::default_config();
            if default_config.write_to_file(&file).is_err() {
                return Err("error occurred while writing \
                            configuration file"
                    .to_string());
            }
        }

        Ok(repo)
    }

    fn default_config() -> ConfigParser {
        let mut config = ConfigParser::new();
        config["core"]["repositoryformatversion"] = String::from("0");
        config["core"]["filemode"] = String::from("false");
        config["core"]["bare"] = String::from("false");

        config
    }
}

pub mod path_utils {
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
        let Some(_) = repo_dir(gitdir, &paths[..(paths.len() - 1)], create)?
        else {
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
}
