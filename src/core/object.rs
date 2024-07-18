#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value
)]

use std::fs;

use crate::core::GitRepository;
use crate::utils::path::repo_file;
use crate::utils::sha1::SHA1;
use crate::zlib;

static OBJECTS_DIR: &str = "objects";
static SPACE_BYTE: u8 = b' ';
static NULL_BYTE: u8 = b'\0';

pub type BlobData = Vec<u8>;

/// Represents one of the four types of objects git uses
/// - blobs
/// - commits
/// - tags
/// - trees
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum GitObject {
    Blob(BlobData),
    Commit,
    Tag,
    Tree,
}

use GitObject::{Blob, Commit, Tag, Tree};

// This is the common implementation for GitObject
// The functions defined here are basically dispatch functions that choose the
// required implementation based on the enum variant
impl GitObject {
    /// Deserializes raw data to create a `GitObject`.
    ///
    /// This method is a dispatches the deserialzer based on the current variant,
    /// it needs to be called on the variant that the expected object is.
    ///
    /// # Panics
    ///
    /// Deserialization may fails if,
    /// - Raw data was malformed
    /// - It was attempted on an object of the wrong kind
    ///
    /// # Example
    /// ```
    /// use mini_git::core::object::{GitObject::*, BlobData};
    /// let data = b"Hello world!";
    ///
    /// // This call to deserialize will create a blob
    /// let blob = Blob(BlobData::new()).deserialize(data);
    /// println!("{blob:?}");
    /// ```
    #[must_use]
    pub fn deserialize(&self, data: &[u8]) -> GitObject {
        match self {
            Blob(_) => Self::blob_deserialize(data),
            Commit => Self::commit_deserialize(data),
            Tag => Self::tag_deserialize(data),
            Tree => Self::tree_deserialize(data),
        }
    }

    /// Serializes the `GitObject` to create a raw data object representation
    /// for the object.
    ///
    /// # Example
    /// ```
    /// use mini_git::core::object::{GitObject::*, BlobData};
    /// let data = b"Hello world!";
    ///
    /// // This call to deserialize will create a blob
    /// let blob = Blob(BlobData::from(data));
    ///
    /// let serialized = blob.serialize();
    /// println!("{serialized:?}");
    /// ```
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            Blob(_) => self.blob_serialize(),
            Commit => self.commit_serialize(),
            Tag => self.tag_serialize(),
            Tree => self.tree_serialize(),
        }
    }

    /// Returns the object format for the current `GitObject`
    ///
    /// # Example
    /// ```
    /// use mini_git::core::object::GitObject;
    ///
    /// let commit = GitObject::Commit;
    /// assert_eq!(commit.format(), b"commit");
    ///
    /// let tag = GitObject::Tag;
    /// assert_eq!(tag.format(), b"tag");
    /// ```
    #[must_use]
    pub const fn format(&self) -> &'static [u8] {
        match self {
            Blob(_) => Self::blob_format(),
            Commit => Self::commit_format(),
            Tag => Self::tag_format(),
            Tree => Self::tree_format(),
        }
    }

    /// Builds a GitObject from raw data, typically used with the
    /// decompressed contents from `".git/objects/.."`
    ///
    /// Unlike [`GitObject::deserialize`], which deserializes based on a given
    /// object variant, this method can determine the type of the object
    /// from the contents of the raw data.
    ///
    /// # Errors
    /// This method may fail if the raw data was malformed. A error message
    /// describing the failure is returned
    ///
    /// # Example
    /// ```no_run
    /// use mini_git::core::object::{GitObject, BlobData};
    /// use GitObject::*;
    ///
    /// let data = b"blob 5\0hello";
    ///
    /// let blob = GitObject::from_raw_data(data)?;
    /// let Blob(blob_data) = blob else {panic!("uh oh, unexpected object")};
    /// assert_eq!(blob_data, data);
    ///
    /// # Ok::<(), String>(())
    /// ```
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
        Blob(Vec::new())
    }

    const fn blob_format() -> &'static [u8] {
        const FORMAT: &[u8] = b"blob";
        FORMAT
    }

    fn blob_serialize(&self) -> Vec<u8> {
        let Blob(data) = self else {
            unreachable!();
        };
        data.clone()
    }

    fn blob_deserialize(data: &[u8]) -> GitObject {
        Blob(BlobData::from(data))
    }
}

// This is the impl for GitObject::Commit
impl GitObject {
    pub fn commit_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        Commit
    }

    const fn commit_format() -> &'static [u8] {
        const FORMAT: &[u8] = b"commit";
        FORMAT
    }

    fn commit_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn commit_deserialize(_data: &[u8]) -> GitObject {
        todo!()
    }
}

// This is the impl for GitObject::Tag
impl GitObject {
    pub fn tag_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        Tag
    }

    const fn tag_format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tag";
        FORMAT
    }

    fn tag_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn tag_deserialize(_data: &[u8]) -> GitObject {
        todo!()
    }
}

// This is the impl for GitObject::Tree
impl GitObject {
    pub fn tree_from<'a>(_iter: impl Iterator<Item = &'a u8>) -> GitObject {
        Tree
    }

    const fn tree_format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tree";
        FORMAT
    }

    fn tree_serialize(&self) -> Vec<u8> {
        todo!()
    }
    fn tree_deserialize(_data: &[u8]) -> GitObject {
        todo!()
    }
}

#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn find_object(
    _repo: &GitRepository,
    name: &str,
    _format: Option<&str>,
    _follow: bool,
) -> String {
    name.to_owned()
}

