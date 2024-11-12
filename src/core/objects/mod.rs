pub mod blob;
pub mod commit;
pub mod packfiles;
pub mod tag;
pub mod traits;
pub mod tree;
pub mod worktree;

use std::fs;
use std::path::Path;

use crate::core::GitRepository;
use crate::utils::collections::ordered_map::OrderedMap;
use crate::utils::hex;
use crate::utils::path;
use crate::utils::sha1;
use crate::utils::zlib;
use traits::{Deserialize, Format, Serialize, KVLM};

static OBJECTS_DIR: &str = "objects";
static SPACE_BYTE: u8 = b' ';
static NULL_BYTE: u8 = b'\0';

/// Represents one of the four types of objects git uses
/// - blobs
/// - commits
/// - tags
/// - trees
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum GitObject {
    Blob(blob::Blob),
    Commit(commit::Commit),
    Tag(tag::Tag),
    Tree(tree::Tree),
}

use GitObject::{Blob, Commit, Tag, Tree};

// This is the common implementation for GitObject
// The functions defined here are basically dispatch functions that choose the
// required implementation based on the enum variant
impl GitObject {
    /// Deserializes raw data to create a `GitObject`.
    ///
    /// This method dispatches the deserializer based on the current variant.
    ///
    /// # Errors
    ///
    /// Deserialization may fail if:
    /// - Raw data is malformed.
    /// - It's attempted on an object of the wrong kind.
    ///
    /// A [`String`] describing the error message is returned.
    ///
    /// # Example
    /// ```
    /// use mini_git::core::objects::{GitObject::*, blob};
    /// let data = b"Hello world!";
    ///
    /// // This call to deserialize will create a blob
    /// let blob = Blob(blob::Blob::default());
    /// let blob = blob.deserialize(data);
    /// println!("{blob:?}");
    /// ```
    pub fn deserialize(&self, data: &[u8]) -> Result<GitObject, String> {
        Ok(match self {
            Blob(..) => Blob(blob::Blob::deserialize(data)?),
            Commit(..) => Commit(commit::Commit::deserialize(data)?),
            Tag(..) => Tag(tag::Tag::deserialize(data)?),
            Tree(..) => Tree(tree::Tree::deserialize(data)?),
        })
    }

    /// Serializes the `GitObject` to create a raw data object representation
    /// for the object.
    ///
    /// # Example
    /// ```
    /// use mini_git::core::objects::{GitObject::*, blob};
    /// use mini_git::core::objects::traits::Serialize;
    /// let data = b"Hello world!";
    ///
    /// // This call to deserialize will create a blob
    /// let blob = Blob(blob::Blob::from(data.as_slice()));
    ///
    /// let serialized = blob.serialize();
    /// println!("{serialized:?}");
    /// ```
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            Blob(blob) => blob.serialize(),
            Commit(commit) => commit.serialize(),
            Tag(tag) => tag.serialize(),
            Tree(tree) => tree.serialize(),
        }
    }

    /// Returns the object format for the current `GitObject`
    ///
    /// # Example
    /// ```
    /// use mini_git::core::objects::{GitObject, blob, commit};
    ///
    /// let blob = GitObject::Blob(blob::Blob::default());
    /// assert_eq!(blob.format(), b"blob");
    ///
    /// let commit = GitObject::Commit(commit::Commit::default());
    /// assert_eq!(commit.format(), b"commit");
    /// ```
    #[must_use]
    pub fn format(&self) -> &'static [u8] {
        match self {
            Blob(_) => blob::Blob::format(),
            Commit(_) => commit::Commit::format(),
            Tag(..) => tag::Tag::format(),
            Tree(..) => tree::Tree::format(),
        }
    }

    /// Builds a `GitObject` from raw data, typically used with the
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
    /// use mini_git::core::objects::GitObject;
    /// use GitObject::*;
    ///
    /// let data = b"blob 5\0hello";
    ///
    /// let blob = GitObject::from_raw_data(data)?;
    /// let Blob(blob) = blob else {panic!("uh oh, unexpected object")};
    /// assert_eq!(blob.data(), data);
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

        let raw = &raw[(null_idx + 1)..];

        // Create object from data
        match format.as_slice() {
            b"blob" => Ok(Blob(blob::Blob::deserialize(raw)?)),
            b"commit" => Ok(Commit(commit::Commit::deserialize(raw)?)),
            b"tag" => Ok(Tag(tag::Tag::deserialize(raw)?)),
            b"tree" => Ok(Tree(tree::Tree::deserialize(raw)?)),
            _ => Err(format!("Unknown format {format:?}")),
        }
    }
}

