#[derive(Debug)]
struct SHA1 {
    digest: [u32; 5],
    buffer: String,
    transforms: usize,
}
