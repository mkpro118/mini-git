use crate::core::kvlm::KVLM;

#[derive(Debug)]
pub struct Commit {
    pub(crate) kvlm: KVLM,
}

impl Commit {
    pub fn new() -> Self {
        Self {kvlm: KVLM::new()}
    }

    pub const fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"commit";
        FORMAT
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.kvlm.serialize()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        Ok(Commit {kvlm: KVLM::parse(data)?})
    }
}

impl Default for Commit {
    fn default() -> Self {
        Self::new()
    }
}
