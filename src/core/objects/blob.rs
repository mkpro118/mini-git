#[derive(Debug)]
pub struct Blob {
    pub(crate) data: Vec<u8>,
}

impl<'a> Blob {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub const fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"blob";
        FORMAT
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        Ok(Blob {
            data: Vec::from(data),
        })
    }
}

impl Default for Blob {
    fn default() -> Self {
        Self::new()
    }
}
