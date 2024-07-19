use std::num::ParseIntError;

pub enum DecodeHexError {
    OddLength,
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for DecodeHexError {
    fn from(e: ParseIntError) -> Self {
        DecodeHexError::ParseInt(e)
    }
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, DecodeHexError> {
    if s.len() & 1 != 0 {
        return Err(DecodeHexError::OddLength);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
        .collect()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, &byte| {
            s.push_str(format!("{byte:02x}").as_str());
            s
        })
}
