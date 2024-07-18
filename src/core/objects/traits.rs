use crate::utils::collections::kvlm;

pub trait Format {
    fn format() -> &'static [u8];
}

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

pub trait Deserialize {
    fn deserialize(data: &[u8]) -> Result<Self, String>
    where
        Self: Sized;
}

pub trait KVLM {
    fn with_kvlm(kvlm: kvlm::KVLM) -> Self;

    fn kvlm(&self) -> &kvlm::KVLM;

    fn serialize(&self) -> Vec<u8> {
        self.kvlm().serialize()
    }

    fn deserialize(data: &[u8]) -> Result<Self, String>
    where
        Self: Sized,
    {
        Ok(Self::with_kvlm(kvlm::KVLM::parse(data)?))
    }
}
