// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Packet compression using zlib (deflate) with custom Adler32 checksums.
//!
//! Packets larger than [`COMPRESSION_THRESHOLD`] bytes are compressed before
//! encryption. The compressed format prepends uncompressed size and Adler32
//! checksums for integrity verification.
//!
//! C# RustyCore uses a persistent deflate stream with Z_SYNC_FLUSH per packet.
//! We match this by using flate2's low-level `Compress` API with `FlushCompress::Sync`.

use flate2::{Compress, Compression, FlushCompress};

use crate::world_packet::PacketError;

/// Packets larger than this are compressed.
///
/// Matches C# RustyCore threshold: 0x400 = 1024 bytes.
/// C# checks `packetSize > 0x400` where packetSize = payload length (without opcode).
/// We check `data.len() > COMPRESSION_THRESHOLD` where data = opcode + payload.
/// The 2-byte difference is negligible.
pub const COMPRESSION_THRESHOLD: usize = 0x400;

/// Custom Adler32 initial value used by the WoW protocol.
const ADLER32_INIT: u32 = 0x9827_D8F1;

/// Adler32 modulus.
const ADLER32_MOD: u32 = 65521;

/// Compute a custom Adler32 checksum with the WoW-specific initial value.
pub fn adler32(data: &[u8]) -> u32 {
    adler32_with_init(data, ADLER32_INIT)
}

/// Compute Adler32 starting from a given initial value.
pub fn adler32_with_init(data: &[u8], init: u32) -> u32 {
    let mut a = init & 0xFFFF;
    let mut b = (init >> 16) & 0xFFFF;

    for &byte in data {
        a = (a + u32::from(byte)) % ADLER32_MOD;
        b = (b + a) % ADLER32_MOD;
    }

    (b << 16) | a
}

/// Persistent compressor that maintains deflate state across packets.
///
/// C# RustyCore uses a single `z_stream` per socket, initialized once
/// with `deflateInit2(stream, 1, 8, -15, 8, 0)` and reused for all
/// packets. The WoW client has a matching persistent inflate stream.
/// We must match this: each `compress_packet_with` call feeds into
/// the same deflate state, so the client's inflate can decode correctly.
pub struct PacketCompressor {
    inner: Compress,
}

impl PacketCompressor {
    /// Create a new persistent compressor (one per socket).
    pub fn new() -> Self {
        // Raw deflate, level 1 (fast), matching C#:
        // deflateInit2(stream, 1, 8, -15, 8, 0)
        Self {
            inner: Compress::new(Compression::fast(), false),
        }
    }

    /// Compress a packet using the persistent deflate stream.
    ///
    /// Format: `[UncompressedSize: i32][UncompressedAdler32: u32][CompressedAdler32: u32][CompressedData]`
    pub fn compress_packet(&mut self, opcode_bytes: &[u8; 2], payload: &[u8]) -> Vec<u8> {
        compress_packet_impl(&mut self.inner, opcode_bytes, payload)
    }
}

/// Compressed packet format:
///
/// ```text
/// [UncompressedSize: i32 LE]      — original opcode(2) + payload size
/// [UncompressedAdler32: u32 LE]   — adler32 of original opcode + payload
/// [CompressedAdler32: u32 LE]     — adler32 of compressed data
/// [CompressedData: ...]           — deflated bytes (raw deflate, Z_SYNC_FLUSH)
/// ```
///
/// Returns the compressed data (to be sent with `ServerOpcodes::CompressedPacket`).
///
/// **Note:** This creates a fresh compressor per call. For production use,
/// prefer [`PacketCompressor`] which maintains state across packets
/// (required for the WoW client's persistent inflate stream).
pub fn compress_packet(opcode_bytes: &[u8; 2], payload: &[u8]) -> Vec<u8> {
    let mut compressor = Compress::new(Compression::fast(), false);
    compress_packet_impl(&mut compressor, opcode_bytes, payload)
}

