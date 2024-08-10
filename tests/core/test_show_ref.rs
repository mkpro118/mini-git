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
                *inner = Some(create_mock_repo());
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        };
    }

    fn create_mock_repo() -> TempDir<'static, ()> {
        let tmp =
            TempDir::create("cmd_show_ref").with_mutex(&crate::TEST_MUTEX);
        let _ = GitRepository::create(tmp.tmp_dir()).expect("Create repo");
        tmp.switch();
        tmp.run(|| {
            create_heads();
            create_tags();
            create_remotes();
        });
        tmp
    }

    fn create_heads() {
        let top = REFS_DIR().join("heads");
        fs::create_dir(&top).expect("create dir");
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
        fs::create_dir(&top).expect("create dir");
        let obj_top = OBJECT_DIR().join("ab");
        fs::create_dir(&obj_top).expect("create dir");

        let tag_kvlm =
            kvlm::KVLM::parse(format!("object {}", "12".repeat(20)).as_bytes())
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
            fs::create_dir_all(&top).expect("create dir");

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
    }
}
