//! # Test Utilities Module
//!
//! This module provides utilities for setting up and managing temporary directories
//! for testing purposes. It includes functionality for creating isolated test
//! environments, reverting changes, and walking directory structures.
//!
//! ## Main Components
//!
//! - [`TempDir`]: A struct for creating and managing temporary directories.
//! - [`walkdir`]: A function for recursively listing files in a directory.
//!
//! ## Usage
//!
//! This module is primarily intended for use in test scenarios where isolated
//! file system operations are required. It allows for easy creation of temporary
//! directories that are automatically cleaned up when they go out of scope.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A struct representing a temporary directory for testing purposes.
///
/// This struct manages the creation and cleanup of a temporary directory,
/// as well as changing the current working directory to the temporary directory
/// and reverting it back when done.
///
/// # Examples
///
/// ```
/// use mini_git::utils::test::TempDir;
///
/// let temp_dir = TempDir::<()>::create("my_test");
/// // Perform test operations in the temporary directory
/// // The directory will be automatically cleaned up when `temp_dir` goes out of scope
/// ```
#[derive(Clone)]
pub struct TempDir<'a, T> {
    original_dir: PathBuf,
    tmp_dir: PathBuf,
    auto_revert: bool,
    mutex: Option<&'a Mutex<T>>,
}

impl<'a, T> TempDir<'a, T> {
    /// Returns a reference to the path of the temporary test directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::<()>::create("my_test");
    /// let test_path = temp_dir.tmp_dir();
    /// println!("Temporary directory path: {:?}", test_path);
    /// ```
    #[must_use]
    pub fn tmp_dir(&self) -> &Path {
        &self.tmp_dir
    }

    /// Creates a new temporary directory for testing.
    ///
    /// This function creates a unique temporary directory based on the provided
    /// `dirname` and a timestamp. It also changes the current working directory
    /// to the newly created temporary directory.
    ///
    /// # Arguments
    ///
    /// * `dirname` - A base name for the temporary directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::<()>::create("my_test");
    /// // The current working directory is now the temporary directory
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - It fails to get the current system time.
    /// - It fails to create the temporary directory.
    /// - It fails to change the current working directory to the temporary directory.
    #[must_use]
    pub fn create(dirname: &str) -> Self {
        let salt = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Should return time")
            .as_nanos();
        let original_dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();

        let dirname = format!("{dirname}{salt}");
        let tmp_dir = env::temp_dir().join(dirname);
        fs::create_dir_all(&tmp_dir).unwrap();

        Self {
            original_dir,
            tmp_dir,
            auto_revert: true,
            mutex: None,
        }
    }

    /// Controls whether the current working directory is reverted to the
    /// original directory that this [`TempDir`] was created in when dropped.
    ///
    /// Default behavior is automatically reverting, this function can override
    /// that behavior.
    pub fn auto_revert(&mut self, revert: bool) {
        self.auto_revert = revert;
    }

    /// Switches to the temporary directory.
    ///
    /// [`TempDir::<()>::create`] automatically switches to the temporary directory,
    /// however this function allows a manual switch as needed.
    ///
    /// This is especially useful when working in a multithreaded context,
    /// where other threads may change the current working directory in a
    /// non-deterministic order.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// // Create s
    /// let temp_dir = TempDir::<()>::create("my_test");
    /// // Perform test operations
    /// temp_dir.revert();
    /// // The current working directory is now back to the original
    /// ```
    pub fn switch(&self) {
        Self::switch_to(self.mutex, &self.tmp_dir);
    }

    /// Switches back to the orignal directory where this [`TempDir`] was
    /// created.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// // Create
    /// let temp_dir = TempDir::<()>::create("my_test");
    /// temp_dir.switch();
    /// // Perform operations
    ///
    /// temp_dir.switch_back();
    /// // The current working directory is now back to the original
    /// ```
    pub fn switch_back(&self) {
        Self::switch_to(self.mutex, &self.original_dir);
    }