/// Shared implementation: compress using an existing deflate stream.
fn compress_packet_impl(
    compressor: &mut Compress,
    opcode_bytes: &[u8; 2],
    payload: &[u8],
) -> Vec<u8> {
    let uncompressed_size = (2 + payload.len()) as i32;

    // Adler32 of original opcode + payload
    let uncompressed_adler = {
        let a = adler32_with_init(opcode_bytes, ADLER32_INIT);
        adler32_with_init(payload, a)
    };

    // Concatenate input: opcode + payload
    let input_len = 2 + payload.len();
    let mut input = Vec::with_capacity(input_len);
    input.extend_from_slice(opcode_bytes);
    input.extend_from_slice(payload);

    // Output buffer — deflateBound is roughly input_len + input_len/1000 + 20
    // Use generous size to avoid BufError
    let out_bound = input_len + (input_len >> 9) + 256;
    let mut output = vec![0u8; out_bound];

    // Track how much THIS call consumes/produces (compressor.total_in/out
    // are cumulative across all calls on a persistent stream).
    let base_in = compressor.total_in() as usize;
    let base_out = compressor.total_out() as usize;

    // Compress all input with Z_SYNC_FLUSH
    loop {
        let consumed = compressor.total_in() as usize - base_in;
        let produced = compressor.total_out() as usize - base_out;

        let status = compressor
            .compress(
                &input[consumed..],
                &mut output[produced..],
                FlushCompress::Sync,
            )
            .expect("deflate compression failed");

        let new_consumed = compressor.total_in() as usize - base_in;
        let new_produced = compressor.total_out() as usize - base_out;

        match status {
            flate2::Status::Ok => {
                if new_consumed >= input_len {
                    break;
                }
            }
            flate2::Status::BufError => {
                output.resize(output.len() * 2, 0);
            }
            flate2::Status::StreamEnd => {
                break;
            }
        }

        // Safety: if nothing was consumed or produced, avoid infinite loop
        if new_consumed == consumed && new_produced == produced {
            break;
        }
    }

    let total_produced = compressor.total_out() as usize - base_out;
    output.truncate(total_produced);

    // Adler32 of compressed data
    let compressed_adler = adler32(&output);

    // Build the result
    let mut result = Vec::with_capacity(12 + output.len());
    result.extend_from_slice(&uncompressed_size.to_le_bytes());
    result.extend_from_slice(&uncompressed_adler.to_le_bytes());
    result.extend_from_slice(&compressed_adler.to_le_bytes());
    result.extend_from_slice(&output);

    result
}

