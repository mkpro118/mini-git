#[cfg(test)]
mod tests {
    use mini_git::utils::fnmatch::fnmatch;
    #[cfg(test)]
    use mini_git::utils::test::TempDir;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    fn create_test_file(test_dir: &Path, filename: &str) {
        let path = test_dir.join(filename);
        let mut file = File::create(path).expect("Should have created file");
        file.write_all(b"test content").unwrap();
    }

    #[cfg(target_family = "unix")]
    mod unix_tests {
        use super::*;

        #[test]
        fn test_fnmatch_with_existing_files() {
            let tmp_dir = TempDir::create("existing_files");
            let test_dir = tmp_dir.test_dir();
            create_test_file(test_dir, "test1.txt");
            create_test_file(test_dir, "test2.txt");
            create_test_file(test_dir, "other.log");

            let pattern = format!("{}/*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern).unwrap();

            assert_eq!(result.len(), 2);
            assert!(result.contains(&format!(
                "{}/test1.txt",
                test_dir.to_str().unwrap()
            )));
            assert!(result.contains(&format!(
                "{}/test2.txt",
                test_dir.to_str().unwrap()
            )));
        }

        #[test]
        fn test_fnmatch_with_no_matches() {
            let tmp_dir = TempDir::create("no_matches");
            let test_dir = tmp_dir.test_dir();
            create_test_file(test_dir, "test.log");

            let pattern = format!("{}/*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern);

            assert!(result.is_err());
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
            let tmp_dir = TempDir::create("existing_files");
            let test_dir = tmp_dir.test_dir();
            create_test_file(test_dir, "test1.txt");
            create_test_file(test_dir, "test2.txt");
            create_test_file(test_dir, "other.log");

            let pattern = format!("{}\\*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern).unwrap();

            assert_eq!(result.len(), 2);
            assert!(result.contains(&format!(
                "{}\\test1.txt",
                test_dir.to_str().unwrap()
            )));
            assert!(result.contains(&format!(
                "{}\\test2.txt",
                test_dir.to_str().unwrap()
            )));
        }

        #[test]
        fn test_fnmatch_with_no_matches() {
            let tmp_dir = TempDir::create("no_matches");
            let test_dir = tmp_dir.test_dir();
            create_test_file(test_dir, "test.log");

            let pattern = format!("{}\\*.txt", test_dir.to_str().unwrap());
            let result = fnmatch(&pattern);

            assert!(result.is_err());
        }

        #[test]
        fn test_fnmatch_with_invalid_pattern() {
            let result = fnmatch("\0invalid");
            assert!(result.is_err());
        }
    }
}
