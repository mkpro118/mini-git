#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::objects::traits::{Deserialize, KVLM};
use crate::core::objects::{blob, commit, tag, tree, GitObject};
use crate::core::GitRepository;
use crate::utils::path;
use crate::utils::zlib;

const HASH_SIZE: usize = 20;
type Hash = [u8; HASH_SIZE];

#[derive(Debug)]
pub struct PackFile {
    index: HashMap<Hash, u64>,
    pack_file: fs::File,
    object_cache: HashMap<u64, Vec<u8>>,
}

impl PackFile {
    #[allow(clippy::similar_names, clippy::cast_possible_wrap)]
    pub fn from_files(
        idx_path: &Path,
        pack_path: &Path,
    ) -> Result<Self, String> {
        // Parse the index file
        let idx_file = fs::File::open(idx_path).map_err(|e| e.to_string())?;
        let mut idx_reader = std::io::BufReader::new(&idx_file);

        // Read the header
        let mut header = [0u8; 8];
        idx_reader
            .read_exact(&mut header)
            .map_err(|e| e.to_string())?;

        if &header[0..4] == b"\xfftOc" {
            // Version 2
            let version = u32::from_be_bytes([
                header[4], header[5], header[6], header[7],
            ]);
            if version != 2 {
                return Err(format!(
                    "Unsupported pack index version: {version}"
                ));
            }

            // Read fan-out table
            let mut fanout_table = [0u32; 256];
            for item in &mut fanout_table {
                let mut buf = [0u8; 4];
                idx_reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
                *item = u32::from_be_bytes(buf);
            }

            let num_objects = fanout_table[255] as usize;

            // Read object hashes
            let mut hashes = Vec::with_capacity(num_objects);
            for _ in 0..num_objects {
                let mut hash = [0u8; 20];
                idx_reader
                    .read_exact(&mut hash)
                    .map_err(|e| e.to_string())?;
                hashes.push(hash);
            }

            // Skip CRC32 checksums
            idx_reader
                .seek(SeekFrom::Current((num_objects * 4) as i64))
                .map_err(|e| e.to_string())?;

            // Read 4-byte offsets
            let mut offsets = Vec::with_capacity(num_objects);
            let mut large_offsets_indices = Vec::new();
            for i in 0..num_objects {
                let mut buf = [0u8; 4];
                idx_reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
                let offset = u32::from_be_bytes(buf);
                if offset & 0x8000_0000 != 0 {
                    // Large offset
                    large_offsets_indices.push(i);
                    offsets.push(0);
                } else {
                    offsets.push(u64::from(offset));
                }
            }

            // Read large offsets
            let num_large_offsets = large_offsets_indices.len();
            let mut large_offsets = Vec::with_capacity(num_large_offsets);
            for _ in 0..num_large_offsets {
                let mut buf = [0u8; 8];
                idx_reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
                let offset = u64::from_be_bytes(buf);
                large_offsets.push(offset);
            }

            // Map large offsets
            for (i, &index) in large_offsets_indices.iter().enumerate() {
                offsets[index] = large_offsets[i];
            }

            // Build the index
            let mut index = HashMap::new();
            for i in 0..num_objects {
                index.insert(hashes[i], offsets[i]);
            }

            // Open the pack file
            let pack_file =
                fs::File::open(pack_path).map_err(|e| e.to_string())?;

            // Read packfile header to get version and object count
            let mut pack_reader = std::io::BufReader::new(&pack_file);
            let mut pack_header = [0u8; 12];
            pack_reader
                .read_exact(&mut pack_header)
                .map_err(|e| e.to_string())?;

            if &pack_header[0..4] != b"PACK" {
                return Err("Invalid packfile signature".to_string());
            }
            let pack_version = u32::from_be_bytes([
                pack_header[4],
                pack_header[5],
                pack_header[6],
                pack_header[7],
            ]);
            if pack_version != 2 {
                return Err(format!(
                    "Packfile version not supported: {pack_version}."
                ));
            }

            Ok(PackFile {
                index,
                pack_file,
                object_cache: HashMap::new(),
            })
        } else {
            // Version 1 (legacy) format is not supported
            Err("Unsupported pack index version".to_string())
        }
    }

    pub fn read_object(&mut self, hash: &Hash) -> Result<GitObject, String> {
        let &offset = self
            .index
            .get(hash)
            .ok_or_else(|| "Object not found in packfile".to_string())?;

        let data = self.read_object_at_offset(offset)?;

        // Read object type
        self.pack_file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut reader = std::io::BufReader::new(&self.pack_file);

        // Read object header
        let mut first_byte = [0u8; 1];
        reader
            .read_exact(&mut first_byte)
            .map_err(|e| e.to_string())?;
        let c = first_byte[0];
        let object_type = (c >> 4) & 0x07;

        // Create GitObject from data
        let git_object = match object_type {
            1 => {
                // Commit
                let commit = commit::Commit::deserialize(&data)?;
                GitObject::Commit(commit)
            }
            2 => {
                // Tree
                let tree = tree::Tree::deserialize(&data)?;
                GitObject::Tree(tree)
            }
            3 => {
                // Blob
                let blob = blob::Blob::deserialize(&data)?;
                GitObject::Blob(blob)
            }
            4 => {
                // Tag
                let tag = tag::Tag::deserialize(&data)?;
                GitObject::Tag(tag)
            }
            _ => {
                return Err(format!("Unknown object type: {object_type}"));
            }
        };

        Ok(git_object)
    }

