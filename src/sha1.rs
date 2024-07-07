pub const DIGEST_SIZE_BYTES: usize = 20;
const SIZE_OF_U32: usize = 32;
const DIGEST_SIZE: usize = 5;

const SHA1_INIT_DIGEST: [u32; DIGEST_SIZE] =
    [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

#[derive(Debug)]
pub struct SHA1 {
    digest: [u32; DIGEST_SIZE],
    buffer: String,
    transforms: usize,
}

fn rotate_left(uint: u32, shift: usize) -> u32 {
    uint << shift | uint >> (SIZE_OF_U32 - shift)
}

trait Hash<T> {
    fn hash(&self, message: T);
}

impl SHA1 {
    fn new() -> Self {
        Self {
            digest: SHA1_INIT_DIGEST,
            buffer: String::new(),
            transforms: 0usize,
        }
    }

    fn hex_digest(&self) -> String {
        unimplemented!();
    }
}

impl Hash<&str> for SHA1 {
    fn hash(&self, message: &str) {
        self.hash(message.as_bytes())
    }
}

impl Hash<&[u8]> for SHA1 {
    fn hash(&self, message: &[u8]) {}
}

impl From<&str> for SHA1 {
    fn from(text: &str) -> Self {
        let sha = Self::new();
        sha.hash(text);
        sha
    }
}
