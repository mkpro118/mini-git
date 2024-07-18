#[derive(Debug)]
pub struct Tag;

impl Tag {
    pub fn new() -> Self {
        todo!();
    }

    pub const fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tag";
        FORMAT
    }

    pub fn serialize(&self) -> Vec<u8> {
        todo!()
    }

    pub fn deserialize(_data: &[u8]) -> Result<Self, String> {
        todo!()
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new()
    }
}