    /// Runs a function in the temporary directory
    ///
    /// If the `TempDir` has a mutex, this function will prevent other threads
    /// from changing the working directory, allowing the function to
    /// complete in the temporary directory.
    ///
    /// If a mutex is not used, the working directory will be changed to the
    /// temporary directory, but it is not guaranteed that the function will
    /// complete in the temporary directory.
    ///
    /// # Panics
    ///
    /// This function panics if the mutex fails or is poisoned.
    pub fn run<R>(&self, f: impl Fn() -> R) -> R {
        if let Some(mutex) = self.mutex {
            if let Ok(_guard) = mutex.lock() {
                Self::switch_dir(&self.tmp_dir);
                return f();
            }
            panic!("TempDir Mutex failed!");
        }

        Self::switch_dir(&self.tmp_dir);
        f()
    }

    fn switch_to<P>(mutex: Option<&'a Mutex<T>>, to: P)
    where
        P: AsRef<Path>,
    {
        if let Some(mutex) = mutex {
            if let Ok(_guard) = mutex.lock() {
                Self::switch_dir(to);
                return;
            }
            panic!("TempDir Mutex failed!")
        }

        Self::switch_dir(to);
    }

    fn switch_dir<P>(to: P)
    where
        P: AsRef<Path>,
    {
        env::set_current_dir(to).expect("Should chdir");
    }

    /// Reverts the current working directory to the original directory
    /// and attempts to remove the temporary directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::<()>::create("my_test");
    /// // Perform test operations
    /// temp_dir.revert();
    /// // The current working directory is now back to the original
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if it fails to change the current working
    /// directory back to the original directory.
    pub fn revert(&self) {
        println!("Deleting {:?}", &self.tmp_dir);
        // This may not immediately delete, so we just ignore the retval
        if let Err(res) = fs::remove_dir_all(&self.tmp_dir) {
            println!("CLEANING UP {:?}", &self.tmp_dir);
            println!("RESULT {:?}", res);
        }

        if self.auto_revert {
            self.switch_back();
        }
    }

    /// Returns a [`TempDir`] that performs uses the given mutex before
    /// switching dirs or running a closure in context.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    /// use std::sync::Mutex;
    ///
    /// let mutex = Mutex::new(());
    /// let tmp = TempDir::<()>::create("temp").with_mutex(&mutex);
    ///
    /// // Operations will use mutex.lock()
    /// tmp.switch();
    /// ```
    #[must_use]
    pub fn with_mutex(mut self, mutex: &'a Mutex<T>) -> Self {
        self.mutex = Some(mutex);
        self
    }
}

impl<'a, T> Drop for TempDir<'a, T> {
    fn drop(&mut self) {
        self.revert();
    }
}

/// Recursively walks a directory and returns a vector of all file paths.
///
/// This function traverses the given directory and its subdirectories,
/// collecting the paths of all files (excluding hidden files) into a vector.
///
/// # Arguments
///
/// * `top` - The path to the directory to walk.
///
/// # Returns
///
/// A `Vec<PathBuf>` containing the paths of all non-hidden files in the directory
/// and its subdirectories.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use mini_git::utils::test::walkdir;
///
/// let dir_path = Path::new("/path/to/directory");
/// let files = walkdir(dir_path);
/// for file in files {
///     println!("Found file: {:?}", file);
/// }
/// ```
///
/// # Panics
///
/// This function will panic if:
/// - The provided `top` path is not a directory.
/// - It fails to read the contents of the directory or any of its subdirectories.
#[must_use]
pub fn walkdir(top: &Path) -> Vec<PathBuf> {
    assert!(top.is_dir(), "Top is not a directory (top = {top:?})");
    top.read_dir()
        .expect("Should read the dir")
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.file_stem().is_some_and(|stem| {
                !stem.to_str().is_some_and(|x| x.starts_with('.'))
            })
        })
        .fold(vec![], |mut paths, entry| {
            if entry.is_file() {
                paths.push(entry);
            } else {
                paths.extend_from_slice(&walkdir(&entry));
            }
            paths
        })
}
