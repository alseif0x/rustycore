// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Packet header: 16-byte wire format (4-byte size + 12-byte GCM auth tag).

use crate::world_packet::MAX_PACKET_SIZE;

/// Size of the wire header in bytes.
pub const HEADER_SIZE: usize = 16;

/// Size of the GCM authentication tag in the header.
pub const TAG_SIZE: usize = 12;

/// 16-byte packet header sent on the wire.
///
/// Format: `[Size: i32 LE][Tag: 12 bytes]`
///
/// - `size`: Total size of the encrypted data following the header.
/// - `tag`: AES-128-GCM authentication tag (truncated to 12 bytes).
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    pub size: i32,
    pub tag: [u8; TAG_SIZE],
}

impl PacketHeader {
    /// Create a new header with the given size and tag.
    pub fn new(size: i32, tag: [u8; TAG_SIZE]) -> Self {
        Self { size, tag }
    }

    /// Create an empty header (size=0, tag=zeros).
    pub fn zeroed() -> Self {
        Self {
            size: 0,
            tag: [0; TAG_SIZE],
        }
    }

    /// Read a header from a 16-byte buffer.
    pub fn read(buf: &[u8; HEADER_SIZE]) -> Self {
        let size = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(&buf[4..16]);
        Self { size, tag }
    }

    /// Write the header into a 16-byte buffer.
    pub fn write(&self, buf: &mut [u8; HEADER_SIZE]) {
        buf[0..4].copy_from_slice(&self.size.to_le_bytes());
        buf[4..16].copy_from_slice(&self.tag);
    }

    /// Write the header and return a new 16-byte array.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        self.write(&mut buf);
        buf
    }

    /// Check whether the packet size is valid (> 0 and <= MAX_PACKET_SIZE).
    pub fn is_valid_size(&self) -> bool {
        self.size > 0 && (self.size as usize) <= MAX_PACKET_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let tag = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let hdr = PacketHeader::new(1024, tag);

        let bytes = hdr.to_bytes();
        let hdr2 = PacketHeader::read(&bytes);

        assert_eq!(hdr2.size, 1024);
        assert_eq!(hdr2.tag, tag);
    }

    #[test]
    fn header_valid_size() {
        assert!(PacketHeader::new(1, [0; TAG_SIZE]).is_valid_size());
        assert!(PacketHeader::new(MAX_PACKET_SIZE as i32, [0; TAG_SIZE]).is_valid_size());
        assert!(!PacketHeader::new(0, [0; TAG_SIZE]).is_valid_size());
        assert!(!PacketHeader::new(-1, [0; TAG_SIZE]).is_valid_size());
        assert!(!PacketHeader::new(MAX_PACKET_SIZE as i32 + 1, [0; TAG_SIZE]).is_valid_size());
    }

    #[test]
    fn header_zeroed() {
        let hdr = PacketHeader::zeroed();
        assert_eq!(hdr.size, 0);
        assert_eq!(hdr.tag, [0; TAG_SIZE]);
    }
}
