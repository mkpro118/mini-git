pub const DIGEST_SIZE_BYTES: usize = 20;
const SIZE_OF_BYTE: usize = 8;
const SIZE_OF_U32: usize = 32;
const SIZE_OF_U64: usize = 64;
const DIGEST_SIZE: usize = 5;
const CHUNK_SIZE: usize = 512;

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
        let sha = Self::default();
        sha.hash(text);
        sha
    }
}

trait Hash<T> {
    fn hash(&self, message: T);
}

impl Hash<&str> for SHA1 {
    fn hash(&self, message: &str) {
        self.hash(message.as_bytes())
    }
}

impl Hash<&[u8]> for SHA1 {
    fn hash(&self, message: &[u8]) {
        unimplemented!()
    }
}

// This function follows the algorithm specified by Wikipedia
// for the SHA-1 algorithm
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

fn rotate_left(uint: u32, shift: usize) -> u32 {
    uint << shift | uint >> (SIZE_OF_U32 - shift)
}
