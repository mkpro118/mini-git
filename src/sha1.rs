const SHA1_INIT_DIGEST: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

const SIZE_OF_U32: usize = 32;

#[derive(Debug)]
pub struct SHA1 {
    digest: [u32; 5],
    buffer: String,
    transforms: usize,
}

fn rotate_left(uint: u32, shift: usize) -> u32 {
    uint << shift | uint >> (SIZE_OF_U32 - shift)
}

impl SHA1 {
    fn new() -> Self {
        Self {
            digest: SHA1_INIT_DIGEST,
            buffer: String::new(),
            transforms: 0usize,
        }
    }
}