    fn read_object_at_offset(
        &mut self,
        offset: u64,
    ) -> Result<Vec<u8>, String> {
        if let Some(data) = self.object_cache.get(&offset) {
            return Ok(data.clone());
        }

        self.pack_file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut reader = std::io::BufReader::new(&self.pack_file);

        // Read object header
        let mut first_byte = [0u8; 1];
        reader
            .read_exact(&mut first_byte)
            .map_err(|e| e.to_string())?;
        let mut c = first_byte[0];
        let object_type = (c >> 4) & 0x07;
        while c & 0x80 != 0 {
            reader
                .read_exact(&mut first_byte)
                .map_err(|e| e.to_string())?;
            c = first_byte[0];
        }

        let mut base_offset = 0u64;
        let mut base_hash = [0u8; 20];
        match object_type {
            1..=4 => {
                // Base object types
            }
            6 => {
                // OFS_DELTA
                let mut c = [0u8; 1];
                reader.read_exact(&mut c).map_err(|e| e.to_string())?;
                base_offset = u64::from(c[0] & 0x7F);
                while c[0] & 0x80 != 0 {
                    base_offset += 1;
                    base_offset <<= 7;
                    reader.read_exact(&mut c).map_err(|e| e.to_string())?;
                    base_offset |= u64::from(c[0] & 0x7F);
                }
                base_offset = offset - base_offset;
            }
            7 => {
                // REF_DELTA
                reader
                    .read_exact(&mut base_hash)
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                return Err(format!("Unknown object type: {object_type}"));
            }
        }

        // Read compressed data
        let compressed_data = {
            let mut buf = vec![];
            reader.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            zlib::decompress(&buf)?
        };

        let data = if object_type == 6 || object_type == 7 {
            let base_data = if object_type == 6 {
                self.read_object_at_offset(base_offset)?
            } else {
                let &base_offset =
                    self.index.get(&base_hash).ok_or_else(|| {
                        "Base object not found in packfile".to_string()
                    })?;
                self.read_object_at_offset(base_offset)?
            };
            delta::apply_delta(&base_data, &compressed_data)?
        } else {
            compressed_data
        };

        self.object_cache.insert(offset, data.clone());

        Ok(data)
    }
}

pub fn find_packfiles(repo: &GitRepository) -> Result<Vec<PackFile>, String> {
    let pack_dir = path::repo_dir(repo.gitdir(), &["objects", "pack"], false)?
        .ok_or_else(|| "Pack directory not found".to_string())?;

    let mut packfiles = Vec::new();

    let entries = fs::read_dir(pack_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if let Some(extension) = path.extension() {
            if extension == "idx" {
                let pack_path = path.with_extension("pack");
                if pack_path.exists() {
                    let packfile = PackFile::from_files(&path, &pack_path)?;
                    packfiles.push(packfile);
                }
            }
        }
    }

    Ok(packfiles)
}

mod delta {
    pub fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
        let mut delta = delta;

        // Read base object size
        let (base_size, offset) = read_varint(delta)?;
        delta = &delta[offset..];

        if base_size != base.len() {
            return Err("Delta base size mismatch".to_string());
        }

        // Read result size
        let (result_size, offset) = read_varint(delta)?;
        delta = &delta[offset..];

        let mut result = Vec::with_capacity(result_size);

        while !delta.is_empty() {
            let opcode = delta[0];
            delta = &delta[1..];
            if opcode & 0x80 != 0 {
                let mut copy_offset = 0usize;
                let mut copy_size = 0usize;
                if opcode & 0x01 != 0 {
                    copy_offset |= delta[0] as usize;
                    delta = &delta[1..];
                }
                if opcode & 0x02 != 0 {
                    copy_offset |= (delta[0] as usize) << 8;
                    delta = &delta[1..];
                }
                if opcode & 0x04 != 0 {
                    copy_offset |= (delta[0] as usize) << 16;
                    delta = &delta[1..];
                }
                if opcode & 0x08 != 0 {
                    copy_offset |= (delta[0] as usize) << 24;
                    delta = &delta[1..];
                }
                if opcode & 0x10 != 0 {
                    copy_size |= delta[0] as usize;
                    delta = &delta[1..];
                }
                if opcode & 0x20 != 0 {
                    copy_size |= (delta[0] as usize) << 8;
                    delta = &delta[1..];
                }
                if opcode & 0x40 != 0 {
                    copy_size |= (delta[0] as usize) << 16;
                    delta = &delta[1..];
                }
                if copy_size == 0 {
                    copy_size = 0x10000;
                }
                result.extend_from_slice(
                    &base[copy_offset..copy_offset + copy_size],
                );
            } else if opcode != 0 {
                let insert_size = opcode as usize;
                result.extend_from_slice(&delta[..insert_size]);
                delta = &delta[insert_size..];
            } else {
                return Err("Invalid delta opcode 0".to_string());
            }
        }

