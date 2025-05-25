// DISCLAIMER
//
// Some tests in this module are designed to simulate the behavior of
// `mini-git ls-files` in a freshly initialized repository *without* a Git index.
//
// Since tests run in parallel and reuse the same temporary repository
// directory, another test may have already created an index file. To ensure
// test correctness and reproducibility, this test temporarily *renames*
// `.git/index` to `.git/index.orig` before running and restores it afterward.
//
// Restoration is performed using a guard object that ensures the index file is
// renamed back even if the test panics—making this approach safe and reversible.
//
// NOTE:
// No files are deleted in this process. This test should always run in
// disposable test environments to avoid conflicts.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mini_git::utils::path::to_posix_path;

/// A struct representing a Git index entry.
struct IndexEntry {
    ctime_s: u32,
    ctime_ns: u32,
    mtime_s: u32,
    mtime_ns: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    file_size: u32,
    sha1: [u8; 20],
    flags: u16,
    path: String,
}

impl IndexEntry {
    /// Create a new dummy index entry for testing
    fn new_dummy(path: &str) -> Self {
        // Get current time for timestamps
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        // Default mode: 100644 (regular file)
        // 100644 in octal = 0x81A4 in hex
        let default_mode = 0o100_644;

        // Create default flags:
        // - Path length for paths <= 0xFFF (12 bits)
        // - Assume stage 0 (2 bits)
        // - Extended flag = false (1 bit)
        // - First 1 bit (1 bit)
        #[expect(clippy::cast_possible_truncation)]
        let path_len = path.len() as u16;
        let flags = path_len & 0xFFF; // Only use the lower 12 bits for path length

        let path = to_posix_path(&PathBuf::from(path))
            .expect("Should convert to posix path");

        #[expect(clippy::cast_possible_truncation)]
        IndexEntry {
            ctime_s: now.as_secs() as u32,
            ctime_ns: now.subsec_nanos(),
            mtime_s: now.as_secs() as u32,
            mtime_ns: now.subsec_nanos(),
            dev: 0,
            ino: 0,
            mode: default_mode,
            uid: if cfg!(windows) { 0 } else { 1000 },
            gid: if cfg!(windows) { 0 } else { 1000 },
            file_size: 0,
            sha1: [0; 20],
            flags,
            path,
        }
    }

    /// Write the entry to a buffer in Git index format
    fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Write entry metadata
        writer.write_all(&self.ctime_s.to_be_bytes())?;
        writer.write_all(&self.ctime_ns.to_be_bytes())?;
        writer.write_all(&self.mtime_s.to_be_bytes())?;
        writer.write_all(&self.mtime_ns.to_be_bytes())?;
        writer.write_all(&self.dev.to_be_bytes())?;
        writer.write_all(&self.ino.to_be_bytes())?;
        writer.write_all(&self.mode.to_be_bytes())?;
        writer.write_all(&self.uid.to_be_bytes())?;
        writer.write_all(&self.gid.to_be_bytes())?;
        writer.write_all(&self.file_size.to_be_bytes())?;

        // Write SHA-1 (zeros for testing)
        writer.write_all(&self.sha1)?;

        // Write flags
        writer.write_all(&self.flags.to_be_bytes())?;

        // Write path
        writer.write_all(self.path.as_bytes())?;

        // Null-terminate the path
        writer.write_all(&[0])?;

        // Calculate padding to ensure next entry is aligned to 8 bytes
        let entry_size = 62 + self.path.len() + 1; // 62 = fixed metadata size, +1 for null terminator
        let padding_size = (8 - (entry_size % 8)) % 8;

        if padding_size > 0 {
            let padding = vec![0u8; padding_size];
            writer.write_all(&padding)?;
        }

        Ok(())
    }
}

