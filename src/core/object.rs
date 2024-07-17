#![allow(dead_code, clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::fs;

use crate::core::GitRepository;
use crate::utils::path::repo_file;
use crate::zlib;

static OBJECTS_DIR: &str = "objects";
static SPACE_BYTE: u8 = b' ';
static NULL_BYTE: u8 = b'\0';

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum GitObject {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl GitObject {
    #[must_use]
    pub fn deserialize(&self, data: &[u8]) {
        match self {
            GitObject::Blob => self.blob_deserialize(data),
            GitObject::Commit => self.commit_deserialize(data),
            GitObject::Tag => self.tag_deserialize(data),
            GitObject::Tree => self.tree_deserialize(data),
        }
    }

    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            GitObject::Blob => self.blob_serialize(),
            GitObject::Commit => self.commit_serialize(),
            GitObject::Tag => self.tag_serialize(),
            GitObject::Tree => self.tree_serialize(),
        }
    }

    pub fn from_raw_data(raw: &[u8]) -> Result<GitObject, String> {
        let total_size = raw.len();
        let mut raw_iter = raw.iter();
        // Read the object format
        let Some(space_idx) = raw_iter.position(|byte| *byte == SPACE_BYTE)
        else {
            return Err("format not specified".to_owned());
        };
        let format = raw[..space_idx].to_vec();

        // Read the object size
        let Some(null_idx) = raw_iter.position(|byte| *byte == 0) else {
            return Err("size not specified".to_owned());
        };
        // Iterator position restarts from 0, add prev offset
        let null_idx = null_idx + space_idx + 1;
        let Ok(size) = String::from_utf8(raw[space_idx..null_idx].to_vec())
        else {
            return Err("invalid size".to_owned());
        };
        let Ok(size) = size.trim().parse::<usize>() else {
            return Err("failed to read size".to_owned());
        };

        // Ensure size matches contents
        if size != (total_size - null_idx - 1) {
            return Err("size mismatch!".to_owned());
        }

        // Create object from data
        match format.as_slice() {
            b"blob" => Ok(Self::blob_from(raw_iter)),
            b"commit" => Ok(Self::commit_from(raw_iter)),
            b"tag" => Ok(Self::tag_from(raw_iter)),
            b"tree" => Ok(Self::tree_from(raw_iter)),
            _ => Err(format!("Unknown format {format:?}")),
        }
    }
}

// This is the implementation for GitObject::Blob
impl GitObject {
    pub fn blob_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        GitObject::Blob
    }

    fn blob_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn blob_deserialize(&self, _data: &[u8]) {
        todo!()
    }
}

// This is the impl for GitObject::Commit
impl GitObject {
    pub fn commit_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        GitObject::Commit
    }

    fn commit_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn commit_deserialize(&self, _data: &[u8]) {
        todo!()
    }
}

// This is the impl for GitObject::Tag
impl GitObject {
    pub fn tag_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        GitObject::Tag
    }

    fn tag_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn tag_deserialize(&self, _data: &[u8]) {
        todo!()
    }
}

// This is the impl for GitObject::Tree
impl GitObject {
    pub fn tree_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        GitObject::Tree
    }

    fn tree_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn tree_deserialize(&self, _data: &[u8]) {
        todo!()
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn read_object(
    repo: &GitRepository,
    sha: &str,
) -> Result<GitObject, String> {
    // Calculate the path to the object
    let path =
        repo_file(repo.gitdir(), &[OBJECTS_DIR, &sha[..2], &sha[2..]], false)?;

    // Ensure the path is a valid file
    let path = match path {
        Some(path) if path.is_file() => path,
        _ => return Err(format!("failed to find object with digest {sha}")),
    };

    // Read and decompress the file
    let Ok(raw) = fs::read(path) else {
        return Err(format!("failed to read object with digest {sha}"));
    };
    let raw = zlib::decompress(&raw)?;
    let res = match GitObject::from_raw_data(&raw) {
        Ok(obj) => obj,
        Err(msg) => {
            return Err(format!("malformed object with digest {sha}, {msg}"))
        }
    };
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::repo_dir;
    use crate::utils::test::TempDir;

    #[test]
    fn test_read_object_bad_path() {
        let tmp_dir = TempDir::create("test_read_object_bad_path");
        let sha = "abcdef09123456789abc";

        let repo = GitRepository::create(tmp_dir.test_dir())
            .expect("Should create repo");

        assert!(read_object(&repo, sha).is_err_and(|msg| msg.contains(sha)));
    }

    #[test]
    fn test_read_object_good_path() {
        let tmp_dir = TempDir::create("test_read_object_bad_path");
        let sha = "abcdef09123456789abc";

        let repo = GitRepository::create(tmp_dir.test_dir())
            .expect("Should create repo");

        let path = repo_dir(repo.gitdir(), &[OBJECTS_DIR, &sha[..2]], true)
            .expect("Should create dir!")
            .expect("Should contain path!");

        let contents = b"tree 0\0";
        let compressed = zlib::compress(contents, &zlib::Strategy::Fixed);

        fs::write(path.join(&sha[2..]), &compressed)
            .expect("Should write contents");

        assert!(read_object(&repo, sha).is_ok_and(|obj| match obj {
            GitObject::Tree => true,
            _ => false,
        }));
    }
}
