#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::objects::tag::Tag;
    use mini_git::core::objects::traits::KVLM;
    use mini_git::core::show_ref::*;
    use mini_git::core::GitRepository;

    use mini_git::utils::collections::kvlm;
    use mini_git::utils::test::TempDir;

    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);
    static REFS_DIR: fn() -> PathBuf =
        || std::env::current_dir().unwrap().join(".git").join("refs");

    static OBJECT_DIR: fn() -> PathBuf = || {
        std::env::current_dir()
            .unwrap()
            .join(".git")
            .join("objects")
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

    fn setup() {
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                println!("Allowing setup!");
                *inner = Some(create_mock_repo());
            }
            Ok(..) => {
                println!("Bypassing setup!");
            }
            Err(..) => panic!("Mutex failed!"),
        };
    }

    fn create_mock_repo() -> TempDir<'static, ()> {
        let tmp =
            TempDir::create("cmd_show_ref").with_mutex(&crate::TEST_MUTEX);
        println!("temp = {:?}", tmp.tmp_dir());
        let _ = GitRepository::create(tmp.tmp_dir()).expect("Create repo");
        tmp.switch();
        tmp.run(|| {
            create_heads();
            create_tags();
            create_remotes();
        });
        tmp
    }

    fn create_dir_if_not_exists(path: &std::path::Path) {
        if !path.exists() {
            fs::create_dir_all(path)
                .unwrap_or_else(|_| panic!("create dir {:?}", path));
            return;
        }
        assert!(path.is_dir(), "Invalid test state!");
    }

    fn create_heads() {
        let top = REFS_DIR().join("heads");
        create_dir_if_not_exists(&top);

        for res in [
            fs::write(top.join("main"), format!("{}\n", "0".repeat(40))),
            fs::write(top.join("feature1"), format!("{}\n", "1".repeat(40))),
            fs::write(top.join("feature2"), b"ref: refs/heads/feature1\n"),
        ] {
            res.expect("Should write to refs/heads");
        }
    }

    fn create_tags() {
        let top = REFS_DIR().join("tags");
        create_dir_if_not_exists(&top);
        let obj_top = OBJECT_DIR().join("ab");
        create_dir_if_not_exists(&obj_top);

        let tag_kvlm = kvlm::KVLM::parse(
            format!("object {}\n\n", "12".repeat(20)).as_bytes(),
        )
        .expect("parsed kvlm");
        let tag = Tag::with_kvlm(tag_kvlm).serialize();

        for res in [
            fs::write(obj_top.join("ab".repeat(19)), tag),
            fs::write(top.join("v1"), format!("{}\n", "ab".repeat(20))),
        ] {
            res.expect("Should write to refs/tags");
        }
    }

    fn create_remotes() {
        let mut iter = ["5", "6", "7", "8"].into_iter();
        for top in ["origin", "develop"] {
            let top = REFS_DIR().join("remotes").join(top);
            create_dir_if_not_exists(&top);

            for res in [
                fs::write(
                    top.join("main"),
                    format!("{}\n", iter.next().unwrap().repeat(40)),
                ),
                fs::write(
                    top.join("feature1"),
                    format!("{}\n", iter.next().unwrap().repeat(40)),
                ),
            ] {
                res.expect("Should write to refs/remotes");
            }
        }
    }

    #[test]
    fn test_show_ref_basic() {
        setup();
        let args: [&[&str]; 1] = [&[]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            dbg!(&namespace);
            show_ref(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("OUTPUT=\n{:?}", &output);
        assert!(output.contains("refs/heads/main"));
        assert!(output.contains("refs/heads/feature1"));
        assert!(output.contains("refs/heads/feature2"));
        assert!(output.contains("refs/tags/v1"));
        assert!(output.contains("refs/remotes/origin/main"));
        assert!(output.contains("refs/remotes/origin/feature1"));
        assert!(output.contains("refs/remotes/develop/main"));
        assert!(output.contains("refs/remotes/develop/feature1"));
    }

    #[test]
    fn test_show_ref_heads() {
        setup();
        let args: [&[&str]; 1] = [&["--heads"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            show_ref(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("refs/heads/main"));
        assert!(output.contains("refs/heads/feature1"));
        assert!(output.contains("refs/heads/feature2"));
        assert!(!output.contains("refs/tags/v1"));
        assert!(!output.contains("refs/remotes/origin/main"));
        assert!(!output.contains("refs/remotes/origin/feature1"));
        assert!(!output.contains("refs/remotes/develop/main"));
        assert!(!output.contains("refs/remotes/develop/feature1"));
    }

    #[test]
    fn test_show_ref_tags() {
        setup();
        let args: [&[&str]; 1] = [&["--tags"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            show_ref(&namespace)
        });

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("refs/heads/main"));
        assert!(!output.contains("refs/heads/feature1"));
        assert!(!output.contains("refs/heads/feature2"));
        assert!(output.contains("refs/tags/v1"));
        assert!(!output.contains("refs/remotes/origin/main"));
        assert!(!output.contains("refs/remotes/origin/feature1"));
        assert!(!output.contains("refs/remotes/develop/main"));
        assert!(!output.contains("refs/remotes/develop/feature1"));
    }

    #[test]
    fn test_show_ref_remotes() {
        setup();
        let args: [&[&str]; 1] = [&["--heads"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            show_ref(&namespace)
        });

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("refs/heads/main"));
        assert!(output.contains("refs/heads/feature1"));
        assert!(output.contains("refs/heads/feature2"));
        assert!(!output.contains("refs/remotes/origin/main"));
        assert!(!output.contains("refs/remotes/origin/feature1"));
        assert!(!output.contains("refs/remotes/develop/main"));
        assert!(!output.contains("refs/remotes/develop/feature1"));
    }

    #[test]
    fn test_show_ref_exists() {
        setup();
        let args: [&[&str]; 1] = [&["--exists", "refs/heads/main"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            show_ref(&namespace)
        });

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.is_empty()); // --exists flag implies no output if ref exists
    }

    #[test]
    fn test_show_ref_not_exists() {
        setup();
        let args: [&[&str]; 1] = [&["--exists", "refs/heads/nonexistent"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            show_ref(&namespace)
        });
        assert!(result.is_err());
    }
}
