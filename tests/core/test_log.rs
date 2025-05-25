#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::commands::log::*;
    use mini_git::core::objects::commit::Commit;
    use mini_git::core::objects::traits::KVLM;
    use mini_git::core::GitRepository;

    use mini_git::utils::collections::kvlm;
    use mini_git::utils::test::TempDir;
    use mini_git::utils::zlib;

    use std::sync::Mutex;

    const RESET: &str = "\x1b[0m";
    const YELLOW: &str = "\x1b[33m";

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);

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

    fn create_commit(kvlm_data: kvlm::KVLM, hash: &str) -> (Vec<u8>, String) {
        let commit = Commit::with_kvlm(kvlm_data);
        let serialized_commit = &commit.serialize();
        let len = serialized_commit.len();
        let mut data = format!("commit {len}\0").as_bytes().to_vec();
        data.extend_from_slice(serialized_commit);
        let compressed = zlib::compress(&data, &zlib::Strategy::Auto);
        (compressed, hash.to_string())
    }

    fn create_temp_repo<'a>() -> TempDir<'a, ()> {
        let tmp = TempDir::create("cmd_log").with_mutex(&crate::TEST_MUTEX);
        let repo = GitRepository::create(tmp.tmp_dir()).expect("Create repo");

        // Create commits manually
        let mut serialized = vec![];

        // Create initial commit
        let kvlm_data = kvlm::KVLM::parse(
            b"author John Doe <john@example.com> 1627890123 +0200
committer John Doe <john@example.com> 1627890123 +0200

Initial commit",
        )
        .expect("Parse");

        let (data_initial, hash_initial) =
            create_commit(kvlm_data, &"a".repeat(40));
        serialized.push((data_initial, hash_initial.clone()));

        // Create second commit
        let kvlm_data = kvlm::KVLM::parse(
            format!(
                "tree {hash_initial}
parent {hash_initial}
author Jane Smith <jane@example.com> 1234567890 +0200
committer Jane Smith <jane@example.com> 1234567890 +0200

Second commit"
            )
            .as_bytes(),
        )
        .expect("Parse");

        let (data_second, hash_second) =
            create_commit(kvlm_data, &"b".repeat(40));
        serialized.push((data_second, hash_second.clone()));

        // Write objects to the .git/objects directory
        let obj_dir = repo.gitdir().join("objects");

        for (data, hash) in serialized {
            let dir = obj_dir.join(&hash[..2]);
            std::fs::create_dir_all(&dir).expect("Should create dir");
            let path = dir.join(&hash[2..]);
            assert!(!path.is_file(), "Setup failed! File already exists");
            std::fs::write(&path, data).expect("Should write");
            assert!(path.is_file(), "Setup failed! File write failed");
        }

        // Update HEAD to point to the latest commit
        std::fs::write(repo.gitdir().join("HEAD"), "ref: refs/heads/master\n")
            .expect("Write HEAD");

        // Create refs/heads/master pointing to the latest commit
        let refs_dir = repo.gitdir().join("refs").join("heads");
        std::fs::create_dir_all(&refs_dir).expect("Create refs/heads");
        std::fs::write(refs_dir.join("master"), format!("{hash_second}\n"))
            .expect("Write master ref");

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
    fn test_log_default() {
        setup();

        let args: [&[&str]; 1] = [&[]]; // No arguments, should use defaults

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            log(&namespace)
        });

        assert!(res.is_ok());
        let output = res.unwrap();
        // Check that the output contains both commits in reverse order
        assert!(output.contains("Second commit"));
        assert!(output.contains("Initial commit"));
        // Ensure "Second commit" comes before "Initial commit"
        let index_second = output.find("Second commit").unwrap();
        let index_initial = output.find("Initial commit").unwrap();
        assert!(index_second < index_initial);
    }

    #[test]
    fn test_log_max_commits() {
        setup();

        let args: [&[&str]; 1] = [&["-n", "1"]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            log(&namespace)
        });

        assert!(res.is_ok());
        let output = res.unwrap();
        // Should only contain the second commit
        assert!(output.contains("Second commit"));
        assert!(!output.contains("Initial commit"));
    }

    #[test]
    fn test_log_oneline() {
        setup();

        let args: [&[&str]; 1] = [&["--oneline"]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            log(&namespace)
        });

        assert!(res.is_ok());
        let output = res.unwrap();
        // Output should be concise
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2); // Two commits
                                    // Each line should contain the short hash and commit message
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert!(parts.len() >= 2);
            // The first part is the short hash
            let short_hash = parts[0].strip_prefix(YELLOW).unwrap_or(parts[0]);
            let short_hash =
                short_hash.strip_suffix(RESET).unwrap_or(short_hash);
            assert_eq!(short_hash.len(), 7, "short_hash = {short_hash:?}"); // Short hash length
        }
    }

    #[test]
    fn test_log_no_author() {
        setup();

        let args: [&[&str]; 1] = [&["--no-author"]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            log(&namespace)
        });

        assert!(res.is_ok());
        let output = res.unwrap();
        // Output should not contain "Author" lines
        assert!(!output.contains("Author:"));
    }

    #[test]
    fn test_log_specific_commit() {
        setup();

        let args: [&[&str]; 1] = [&["--revision", &"a".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            log(&namespace)
        });

        assert!(res.is_ok());
        let output = res.unwrap();
        // Should only contain the initial commit
        assert!(output.contains("Initial commit"));
        assert!(!output.contains("Second commit"));
    }
}
