use crate::core::objects::index_file::GitIndex;
use crate::core::utils::gitignore_matcher::{GitIgnoreResult, GitignoreSet};
use crate::core::GitRepository;
use crate::utils::path;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct GitIgnore<'a> {
    repo: &'a GitRepository,
    index: Option<GitIndex<'a>>,
    absolute: Vec<GitignoreSet>, // Global and repo-wide rules
    scoped: HashMap<PathBuf, GitignoreSet>, // Directory-specific .gitignore files
}

pub enum IndexStrategy {
    IgnoreIndex,
    UseIndex,
}

impl<'a> GitIgnore<'a> {
    /// Creates a new `GitIgnore` instance from a `GitRepository`
    ///
    /// # Errors
    ///
    /// Errors if the global or local .gitignore files cannot be read
    pub fn from_repo(
        repo: &'a GitRepository,
        index_strategy: &IndexStrategy,
    ) -> Result<Self, String> {
        let mut gitignore = Self {
            repo,
            index: match index_strategy {
                IndexStrategy::IgnoreIndex => None,
                IndexStrategy::UseIndex => Some(GitIndex::read_index(repo)?),
            },
            absolute: Vec::new(),
            scoped: HashMap::new(),
        };

        // Read local configuration in .git/info/exclude
        let repo_exclude_file = gitignore.repo.gitdir().join("info/exclude");
        if repo_exclude_file.exists() {
            let mut ruleset =
                GitignoreSet::new(gitignore.repo.worktree().to_path_buf());
            ruleset.add_patterns_from_file(&repo_exclude_file)?;
            gitignore.absolute.push(ruleset);
        }

        // Global configuration
        let config_home =
            std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
                std::env::var("HOME").map_or_else(
                    |_| String::from("~/.config"),
                    |home| format!("{home}/.config",),
                )
            });

        let global_file = PathBuf::from(config_home).join("git/ignore");
        if global_file.exists() {
            let mut ruleset =
                GitignoreSet::new(gitignore.repo.worktree().to_path_buf());
            ruleset.add_patterns_from_file(&global_file)?;
            gitignore.absolute.push(ruleset);
        }

        // Find all .gitignore files in the repository
        gitignore.load_gitignore_files(gitignore.repo.worktree())?;

        Ok(gitignore)
    }

    fn load_gitignore_files(&mut self, dir: &Path) -> Result<(), String> {
        let gitignore_path = dir.join(".gitignore");
        if gitignore_path.exists() {
            let relative_dir = dir
                .strip_prefix(self.repo.worktree())
                .unwrap_or(Path::new(""))
                .to_path_buf();

            let mut ruleset =
                GitignoreSet::new(self.repo.worktree().to_path_buf());
            ruleset.add_patterns_from_file(&gitignore_path)?;
            self.scoped.insert(relative_dir, ruleset);
        }

        // Recursively load .gitignore files from subdirectories
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let path = entry.path();
                        // Skip .git directory
                        if path.file_name()
                            != Some(std::ffi::OsStr::new(".git"))
                        {
                            self.load_gitignore_files(&path)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_ignore_scoped(
        &self,
        absolute_path: &Path,
        is_dir: bool,
    ) -> GitIgnoreResult {
        // Get path relative to repo root for directory lookup
        let Some(repo_relative_path) =
            path::repo_relative_path(self.repo, absolute_path)
        else {
            return GitIgnoreResult::NotIgnored;
        };

        let mut current_dir =
            repo_relative_path.parent().unwrap_or(Path::new(""));

        loop {
            if let Some(ruleset) = self.scoped.get(current_dir) {
                let result = ruleset.is_ignored(absolute_path, is_dir);
                if !matches!(result, GitIgnoreResult::NotIgnored) {
                    return result;
                }
            }

            if current_dir == Path::new("") {
                break;
            }
            current_dir = current_dir.parent().unwrap_or(Path::new(""));
        }

        GitIgnoreResult::NotIgnored
    }

    fn check_ignore_absolute(
        &self,
        absolute_path: &Path,
        is_dir: bool,
    ) -> GitIgnoreResult {
        for ruleset in &self.absolute {
            let result = ruleset.is_ignored(absolute_path, is_dir);
            if !matches!(result, GitIgnoreResult::NotIgnored) {
                return result;
            }
        }
        GitIgnoreResult::NotIgnored
    }

    /// Checks if a path is ignored by gitignore rules.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check. If relative, it's resolved relative to the current working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::core::GitRepository;
    /// use mini_git::core::objects::ignore::{GitIgnore, IndexStrategy};
    ///
    /// let repo = GitRepository::create(Path::new("/test-repo"))?;
    /// let gitignore = GitIgnore::from_repo(&repo, &IndexStrategy::UseIndex)?;
    ///
    /// // If current directory is /test-repo/subdir and we check "file"
    /// // it will check /test-repo/subdir/file
    /// let result = gitignore.is_ignored(Path::new("file"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> GitIgnoreResult {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // If path is relative, resolve it relative to current working directory
            let current_dir = path::current_dir()
                .unwrap_or_else(|_| self.repo.worktree().to_path_buf());
            current_dir.join(path)
        };

        // Only check paths within the repository
        if !absolute_path.starts_with(self.repo.worktree()) {
            return GitIgnoreResult::NotIgnored;
        }

        let is_dir = absolute_path.is_dir();

        // If index exists, check if file is already in the index.
        // Ignored files can be tracked via a forced add.
        if !is_dir && self.index.is_some() {
            let Some(index) = self.index.as_ref() else {
                unreachable!("Index should exist if index is Some");
            };
            if let Ok(Some(_)) = index.get_file(&absolute_path) {
                return GitIgnoreResult::NotIgnored;
            }
        }

        // First check scoped rules (directory-specific .gitignore files)
        let scoped_result = self.check_ignore_scoped(&absolute_path, is_dir);
        if !matches!(scoped_result, GitIgnoreResult::NotIgnored) {
            return scoped_result;
        }

        // Then check absolute rules (global and repo-wide)
        self.check_ignore_absolute(&absolute_path, is_dir)
    }
}