/// Decompress a compressed packet payload.
///
/// Input is the payload after the `CompressedPacket` opcode:
/// `[uncompressed_size: i32][uncompressed_adler: u32][compressed_adler: u32][data...]`
///
/// Returns the decompressed data (opcode + payload).
pub fn decompress_packet(data: &[u8]) -> Result<Vec<u8>, PacketError> {
    if data.len() < 12 {
        return Err(PacketError::DecompressionError(
            "compressed packet too short".into(),
        ));
    }

    let uncompressed_size =
        i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let expected_uncompressed_adler =
        u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let expected_compressed_adler =
        u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

    let compressed = &data[12..];

    // Verify compressed adler32
    let actual_compressed_adler = adler32(compressed);
    if actual_compressed_adler != expected_compressed_adler {
        return Err(PacketError::DecompressionError(format!(
            "compressed adler32 mismatch: expected 0x{expected_compressed_adler:08X}, got 0x{actual_compressed_adler:08X}"
        )));
    }

    // Decompress
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let mut decoder = DeflateDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(uncompressed_size);
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| PacketError::DecompressionError(e.to_string()))?;

    if decompressed.len() != uncompressed_size {
        return Err(PacketError::DecompressionError(format!(
            "size mismatch: expected {uncompressed_size}, got {}",
            decompressed.len()
        )));
    }

    // Verify uncompressed adler32
    let actual_uncompressed_adler = adler32(&decompressed);
    if actual_uncompressed_adler != expected_uncompressed_adler {
        return Err(PacketError::DecompressionError(format!(
            "uncompressed adler32 mismatch: expected 0x{expected_uncompressed_adler:08X}, got 0x{actual_uncompressed_adler:08X}"
        )));
    }

    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_empty() {
        let result = adler32(&[]);
        assert_eq!(result, ADLER32_INIT);
    }

    #[test]
    fn adler32_known_data() {
        // Test with known data to verify algorithm
        let data = b"Hello, World!";
        let result = adler32(data);
        // Just verify it produces a non-init value
        assert_ne!(result, ADLER32_INIT);
    }

    #[test]
    fn adler32_incremental() {
        // Computing adler32 over two chunks should equal computing over combined data
        let data = b"Hello, World!";
        let full = adler32(data);

        let part1 = adler32_with_init(&data[..5], ADLER32_INIT);
        let incremental = adler32_with_init(&data[5..], part1);

        assert_eq!(full, incremental);
    }

    #[test]
    fn compress_decompress_roundtrip() {
        let opcode: [u8; 2] = [0x48, 0x30]; // AuthChallenge opcode LE
        let payload = vec![0xAA; 2048]; // Larger than threshold

        let compressed = compress_packet(&opcode, &payload);
        let decompressed = decompress_packet(&compressed).unwrap();

        // Decompressed = opcode + payload
        assert_eq!(&decompressed[..2], &opcode);
        assert_eq!(&decompressed[2..], &payload);
    }

    #[test]
    fn compress_small_data() {
        let opcode: [u8; 2] = [0x01, 0x00];
        let payload = b"test";

        let compressed = compress_packet(&opcode, payload);
        let decompressed = decompress_packet(&compressed).unwrap();

        assert_eq!(&decompressed[..2], &opcode);
        assert_eq!(&decompressed[2..], payload);
    }

    #[test]
    fn decompress_bad_adler_fails() {
        let opcode: [u8; 2] = [0x01, 0x00];
        let payload = b"test data here!";

        let mut compressed = compress_packet(&opcode, payload);
        // Corrupt the compressed adler32 (bytes 8..12)
        compressed[8] ^= 0xFF;

        assert!(decompress_packet(&compressed).is_err());
    }

    #[test]
    fn decompress_too_short_fails() {
        assert!(decompress_packet(&[0; 8]).is_err());
    }

    #[test]
    fn compressed_data_ends_with_sync_marker() {
        // Z_SYNC_FLUSH should produce data ending with 00 00 FF FF
        let opcode: [u8; 2] = [0x83, 0x25]; // EnumCharactersResult
        let payload = vec![0u8; 2048];

        let compressed = compress_packet(&opcode, &payload);
        // Skip the 12-byte header to get the raw deflated data
        let deflated = &compressed[12..];

        // Z_SYNC_FLUSH appends an empty stored block: 00 00 FF FF
        assert!(
            deflated.len() >= 4,
            "compressed data too short: {} bytes",
            deflated.len()
        );
        let last4 = &deflated[deflated.len() - 4..];
        assert_eq!(
            last4,
            &[0x00, 0x00, 0xFF, 0xFF],
            "expected sync marker at end, got {:02X?}",
            last4
        );
    }

    #[test]
    fn compress_large_realistic_packet() {
        // Simulate a realistic EnumCharactersResult (~3600 bytes)
        let opcode: [u8; 2] = [0x83, 0x25];
        let payload = vec![0u8; 3600];

        let compressed = compress_packet(&opcode, &payload);
        let decompressed = decompress_packet(&compressed).unwrap();

        assert_eq!(decompressed.len(), 3602); // opcode(2) + payload(3600)
        assert_eq!(&decompressed[..2], &opcode);
    }

    #[test]
    fn persistent_compressor_multiple_packets() {
        // The WoW client uses a persistent inflate stream per socket.
        // Verify that PacketCompressor produces data that decompresses correctly
        // across multiple calls (the client does NOT create a new decompressor per packet).
        let mut compressor = PacketCompressor::new();

        let opcode1: [u8; 2] = [0xE0, 0x25]; // UpdateActionButtons
        let payload1 = vec![0u8; 1441]; // Realistic size

        let opcode2: [u8; 2] = [0x24, 0x27]; // InitializeFactions
        let payload2 = vec![0u8; 6125]; // Realistic size

        let opcode3: [u8; 2] = [0xCB, 0x27]; // UpdateObject
        let payload3 = vec![0xAA; 15622]; // Realistic size

        let compressed1 = compressor.compress_packet(&opcode1, &payload1);
        let compressed2 = compressor.compress_packet(&opcode2, &payload2);
        let compressed3 = compressor.compress_packet(&opcode3, &payload3);

        // Each compressed packet must end with the Z_SYNC_FLUSH marker 00 00 FF FF
        for (i, c) in [&compressed1, &compressed2, &compressed3].iter().enumerate() {
            let deflated = &c[12..];
            let last4 = &deflated[deflated.len() - 4..];
            assert_eq!(
                last4,
                &[0x00, 0x00, 0xFF, 0xFF],
                "packet {i} missing sync marker"
            );
        }

        // Each packet must decompress independently using its own fresh decompressor,
        // BUT the WoW client uses a persistent decompressor. To verify the persistent
        // stream is correct, we use a single DeflateDecoder that processes all 3 packets
        // sequentially.
        use flate2::Decompress;
        use flate2::FlushDecompress;

        let mut decompressor = Decompress::new(false); // raw deflate

        for (i, (compressed, opcode, payload_len)) in [
            (&compressed1, &opcode1[..], 1441usize),
            (&compressed2, &opcode2[..], 6125),
            (&compressed3, &opcode3[..], 15622),
        ]
        .iter()
        .enumerate()
        {
            let uncompressed_size =
                i32::from_le_bytes([compressed[0], compressed[1], compressed[2], compressed[3]])
                    as usize;
            let deflated = &compressed[12..];

            let mut output = vec![0u8; uncompressed_size + 256];
            let base_in = decompressor.total_in() as usize;
            let base_out = decompressor.total_out() as usize;

            let status = decompressor
                .decompress(deflated, &mut output, FlushDecompress::Sync)
                .unwrap_or_else(|e| panic!("packet {i} decompress failed: {e}"));

            let produced = decompressor.total_out() as usize - base_out;
            output.truncate(produced);

            assert_eq!(
                produced, uncompressed_size,
                "packet {i} size mismatch: expected {uncompressed_size}, got {produced}"
            );
            assert_eq!(
                &output[..2], *opcode,
                "packet {i} opcode mismatch"
            );
        }
    }
}