/// Reads an object from the given repository with the given SHA digest
///
/// # Errors
/// This function may fail if,
/// - Request object does not exist
/// - I/O errors occur while reading object files
/// - Object files are corrupted/malformed
///
/// Example
/// ```no_run
/// use std::path::Path;
/// use mini_git::core::GitRepository;
/// use mini_git::core::object::read_object;
///
/// // This is an example digest (highly unlikely digest)
/// let digest = "deadbeefdecadedefacecafec0ffeedadfacade8";
/// // Get current repository
/// let repo = GitRepository::new(Path::new("."))?;
///
/// let obj = read_object(&repo, &digest)?;
/// println!("{obj:?}");
/// # Ok::<(), String>(())
/// ```
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

/// Creates a object Hash from an object
///
/// This function returns a tuple of two values
/// - The contents over which the hash was built
/// - The SHA1 object built from the contents
///
/// Example
/// ```
/// use mini_git::core::object::{hash_object, GitObject};
/// use GitObject::*;
///
/// let obj = Blob(vec![]);
/// let (contents, mut hash) = hash_object(&obj);
/// assert_eq!(contents, b"blob 0\0");
/// let digest = hash.hex_digest();
/// assert_eq!(digest, "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
/// ```
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn hash_object(obj: &GitObject) -> (Vec<u8>, SHA1) {
    let data = obj.serialize();
    let len = data.len().to_string();
    let len = len.as_bytes();
    let res = [obj.format(), &[SPACE_BYTE], len, &[NULL_BYTE], &data].concat();

    let mut hash = SHA1::new();
    let _ = hash.update(&res);

    (res, hash)
}

/// Writes an object to the repository files
///
/// # Returns
/// The sha1 hex-digest of the object written.
///
/// ## Note
/// This function will **never** overwrite the contents of the
/// file if it already exists.
///
/// Example
/// ```no_run
/// use std::path::Path;
/// use mini_git::core::GitRepository;
/// use mini_git::core::object::{write_object, GitObject};
/// use GitObject::*;
///
/// let obj = Blob(vec![]);
/// let repo = GitRepository::new(Path::new("."))?;
/// let digest = write_object(&obj, &repo)?;
/// assert_eq!(digest, "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
/// Ok::<(), String>(())
/// ```
#[allow(clippy::module_name_repetitions)]
pub fn write_object(
    obj: &GitObject,
    repo: &GitRepository,
) -> Result<String, String> {
    let (res, mut hash) = hash_object(obj);

    let digest = hash.hex_digest();

    let path = repo_file(
        repo.gitdir(),
        &[OBJECTS_DIR, &digest[..2], &digest[2..]],
        true,
    )?;
    let Some(path) = path else {
        return Err(format!(
            "Failed to create object file for digest {digest}"
        ));
    };

    if !path.exists() {
        let compressed = zlib::compress(&res, &zlib::Strategy::Auto);
        if fs::write(&path, compressed).is_err() {
            return Err(format!(
                "Failed to write to file {:?}",
                path.as_os_str()
            ));
        };
    }

    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::repo_dir;
    use crate::utils::test::TempDir;
    use GitObject::{Blob, Commit, Tag, Tree};

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
        let sha = "deadbeefdecadedefacecafec0ffeedadfacade8";

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
            Tree => true,
            _ => false,
        }));
    }

    #[test]
    #[ignore = "WIP"]
    fn test_hash_object() {
        let objects = [Blob(BlobData::new()), Commit, Tag, Tree];

        for obj in objects {
            let mut expected_hash = SHA1::new();
            let expected_hash = expected_hash
                .update(obj.format())
                .update(b" ")
                .update(b"20")
                .update(b"\0")
                .update(&b"0".repeat(20))
                .hex_digest();

            let (_, mut actual_hash) = hash_object(&obj);
            let actual_hash = actual_hash.hex_digest();

            assert_eq!(expected_hash, actual_hash);
        }
    }

    #[test]
    fn test_write_object_blob() {
        let tmp_dir = TempDir::create("test_read_object_bad_path");

        let repo = GitRepository::create(tmp_dir.test_dir())
            .expect("Should create repo");

        let blob_data = [0; 100];
        let blob = Blob((&blob_data).to_vec());

        let digest = write_object(&blob, &repo).expect("Should write object");

        let file = repo_file(
            repo.gitdir(),
            &[OBJECTS_DIR, &digest[..2], &digest[2..]],
            false,
        )
        .expect("Should have been created")
        .expect("Should be a file");
        let raw = fs::read(&file).expect("Should read file");
        let decompressed =
            zlib::decompress(&raw).expect("Should decompress correctly");

        assert_eq!(&decompressed[..4], b"blob");
        assert_eq!(decompressed[4], SPACE_BYTE);
        assert_eq!(&decompressed[5..8], b"100");
        assert_eq!(decompressed[8], NULL_BYTE);
        assert_eq!(&decompressed[9..], &blob_data);
    }

    #[test]
    #[ignore = "WIP"]
    fn test_write_object_commit() {
        unimplemented!()
    }

    #[test]
    #[ignore = "WIP"]
    fn test_write_object_tag() {
        unimplemented!()
    }

    #[test]
    #[ignore = "WIP"]
    fn test_write_object_tree() {
        unimplemented!()
    }

    #[test]
    fn test_blob_serialize() {
        let data = &[0; 16];
        let blob = Blob(BlobData::from(data));
        let serialized = blob.blob_serialize();
        assert_eq!(&serialized, data);
    }

    #[test]
    fn test_blob_deserialize() {
        let data = &[0; 16];
        match GitObject::blob_deserialize(data) {
            Blob(inner) => assert_eq!(inner, data),
            _ => panic!("Deserialize did not return a blob"),
        }
    }
}
