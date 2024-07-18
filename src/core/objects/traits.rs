use crate::utils::collections::kvlm::KVLM;

trait Format {
    pub const fn format() -> &'static [u8];
}

trait Serialize {
    pub fn serialize(&self) -> Vec<u8>;
}

trait Deserialize {
    pub fn deserialize() -> Result<Self, String>;
}

trait KVLM: Serialize + Deserialize {
    pub fn with_kvlm(kvlm: KVLM) -> Self;

    pub fn kvlm(&self) -> &KVLM;

    pub fn serialize(&self) -> Vec<u8> {
        self.kvlm().serialize()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        Ok(Self::with_kvlm(KVLM::parse(data)?))
    }
}
