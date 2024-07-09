#[cfg(test)]
mod tests {
    use mini_git::fnmatch::fnmatch;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn setup_test_directory(dirname: &str) -> PathBuf {
        let test_dir = env::temp_dir().join(dirname);
        fs::create_dir_all(&test_dir).unwrap();
        test_dir
    }

    fn cleanup_test_directory(test_dir: &PathBuf) {
        fs::remove_dir_all(test_dir).expect("Should remove all");
    }

    fn create_test_file(test_dir: &PathBuf, filename: &str) {
        let path = test_dir.join(filename);
        let mut file = File::create(path).expect("Should have created file");
        file.write_all(b"test content").unwrap();
    }

    #[cfg(target_family = "unix")]
    mod unix_tests {
        use super::*;

        #[test]
        fn test_fnmatch_with_existing_files() {
            let test_dir = setup_test_directory("existing_files");
            create_test_file(&test_dir, "test1.txt");
            create_test_file(&test_dir, "test2.txt");
            create_test_file(&test_dir, "other.log");

            let pattern = format!("{}/*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern).unwrap();

            assert_eq!(result.len(), 2);
            assert!(result.contains(&format!("{}/test1.txt", test_dir.to_str().unwrap())));
            assert!(result.contains(&format!("{}/test2.txt", test_dir.to_str().unwrap())));

            cleanup_test_directory(&test_dir);
        }

        #[test]
        fn test_fnmatch_with_no_matches() {
            let test_dir = setup_test_directory("no_matches");
            create_test_file(&test_dir, "test.log");

            let pattern = format!("{}/*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern);

            assert!(result.is_err());

            cleanup_test_directory(&test_dir);
        }

        #[test]
        fn test_fnmatch_with_invalid_pattern() {
            let result = fnmatch("\0invalid");
            assert!(result.is_err());
        }
    }

    #[cfg(target_family = "windows")]
    mod windows_tests {
        use super::*;

        #[test]
        fn test_fnmatch_with_existing_files() {
            let test_dir = setup_test_directory("existing_files");
            create_test_file(&test_dir, "test1.txt");
            create_test_file(&test_dir, "test2.txt");
            create_test_file(&test_dir, "other.log");

            let pattern = format!("{}\\*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern).unwrap();

            assert_eq!(result.len(), 2);
            assert!(result.contains(&format!("{}\\test1.txt", test_dir.to_str().unwrap())));
            assert!(result.contains(&format!("{}\\test2.txt", test_dir.to_str().unwrap())));

            cleanup_test_directory(&test_dir);
        }

        #[test]
        fn test_fnmatch_with_no_matches() {
            let test_dir = setup_test_directory("no_matches");
            create_test_file(&test_dir, "test.log");

            let pattern = format!("{}\\*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern);

            assert!(result.is_err());

            cleanup_test_directory(&test_dir);
        }

        #[test]
        fn test_fnmatch_with_invalid_pattern() {
            let result = fnmatch("\0invalid");
            assert!(result.is_err());
        }
    }
}