/// Create a Git index file with dummy data for the given list of paths
fn create_dummy_git_index<P: AsRef<Path>>(
    output_path: P,
    file_paths: &[&str],
) -> io::Result<()> {
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Write header
    // - 4-byte signature: "DIRC"
    // - 4-byte version number (version 2)
    // - 4-byte number of entries
    writer.write_all(b"DIRC")?;
    writer.write_all(&2u32.to_be_bytes())?; // Version 2
    #[expect(clippy::cast_possible_truncation)]
    writer.write_all(&(file_paths.len() as u32).to_be_bytes())?;

    // Write entries
    for path in file_paths {
        let entry = IndexEntry::new_dummy(path);
        entry.write_to(&mut writer)?;
    }

    let checksum = [0u8; 20];
    writer.write_all(&checksum)?;

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_namespaces_from;

    use mini_git::core::commands::ls_files::*;
    use mini_git::core::GitRepository;
    use mini_git::utils::test::TempDir;

    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);

    const GIT_INDEX_PATH: &str = const {
        if cfg!(windows) {
            ".git\\index"
        } else {
            ".git/index"
        }
    };

    make_namespaces_from!(make_parser);

    macro_rules! switch_dir {
        ($body:block) => {
            match FS_MUTEX.lock() {
                Ok(inner) if inner.is_some() => {
                    (inner.as_ref().unwrap()).run(|| $body)
                }
                Ok(_) => unreachable!(),
                Err(..) => panic!("FS Mutex failed!"),
            }
        };
    }

    fn create_temp_repo<'a>() -> TempDir<'a, ()> {
        let tmp =
            TempDir::create("cmd_ls_files").with_mutex(&crate::TEST_MUTEX);
        GitRepository::create(tmp.tmp_dir()).expect("Create repo");

        let test_dir_indicator_file = tmp.tmp_dir().join(".minigit.testdir");
        fs::File::create(test_dir_indicator_file)
            .expect("Create test dir indicator file");

        tmp
    }

    fn setup() {
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                let tmp = create_temp_repo();
                *inner = Some(tmp);
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        }
    }

    #[test]
    fn test_create_dummy_git_index() {
        let temp_file = "test_index";
        let file_paths =
            ["file1.txt", "path/to/file2.txt", "another/path/file3.rs"];

        // Create the index file
        create_dummy_git_index(temp_file, &file_paths).unwrap();

        // Verify file exists with non-zero size
        let metadata = fs::metadata(temp_file).unwrap();
        assert!(metadata.len() > 0);

        // Clean up
        fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_ls_files_basic_functionality() {
        setup();
        let test_files = ["file1.txt", "src/main.rs", "README.md"];
        let result = switch_dir!({
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            // Test basic ls-files with no arguments
            let args: [&[&str]; 1] = [&[]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });

        assert!(result.is_ok(), "ls-files should succeed with basic index");

        let output = result.unwrap();

        // Check that all files are listed
        for file in test_files {
            assert!(output.contains(file), "Output should contain {file}");
        }

        // Files should be separated by newlines (default behavior)
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), test_files.len(), "Should list all files");
    }

    #[test]
    fn test_ls_files_with_debug_flag() {
        setup();

        let result = switch_dir!({
            let test_files = ["debug_test.txt"];
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            // Test with --debug flag
            let args: [&[&str]; 1] = [&["--debug"]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });

        assert!(result.is_ok(), "ls-files with debug should succeed");

        let output = result.unwrap();

        // Debug output should contain additional information
        assert!(
            output.contains("ctime:"),
            "Debug output should contain ctime"
        );
        assert!(
            output.contains("mtime:"),
            "Debug output should contain mtime"
        );
        assert!(
            output.contains("dev:"),
            "Debug output should contain device info"
        );
        assert!(
            output.contains("ino:"),
            "Debug output should contain inode info"
        );
        assert!(output.contains("uid:"), "Debug output should contain uid");
        assert!(output.contains("gid:"), "Debug output should contain gid");
        assert!(output.contains("size:"), "Debug output should contain size");
        assert!(
            output.contains("flags:"),
            "Debug output should contain flags"
        );
    }

    #[test]
    fn test_ls_files_with_null_separator() {
        setup();
        let test_files = ["file1.txt", "file2.txt", "file3.txt"];

        let result = switch_dir!({
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            // Test with -z (null separator) flag
            let args: [&[&str]; 1] = [&["-z"]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });
        assert!(result.is_ok(), "ls-files with -z should succeed");

        let output = result.unwrap();

        // Output should use null separators instead of newlines
        let null_separated_files: Vec<&str> = output.split('\0').collect();

        // Should have one more element than files due to trailing null
        assert!(
            null_separated_files.len() >= test_files.len(),
            "Should have null-separated entries"
        );

        // Check that files are present
        for file in test_files {
            assert!(output.contains(file), "Output should contain {file}");
        }
    }

    #[test]
    fn test_ls_files_with_full_path() {
        setup();
        let test_files = ["subdir/nested_file.txt", "root_file.txt"];

        let result = switch_dir!({
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            // Test with --full-path flag
            let args: [&[&str]; 1] = [&["--full-path"]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });
        assert!(result.is_ok(), "ls-files with --full-path should succeed");

        let output = result.unwrap();

        // All paths should be shown as full paths from repository root
        for file in &test_files {
            assert!(
                output.contains(file),
                "Output should contain full path {file}"
            );
        }
    }

    #[test]
    fn test_ls_files_combined_flags() {
        setup();

        let result = switch_dir!({
            let test_files = ["combined_test.txt"];
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            // Test with multiple flags combined
            let args: [&[&str]; 1] = [&["--debug", "--full-path", "-z"]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });
        assert!(
            result.is_ok(),
            "ls-files with combined flags should succeed"
        );

        let output = result.unwrap();

        // Should contain debug information
        assert!(
            output.contains("ctime:"),
            "Combined output should contain debug info"
        );

        // Should use null separators
        assert!(
            output.contains('\0'),
            "Combined output should use null separators"
        );
    }

    #[test]
    fn test_ls_files_empty_index() {
        setup();

        let result = switch_dir!({
            // Create empty index
            let empty_files: [&str; 0] = [];
            create_dummy_git_index(GIT_INDEX_PATH, &empty_files)
                .expect("Failed to create empty index");

            let args: [&[&str]; 1] = [&[]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });
        assert!(result.is_ok(), "ls-files should handle empty index");

        let output = result.unwrap();
        assert!(
            output.trim().is_empty(),
            "Output should be empty for empty index"
        );
    }

    #[test]
    fn test_ls_files_missing_index() {
        struct IndexBackup {
            orig: PathBuf,
            backup: PathBuf,
        }

        impl IndexBackup {
            fn new() -> std::io::Result<Option<Self>> {
                let orig = PathBuf::from(GIT_INDEX_PATH);
                let backup = PathBuf::from(&format!("{GIT_INDEX_PATH}.orig"));

                if orig.exists() {
                    if backup.exists() {
                        fs::remove_file(&backup)?;
                    }
                    fs::rename(&orig, &backup)?;
                    Ok(Some(Self { orig, backup }))
                } else {
                    Ok(None)
                }
            }
        }

        impl Drop for IndexBackup {
            fn drop(&mut self) {
                // Restore index file if backup exists
                if self.backup.exists() {
                    let _ = fs::rename(&self.backup, &self.orig);
                }
            }
        }

        setup();

        let result = switch_dir!({
            // will restore the index file on drop
            let _index_guard =
                IndexBackup::new().expect("should backup index file");

            let args: [&[&str]; 1] = [&[]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });

        assert!(result.is_ok(), "ls-files should handle missing index");
        let output = result.unwrap();
        assert!(
            output.trim().is_empty(),
            "Output should be empty when index is missing"
        );
    }

    #[test]
    fn test_ls_files_various_file_types() {
        setup();
        // Test various file types and paths
        let test_files = [
            "README.md",
            "src/main.rs",
            "tests/integration_test.rs",
            "Cargo.toml",
            "docs/guide.md",
            ".gitignore",
            "config/settings.json",
        ];

        let result = switch_dir!({
            create_dummy_git_index(GIT_INDEX_PATH, &test_files)
                .expect("Failed to create dummy index");

            let args: [&[&str]; 1] = [&[]];
            let namespace = make_namespaces(&args).next().unwrap();

            ls_files(&namespace)
        });
        assert!(result.is_ok(), "ls-files should handle various file types");

        let output = result.unwrap();

        for file in &test_files {
            assert!(output.contains(file), "Output should contain {file}");
        }
    }
}
