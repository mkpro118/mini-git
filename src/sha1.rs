//! This module computes the SHA-1 hash following the algorithm specified
//! by Wikipedia
//!
//! The algorithm can be found here:
//! https://en.wikipedia.org/wiki/SHA-1#SHA-1_pseudocode

use std::ops::BitXor;

pub const DIGEST_SIZE_BYTES: usize = 20;
const SIZE_OF_BYTE: usize = 8;
const SIZE_OF_U32: usize = 32;
const SIZE_OF_U64: usize = 64;
const DIGEST_SIZE: usize = 5;
const CHUNK_SIZE: usize = 512;
const ROUND1_K: u32 = 0x5A82_7999;
const ROUND2_K: u32 = 0x6ED9_EBA1;
const ROUND3_K: u32 = 0x8F1B_BCDC;
const ROUND4_K: u32 = 0xCA62_C1D6;

const SHA1_INIT_DIGEST: [u32; DIGEST_SIZE] =
    [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

#[derive(Debug)]
pub struct SHA1 {
    digest: [u32; DIGEST_SIZE],
}

impl SHA1 {
    fn reset(&mut self) {
        self.digest = SHA1_INIT_DIGEST;
    }

    fn hex_digest(&self) -> String {
        unimplemented!();
    }
}

impl Default for SHA1 {
    fn default() -> Self {
        let mut sha = Self {
            digest: [0u32; DIGEST_SIZE],
        };
        sha.reset();
        sha
    }
}

impl From<&str> for SHA1 {
    fn from(text: &str) -> Self {
        let mut sha = Self::default();
        sha.hash(text);
        sha
    }
}

/// Allow generic hash function based on param type
trait Hash<T> {
    fn hash(&mut self, message: T);
}

impl Hash<&str> for SHA1 {
    fn hash(&mut self, message: &str) {
        self.hash(message.as_bytes())
    }
}

impl Hash<&[u8]> for SHA1 {
    fn hash(&mut self, message: &[u8]) {
        self.digest = hash(self.digest, message);
    }
}

/// Shortcut function to hash a message
fn hash(initial_digest: [u32; 5], message: &[u8]) -> [u32; 5] {
    let preprocessed = preprocess_message(message);
    hash_pre_processed(&initial_digest, &preprocessed)
}

/// Preprocesses a message to make it SHA-1 compatible
fn preprocess_message(message: &[u8]) -> Vec<u8> {
    // Ensure all values are big-endian
    let mut seq: Vec<u8> = message.iter().map(|x| x.to_be()).collect();

    let message_length: u64 = message.len() as u64;
    // Convert message length to big-endian bytes
    let message_length_as_bytes = message_length.to_be_bytes();

    // Append a '1' to the message, however, since we have to add zeros anyway
    // Add a (0b1000_0000) in one operation
    seq.push(0x80u8);

    // Number of bits after appending a '1' to the message
    let n_bits = seq.len() * SIZE_OF_BYTE;

    // 64 bits reserved for the message_length
    let n_bits_reserved = n_bits + SIZE_OF_U64;

    // Fill the rest of the bits with zeros until length is a multiple of CHUNK_SIZE (512)
    let n_zeros = CHUNK_SIZE * (1 + (n_bits_reserved / CHUNK_SIZE)) - n_bits_reserved;

    // Divide by size of a byte (8) as we add that many u8 values
    let n_zeros = n_zeros / SIZE_OF_BYTE;

    // Append zeros
    seq.extend_from_slice(&[0u8].repeat(n_zeros));

    // Append message length as 64 bit unsigned int
    seq.extend_from_slice(&message_length_as_bytes);

    seq
}

/// Computes the SHA-1 hash over a preprocessed message
fn hash_pre_processed(digest: &[u32; 5], message: &[u8]) -> [u32; 5] {
    assert!(message.len() >= 512, "Message is too short");
    assert_eq!(message.len() % 512, 0, "Message is not a multiple of 512");

    // Break message into 512 bit chunks
    message
        .chunks(CHUNK_SIZE / SIZE_OF_BYTE)
        .fold(*digest, |digest: [u32; 5], chunk: &[u8]| {
            // Convert a chunk of 4 bytes (u8) to a word (u32)
            let chunk: [u32; 16] = chunk
                .chunks(SIZE_OF_U32 / SIZE_OF_BYTE)
                .map(|sub_chunk: &[u8]| {
                    let bytes: [u8; 4] = sub_chunk.try_into().unwrap();
                    u32::from_be_bytes(bytes)
                })
                .collect::<Vec<u32>>()
                .try_into()
                .unwrap();

            // Hash the chunk
            hash_chunk(&digest, &chunk)
        })
}

/// Hashes a 512-bit chunk using the given initial hash variables
/// Returns the final hash variable after performing the SHA-1 hash
/// on the chunk
fn hash_chunk(hash_vars: &[u32; 5], chunk: &[u32; 16]) -> [u32; 5] {
    let schedule = message_schedule(chunk);
    sha1_main(hash_vars, &schedule)
}

/// Creates the message schedule for a given chunk
/// It extends the 16-word chunk into a 80-word chunk
fn message_schedule(chunk: &[u32; 16]) -> [u32; 80] {
    let mut words = [0u32; 80];

    // The first 16 words are from the chunk
    words[..chunk.len()].clone_from_slice(chunk);

    // Populate the rest of the words
    for i in chunk.len()..words.len() {
        words[i] = words[i - 3]
            .bitxor(words[i - 8])
            .bitxor(words[i - 14])
            .bitxor(words[i - 16]);
        words[i] = rotate_left(words[i], 1);
    }

    words
}

/// The main loop of the SHA-1 Hash
fn sha1_main(hash_vars: &[u32; 5], schedule: &[u32; 80]) -> [u32; 5] {
    let r1 = round1(hash_vars, schedule[..20].try_into().unwrap());
    let r2 = round2_or_4(&r1, schedule[20..40].try_into().unwrap(), true);
    let r3 = round3(&r2, schedule[40..60].try_into().unwrap());
    let r4 = round2_or_4(&r3, schedule[60..80].try_into().unwrap(), false);

    // Add this schedule's result to the original hash variables
    hash_vars
        .iter()
        .zip(r4.iter())
        .map(|(x, y)| x.wrapping_add(*y))
        .collect::<Vec<u32>>()
        .try_into()
        .unwrap()
}

/// SHA-1 Round 1, computes hash over the first 20 words of the schedule
#[inline]
fn round1(hash_vars: &[u32; 5], words: &[u32; 20]) -> [u32; 5] {
    let [mut a, mut b, mut c, mut d, mut e] = hash_vars;

    for &word in words {
        let f = (b & c) | ((!b) & d);
        [a, b, c, d, e] = swap_vars(word, [a, b, c, d, e], f, ROUND1_K);
    }

    [a, b, c, d, e]
}

/// This function computes the hash over both round 2 and round 4
/// The only difference between those rounds is the round constant used.
/// `r2 = true` means use the constant for round 2, and `false` means use the
/// constant for round 4.
#[inline]
fn round2_or_4(hash_vars: &[u32; 5], words: &[u32; 20], r2: bool) -> [u32; 5] {
    let [mut a, mut b, mut c, mut d, mut e] = hash_vars;
    let k: u32 = if r2 { ROUND2_K } else { ROUND4_K };

    for &word in words {
        let f = b ^ c ^ d;
        [a, b, c, d, e] = swap_vars(word, [a, b, c, d, e], f, k);
    }

    [a, b, c, d, e]
}

/// SHA-1 Round 3, computes hash over the words 40-60 of the schedule
#[inline]
fn round3(hash_vars: &[u32; 5], words: &[u32; 20]) -> [u32; 5] {
    let [mut a, mut b, mut c, mut d, mut e] = hash_vars;

    for &word in words {
        let f = (b & c) | (b & d) | (c & d);
        [a, b, c, d, e] = swap_vars(word, [a, b, c, d, e], f, ROUND3_K);
    }

    [a, b, c, d, e]
}

/// This function is common to all the rounds, where the hash variables
/// are manipulated and swapped
#[inline]
fn swap_vars(word: u32, vars: [u32; 5], f: u32, k: u32) -> [u32; 5] {
    let temp = rotate_left(vars[0], 5)
        .wrapping_add(f)
        .wrapping_add(vars[4])
        .wrapping_add(k)
        .wrapping_add(word);

    [temp, vars[0], rotate_left(vars[1], 30), vars[2], vars[3]]
}

fn rotate_left(uint: u32, shift: usize) -> u32 {
    uint << shift | uint >> (SIZE_OF_U32 - shift)
}
