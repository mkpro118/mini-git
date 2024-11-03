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

    fn read_varint(data: &[u8]) -> Result<(usize, usize), String> {
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
