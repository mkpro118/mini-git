#[derive(Debug)]
struct SHA1 {
    digest: [u32; 5],
    buffer: String,
    transforms: usize,
}

impl SHA1 {
    fn new() -> Self {
        Self {
            digest: [0u32; 5],
            buffer: String::new(),
            transforms: 0usize,
        }
    }
}