        if result.len() != result_size {
            return Err("Delta result size mismatch".to_string());
        }

        Ok(result)
    }

    pub(super) fn read_varint(data: &[u8]) -> Result<(usize, usize), String> {
        let mut result = 0usize;
        let mut shift = 0;
        let mut offset = 0;
        loop {
            if offset >= data.len() {
                return Err("Unexpected end of delta data".to_string());
            }
            let byte = data[offset];
            offset += 1;
            result |= ((byte & 0x7F) as usize) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Ok((result, offset))
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::test::TempDir;

    use super::delta::{apply_delta, read_varint};
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_read_varint() {
        // Test reading single-byte varint
        let data = [0x7F]; // 127
        let (value, offset) = read_varint(&data).unwrap();
        assert_eq!(value, 127);
        assert_eq!(offset, 1);

        // Test reading multi-byte varint
        let data = [0x81, 0x01]; // 129
        let (value, offset) = read_varint(&data).unwrap();
        assert_eq!(value, 129);
        assert_eq!(offset, 2);

        // Test reading larger varint
        let data = [0x85, 0x80, 0x01]; // 16389
        let (value, offset) = read_varint(&data).unwrap();
        assert_eq!(value, 16389);
        assert_eq!(offset, 3);

        // Test error on empty data
        let data: [u8; 0] = [];
        let result = read_varint(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_delta_copy() {
        // Base data
        let base = b"Hello, world!";
        // Delta instructions to copy entire base
        let delta = vec![
            0x0D, // Base size (13)
            0x0D, // Result size (13)
            0x91, // Opcode: copy, offset[0] and size[0] present
            0x00, // offset[0] = 0x00
            0x0D, // size[0] = 13
        ];

        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, base);
    }

    #[test]
    fn test_apply_delta_insert() {
        // Base data
        let base = b"";
        // Delta instructions to insert "Hello, world!"
        let delta = {
            let mut v = Vec::new();
            v.push(0x00); // Base size
            v.push(0x0D); // Result size
            v.push(0x0D); // Insert command for 13 bytes
            v.extend_from_slice(b"Hello, world!");
            v
        };
        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_apply_delta_mixed() {
        // Base data
        let base = b"Hello, world!";
        // Expected result: "Hello, Rust!"
        let delta = {
            let mut v = vec![
                0x0D, // Base size (13)
                0x0C, // Result size (12)
                // Copy "Hello, "
                0x91, // Opcode: copy, offset[0] and size[0] present
                0x00, // offset[0] = 0
                0x07, // size[0] = 7
                // Insert "Rust"
                0x04, // Insert command for 4 bytes
            ];
            v.extend_from_slice(b"Rust");
            // Copy "!" from base[12]
            v.push(0x91); // Opcode: copy, offset[0] and size[0] present
            v.push(0x0C); // offset[0] = 12
            v.push(0x01); // size[0] = 1
            v
        };
        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, b"Hello, Rust!");
    }

    #[test]
    fn test_apply_delta_invalid_opcode() {
        // Base data
        let base = b"Hello";
        // Delta with invalid opcode (0x00)
        let delta = [0x05, 0x05, 0x00];
        let result = apply_delta(base, &delta);
        assert!(result.is_err());
    }

    #[test]
    fn test_packfile_from_files_invalid_paths() {
        let idx_path = Path::new("nonexistent.idx");
        let pack_path = Path::new("nonexistent.pack");

        let result = PackFile::from_files(idx_path, pack_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_packfiles_empty_repo() {
        let tmp_dir = TempDir::<()>::create("test_find_packfiles_empty_repo");
        let gitdir = tmp_dir.tmp_dir().join(".git");
        fs::create_dir_all(&gitdir).unwrap();
        let repo = GitRepository::create(&gitdir).unwrap();

        let result = find_packfiles(&repo).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_object_at_offset_cache() {
        // Create a dummy PackFile with empty index and a fake pack_file
        let tmp_dir = TempDir::<()>::create("test_read_object_at_offset_cache");
        let pack_path = tmp_dir.tmp_dir().join("packfile.pack");
        let mut pack_file = File::create(&pack_path).unwrap();
        // Write dummy data to pack_file
        pack_file.write_all(b"PACK").unwrap(); // Magic number
        pack_file
            .write_all(&[0x00, 0x00, 0x00, 0x02]) // Version 2
            .unwrap();
        pack_file
            .write_all(&[0x00, 0x00, 0x00, 0x01]) // 1 object
            .unwrap();
        // Write a dummy object at offset 12
        pack_file.write_all(&[0x00]).unwrap(); // Object data
        pack_file.flush().unwrap();

        let packfile = PackFile {
            index: HashMap::new(),
            pack_file: File::open(&pack_path).unwrap(),
            object_cache: HashMap::new(),
        };

        // Since there's no real object, we can't read it, but we can test that
        // the cache is empty initially
        assert!(packfile.object_cache.is_empty());
    }
}