/// Resolves a Git reference to an object ID.
///
/// This function attempts to resolve a given reference (e.g., `"HEAD"`, `"refs/heads/main"`)
/// to an object ID (commit hash) within the specified `GitRepository`.
///
/// # Arguments
///
/// * `repo` - A reference to the `GitRepository` where the reference should be resolved.
/// * `r#ref` - The name of the reference to resolve.
///
/// # Returns
///
/// * `Ok(Some(object_id))` - If the reference is successfully resolved to an object ID.
/// * `Ok(None)` - If the reference does not exist.
/// * `Err(error_message)` - If an error occurs during resolution.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The reference file cannot be found or accessed.
/// * Reading the reference file fails.
/// * An I/O error occurs while accessing the filesystem.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use mini_git::core::objects::resolve_ref;
/// use mini_git::core::GitRepository;
/// let repo = GitRepository::new(&Path::new("."))?;
/// let object_id = resolve_ref(&repo, "HEAD")?;
/// if let Some(oid) = object_id {
///     println!("Resolved object ID: {}", oid);
/// } else {
///     println!("Reference does not exist.");
/// }
/// # Ok::<(), String>(())
/// ```
pub fn resolve_ref(
    repo: &GitRepository,
    r#ref: &str,
) -> Result<Option<String>, String> {
    let Some(path) = path::repo_file(repo.gitdir(), &[r#ref], false)? else {
        unreachable!();
    };

    if !path.is_file() {
        // If not found, try packed-refs
        return resolve_ref_packed(repo, r#ref);
    }

    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Err(format!("Failed to read file at {:?}", path.as_os_str()));
    };

    let contents = contents.trim();
    if let Some(stripped) = contents.strip_prefix("ref: ") {
        resolve_ref(repo, stripped)
    } else {
        Ok(Some(contents.to_owned()))
    }
}

fn resolve_ref_packed(
    repo: &GitRepository,
    r#ref: &str,
) -> Result<Option<String>, String> {
    let packed_refs = parse_packed_refs(repo)?;
    Ok(packed_refs.get(&r#ref.to_owned()).cloned())
}

/// Parses the `packed-refs` file in the specified `GitRepository`.
///
/// # Arguments
///
/// * `repo` - A reference to the `GitRepository` where the `packed-refs` file should be parsed.
///
/// # Errors
///
/// This function will return an error if:
///
/// * Reading the reference file fails.
/// * An I/O error occurs while accessing the filesystem.
///
pub(super) fn parse_packed_refs(
    repo: &GitRepository,
) -> Result<OrderedMap<String, String>, String> {
    const COMMENT_CHAR: char = '#';
    const PEELED_TAG_CHAR: char = '^';

    let packed_refs_path = repo.gitdir().join("packed-refs");
    if !packed_refs_path.exists() {
        return Ok(OrderedMap::new());
    }
    let contents = std::fs::read_to_string(&packed_refs_path)
        .map_err(|_| "Failed to read packed-refs file".to_owned())?;

    let mut lines = contents.lines().map(str::trim).peekable();
    let mut res = OrderedMap::new();

    while let Some(line) = lines.next() {
        if line.is_empty() || line.starts_with(COMMENT_CHAR) {
            continue; // Skip empty lines and comments
        }

        // Lines starting with '^' are peeled lines; they are associated with the previous ref
        if line.starts_with(PEELED_TAG_CHAR) {
            continue; // We'll handle peeled lines after matching the ref
        }

        // Parse the SHA and ref name
        let mut tokens = line.split_whitespace();
        let Some(sha) = tokens.next() else {
            continue; // Skip invalid lines
        };
        let Some(refname) = tokens.next() else {
            continue; // Skip invalid lines
        };

        // Handle peeled lines
        let mut peeled_sha = None;
        while let Some(&next_line) = lines.peek() {
            if next_line.starts_with(PEELED_TAG_CHAR) {
                let Some(line) = lines.next() else {
                    unreachable!();
                };
                peeled_sha =
                    Some(line.trim_start_matches(PEELED_TAG_CHAR).to_owned());
            } else {
                break;
            }
        }

        // Decide which SHA to use
        let final_sha = if let Some(peeled_sha) = peeled_sha {
            peeled_sha
        } else {
            sha.to_owned()
        };

        res.insert(refname.to_owned(), final_sha);
    }
    Ok(res)
}

/// Resolves a Git reference to an object ID.
///
/// This function attempts to resolve a given reference (e.g., `"HEAD"`, `"refs/heads/main"`)
/// to an object ID (commit hash) within the specified `GitRepository`.
///
/// # Arguments
///
/// * `repo` - A reference to the `GitRepository` where the reference should be resolved.
/// * `name` - The name of the reference to resolve.
///
/// # Returns
///
/// * `Ok(Vec<String>)` - A vector of object IDs that match the given reference.
/// * `Err(error_message)` - If an error occurs during resolution.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The reference file cannot be found or accessed.
/// * Reading the reference file fails.
/// * An I/O error occurs while accessing the filesystem.
///
fn resolve_object(
    repo: &GitRepository,
    name: &str,
) -> Result<Vec<String>, String> {
    let mut candidates = Vec::new();

    // Handle the "HEAD" reference
    if name == "HEAD" {
        if let Some(oid) = resolve_ref(repo, name)? {
            candidates.push(oid);
            return Ok(candidates);
        }
        return Err("Could not find HEAD".to_owned());
    }

    // Check for a hex string (short or full hash)
    if name.len() >= 4 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        // Check loose objects
        let prefix = &name[..2];
        let remainder = &name[2..];
        if let Some(path) =
            path::repo_dir(repo.gitdir(), &["objects", prefix], false)?
        {
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with(remainder) {
                    candidates.push(format!("{prefix}{file_name}"));
                }
            }
        }
    }

    // Then check packfiles
    if let Ok(packfiles) = packfiles::find_packfiles(repo) {
        for packfile in packfiles {
            if let Some(full_hash) = packfile.find_object_with_prefix(name) {
                candidates.push(full_hash);
            }
        }
    }

    // Check for tags
    if let Some(tag_ref) = resolve_ref(repo, &format!("refs/tags/{name}"))? {
        candidates.push(tag_ref);
    }

    // Check for branches
    if let Some(branch_ref) = resolve_ref(repo, &format!("refs/heads/{name}"))?
    {
        candidates.push(branch_ref);
    }

    Ok(candidates)
}

/// Finds an object in the repository.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The reference file cannot be found or accessed.
/// * Reading the reference file fails.
/// * An I/O error occurs while accessing the filesystem.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use mini_git::core::objects::find_object;
/// use mini_git::core::GitRepository;
/// let repo = GitRepository::new(&Path::new("."))?;
///
/// let object_name = "HEAD";
/// if let Ok(name) = find_object(&repo, object_name, None, false) {
///     println!("Resolved object name: {}", name);
/// } else {
///     println!("Reference does not exist.");
/// }
/// # Ok::<(), String>(())
/// ```
pub fn find_object(
    repo: &GitRepository,
    name: &str,
    format: Option<&str>,
    follow: bool,
) -> Result<String, String> {
    let candidates = resolve_object(repo, name)?;

    if candidates.is_empty() {
        return Err(format!("No such reference {name}"));
    }

    if candidates.len() > 1 {
        let candidates_str = candidates.join("\n - ");
        return Err(format!(
            "Ambiguous reference {name}: Candidates are:\n - {candidates_str}"
        ));
    }

    let object_id = candidates[0].clone();

    if let Some(obj_format) = format {
        let mut sha = object_id;
        loop {
            let obj = read_object(repo, &sha)?;
            if obj.format() == obj_format.as_bytes() {
                return Ok(sha);
            }

            if !follow {
                return Ok(sha);
            }

            // Follow tags
            if obj.format() == b"tag" {
                sha = String::from_utf8_lossy(&obj.serialize()[8..28])
                    .to_string();
            } else if obj.format() == b"commit" && obj_format == "tree" {
                sha = String::from_utf8_lossy(&obj.serialize()[12..32])
                    .to_string();
            } else {
                return Ok(sha);
            }
        }
    } else {
        Ok(object_id)
    }
}

#[allow(clippy::module_name_repetitions)]
fn read_loose_object(
    repo: &GitRepository,
    sha: &str,
) -> Result<GitObject, String> {
    // Calculate the path to the object
    let path = path::repo_file(
        repo.gitdir(),
        &[OBJECTS_DIR, &sha[..2], &sha[2..]],
        false,
    )?;

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
    let res = GitObject::from_raw_data(&raw)
        .map_err(|msg| format!("malformed object with digest {sha}, {msg}"))?;
    Ok(res)
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
/// use mini_git::core::objects::read_object;
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
pub fn read_object(
    repo: &GitRepository,
    sha: &str,
) -> Result<GitObject, String> {
    if sha.len() > 40 {
        return Err(format!("Invalid SHA digest: {sha}"));
    }

    // Try reading from loose objects first
    let loose_result = read_loose_object(repo, sha);
    if loose_result.is_ok() {
        return loose_result;
    }

    // Convert hex sha to bytes
    let hash = {
        let decoded = hex::decode(sha)
            .map_err(|_| format!("Invalid SHA digest: {sha}"))?;
        let mut buf = [0u8; 20];
        buf[..decoded.len()].copy_from_slice(&decoded);
        buf
    };

    // Try reading from packfiles
    let Ok(packfiles) = packfiles::find_packfiles(repo) else {
        return Err(format!("Object {sha} not found in repository"));
    };

    for mut packfile in packfiles {
        let object = packfile.read_object(&hash);
        if object.is_ok() {
            return object;
        }
    }

    Err(format!("Object {sha} not found in repository"))
}

/// Creates a object Hash from an object
///
/// This function returns a tuple of two values
/// - The contents over which the hash was built
/// - The SHA1 object built from the contents
///
/// Example
/// ```
/// use mini_git::core::objects::{hash_object, GitObject, blob};
/// use GitObject::*;
///
/// let obj = Blob(blob::Blob::default());
/// let (contents, mut hash) = hash_object(&obj);
/// assert_eq!(contents, b"blob 0\0");
/// let digest = hash.hex_digest();
/// assert_eq!(digest, "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
/// ```
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn hash_object(obj: &GitObject) -> (Vec<u8>, sha1::SHA1) {
    let data = obj.serialize();
    let len = data.len().to_string();
    let res = [
        obj.format(),
        &[SPACE_BYTE],
        len.as_bytes(),
        &[NULL_BYTE],
        &data,
    ]
    .concat();

    let mut hash = sha1::SHA1::new();
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
/// # Errors
/// This function may fail if,
/// - Repository does not exist
/// - I/O errors occur while writing object files
///
/// Example
/// ```no_run
/// use std::path::Path;
/// use mini_git::core::GitRepository;
/// use mini_git::core::objects::{write_object, GitObject, blob};
/// use GitObject::*;
///
/// let obj = Blob(blob::Blob::default());
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

    let path = path::repo_file(
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
        fs::write(&path, compressed).map_err(|_| {
            format!("Failed to write to file {:?}", path.as_os_str())
        })?;
    }

    Ok(digest)
}

/// Represents the source of a file, either from a Git blob or the working tree.
#[derive(Debug)]
pub enum FileSource {
    /// A file stored in a Git blob, with a specific path and SHA identifier.
    Blob { path: String, sha: String },

    /// A file located in the working tree with a specified path.
    Worktree { path: String },
}

impl FileSource {
    /// Retrieves the contents of the file source.
    ///
    /// # Arguments
    ///
    /// * `repo` - Reference to the Git repository.
    ///
    /// # Returns
    ///
    /// A `Result` containing the file contents as a vector of bytes if successful,
    /// or an error message if reading the contents failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///   - For `Blob` sources, the object is not a blob.
    ///   - For `Worktree` sources, the file could not be read from the filesystem.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mini_git::core::{RepositoryContext, resolve_repository_context};
    /// use mini_git::core::objects::FileSource;
    ///
    /// let RepositoryContext { repo, .. } = resolve_repository_context()?;
    ///
    /// let file_source = FileSource::Blob { path: "file.txt".to_string(), sha: "abc123".to_string() };
    /// let contents = file_source.contents(&repo)?;
    ///
    /// # Ok::<(), String>(())
    /// ```
    pub fn contents(&self, repo: &GitRepository) -> Result<Vec<u8>, String> {
        Ok(match self {
            FileSource::Blob { sha, .. } => match read_object(repo, sha)? {
                GitObject::Blob(blob) => blob.data,
                x => {
                    return Err(format!(
                        "Expect object {sha} to be a blob, but was {}",
                        String::from_utf8_lossy(x.format())
                    ))
                }
            },
            FileSource::Worktree { path } => match fs::read(path) {
                Ok(data) => data,
                Err(e) => {
                    return Err(format!(
                        "Failed to read file {path}! Error: {e}"
                    ))
                }
            },
        })
    }

    /// Returns the path of the file, either from a Git blob or working tree.
    ///
    /// # Returns
    ///
    /// A `String` representing the path to the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use mini_git::core::objects::FileSource;
    ///
    /// let file_source = FileSource::Worktree { path: "file.txt".to_string() };
    /// assert_eq!(file_source.path(), "file.txt");
    /// ```
    #[must_use]
    pub fn path(&self) -> String {
        match self {
            FileSource::Blob { path, .. } | FileSource::Worktree { path } => {
                path.clone()
            }
        }
    }
}

impl AsRef<Path> for FileSource {
    fn as_ref(&self) -> &Path {
        use FileSource::{Blob, Worktree};
        let (Worktree { ref path } | Blob { ref path, .. }) = self;
        Path::new(path.as_str())
    }
}

/// Retrieves files from a specified tree or the working directory if no tree is specified.
///
/// # Parameters
/// - `repo`: A reference to the `GitRepository`.
/// - `tree`: An optional reference to a tree identifier.
///
/// # Returns
/// - `Ok(Vec<FileSource>)` containing files from the specified tree or working directory.
/// - `Err(String)` if an error occurs while retrieving files.
///
/// # Errors
/// - Returns an error if:
///   - Files cannot be read from the specified tree.
///   - The working directory cannot be accessed.
pub(super) fn get_files(
    repo: &GitRepository,
    tree: Option<&str>,
) -> Result<Vec<FileSource>, String> {
    Ok(match tree {
        // Get contents from the specified tree
        Some(treeish) => {
            tree::get_tree_files(repo, treeish)?.into_iter().collect()
        }

        // Get contents from the working directory
        None => worktree::get_worktree_files(repo, None)?
            .into_iter()
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::repo_dir;
    use crate::utils::test::TempDir;
    use GitObject::{Blob, Commit, Tag, Tree};

    #[test]
    fn test_read_object_bad_path() {
        let tmp_dir = TempDir::<()>::create("test_read_object_bad_path");
        let sha = "abcdef09123456789abc";

        let repo = GitRepository::create(tmp_dir.tmp_dir())
            .expect("Should create repo");

        assert!(read_object(&repo, sha).is_err_and(|msg| msg.contains(sha)));
    }

    #[test]
    fn test_read_object_good_path() {
        let tmp_dir = TempDir::<()>::create("test_read_object_good_path");
        let sha = "deadbeefdecadedefacecafec0ffeedadfacade8";

        let repo = GitRepository::create(tmp_dir.tmp_dir())
            .expect("Should create repo");

        let path = repo_dir(repo.gitdir(), &[OBJECTS_DIR, &sha[..2]], true)
            .expect("Should create dir!")
            .expect("Should contain path!");

        let contents = b"blob 0\0";
        let compressed = zlib::compress(contents, &zlib::Strategy::Fixed);

        fs::write(path.join(&sha[2..]), compressed)
            .expect("Should write contents");

        assert!(
            read_object(&repo, sha).is_ok_and(|obj| matches!(obj, Blob(..)))
        );
    }

    #[test]
    #[ignore = "WIP"]
    fn test_hash_object() {
        let objects = [
            Blob(blob::Blob::default()),
            Commit(commit::Commit::default()),
            Tag(tag::Tag::default()),
            Tree(tree::Tree::default()),
        ];

        for obj in objects {
            let mut expected_hash = sha1::SHA1::new();
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
        let tmp_dir = TempDir::<()>::create("test_write_object_blob");

        let repo = GitRepository::create(tmp_dir.tmp_dir())
            .expect("Should create repo");

        let blob_data = [0; 100];
        let blob = Blob(blob::Blob {
            data: blob_data.to_vec(),
        });

        let digest = write_object(&blob, &repo).expect("Should write object");

        let file = path::repo_file(
            repo.gitdir(),
            &[OBJECTS_DIR, &digest[..2], &digest[2..]],
            false,
        )
        .expect("Should have been created")
        .expect("Should be a file");
        let raw = fs::read(file).expect("Should read file");
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
}
