//! Encoding packets.
//!
//! Separated from handlers.rs under #693.

pub(super) fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

pub(super) fn hex_encode_upper(data: &[u8]) -> String {
    hex_encode(data)
}

pub(super) fn decode_base64_standard_like_cpp(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0usize;
    let mut finished_padding = false;

    for byte in input.bytes() {
        let sextet = if byte == b'=' {
            finished_padding = true;
            64
        } else {
            if finished_padding {
                return None;
            }
            value(byte)?
        };

        chunk[chunk_len] = sextet;
        chunk_len += 1;

        if chunk_len == 4 {
            if chunk[0] == 64 || chunk[1] == 64 || (chunk[2] == 64 && chunk[3] != 64) {
                return None;
            }

            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                output.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                output.push((chunk[2] << 6) | chunk[3]);
            }

            chunk_len = 0;
        }
    }

    match chunk_len {
        0 => Some(output),
        2 if !finished_padding => {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            Some(output)
        }
        3 if !finished_padding => {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            output.push((chunk[1] << 4) | (chunk[2] >> 2));
            Some(output)
        }
        _ => None,
    }
}

pub(super) fn hex_decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| {
            hex.get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok())
        })
        .collect()
}
