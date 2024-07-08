use std::fmt::Write;
use std::iter;

const INITIAL_STATE: [u32; 5] = [
    0x6745_2301,
    0xEFCD_AB89,
    0x98BA_DCFE,
    0x1032_5476,
    0xC3D2_E1F0,
];

pub struct SHA1 {
    state: [u32; 5],
    buffer: Vec<u8>,
    total_len: u64,
}

impl Default for SHA1 {
    fn default() -> Self {
        Self::new()
    }
}

impl SHA1 {
    #[must_use]
    pub fn new() -> Self {
        SHA1 {
            state: INITIAL_STATE,
            buffer: Vec::new(),
            total_len: 0,
        }
    }

    #[must_use]
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.total_len += data.len() as u64;
        self.buffer.extend_from_slice(data);

        let (new_buffer, new_state) =
            self.buffer
                .chunks(64)
                .fold((Vec::new(), self.state), |(mut buffer, state), chunk| {
                    if chunk.len() == 64 {
                        (buffer, process_chunk(chunk, state))
                    } else {
                        buffer.extend_from_slice(chunk);
                        (buffer, state)
                    }
                });

        self.state = new_state;
        self.buffer = new_buffer;

        self
    }

    #[allow(missing_docs)]
    pub fn finalize(&mut self) -> [u8; 20] {
        let mod_len = (self.total_len % 64) as usize;
        let padding = create_padding(mod_len, self.total_len);

        let final_state = self.update(&padding).state;

        final_state
            .iter()
            .flat_map(|&word| word.to_be_bytes())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    pub fn hex_digest(&mut self) -> String {
        self.finalize().iter().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{b:02x}");
            output
        })
    }
}

fn create_padding(mod_len: usize, total_len: u64) -> Vec<u8> {
    let padding_len = if mod_len < 56 {
        56 - mod_len
    } else {
        120 - mod_len
    };
    let mut padding = vec![0u8; padding_len + 8];
    padding[0] = 0x80;
    padding[padding_len..].copy_from_slice(&(total_len * 8).to_be_bytes());
    padding
}

#[allow(clippy::many_single_char_names)]
fn process_chunk(chunk: &[u8], initial_state: [u32; 5]) -> [u32; 5] {
    let words = expand_chunk(chunk);
    let [a, b, c, d, e] = initial_state;

    let final_state = (0..80).fold((a, b, c, d, e), |(a, b, c, d, e), i| {
        let (f, k) = match i {
            0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
            20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            60..=79 => (b ^ c ^ d, 0xCA62_C1D6),
            _ => unreachable!(),
        };

        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(words[i]);

        (temp, a, b.rotate_left(30), c, d)
    });

    [
        initial_state[0].wrapping_add(final_state.0),
        initial_state[1].wrapping_add(final_state.1),
        initial_state[2].wrapping_add(final_state.2),
        initial_state[3].wrapping_add(final_state.3),
        initial_state[4].wrapping_add(final_state.4),
    ]
}

fn expand_chunk(chunk: &[u8]) -> [u32; 80] {
    let mut words = [0u32; 80];
    words[..16].iter_mut().enumerate().for_each(|(i, word)| {
        *word = u32::from_be_bytes([
            chunk[i * 4],
            chunk[i * 4 + 1],
            chunk[i * 4 + 2],
            chunk[i * 4 + 3],
        ]);
    });

    (16..80).for_each(|i| {
        words[i] = (words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16]).rotate_left(1);
    });

    words
}

#[must_use]
pub fn hash(message: &[u8]) -> [u8; 20] {
    SHA1::new().update(message).finalize()
}
