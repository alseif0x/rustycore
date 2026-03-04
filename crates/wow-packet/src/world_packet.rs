// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Binary packet buffer with typed read/write and bit-packing support.

use bytes::{BufMut, BytesMut};
use num_traits::ToPrimitive;
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

/// Maximum allowed packet size (256 KiB).
pub const MAX_PACKET_SIZE: usize = 0x40000;

/// Errors that can occur during packet read/write operations.
#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("read past end of packet: wanted {wanted} bytes, only {available} remaining")]
    ReadPastEnd { wanted: usize, available: usize },

    #[error("string read error: {0}")]
    StringError(String),

    #[error("invalid opcode: 0x{0:04X}")]
    InvalidOpcode(u32),

    #[error("packet too large: {size} bytes (max {MAX_PACKET_SIZE})")]
    TooLarge { size: usize },

    #[error("decompression error: {0}")]
    DecompressionError(String),

    #[error("unexpected packet: expected opcode 0x{expected:04X}, got 0x{actual:04X}")]
    UnexpectedOpcode { expected: u32, actual: u32 },
}

/// A binary packet buffer for the WoW protocol.
///
/// Supports sequential read/write of primitive types and bit-packed fields.
/// The first 2 bytes of data are always the opcode (u16 LE).
#[derive(Debug, Clone)]
pub struct WorldPacket {
    data: BytesMut,
    read_pos: usize,
    // Bit-packing state for reading
    bit_buf: u8,
    bit_pos: u8, // bits remaining in bit_buf (0 = need new byte)
    // Bit-packing state for writing
    wbit_buf: u8,
    wbit_pos: u8, // next bit position (0..8), 0 = no pending bits
}

impl WorldPacket {
    // ── Construction ──────────────────────────────────────────────

    /// Create an empty packet (no opcode set).
    pub fn new_empty() -> Self {
        Self {
            data: BytesMut::with_capacity(64),
            read_pos: 0,
            bit_buf: 0,
            bit_pos: 0,
            wbit_buf: 0,
            wbit_pos: 0,
        }
    }

    /// Create a packet from raw bytes (opcode is first 2 bytes).
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            data: BytesMut::from(data),
            read_pos: 0,
            bit_buf: 0,
            bit_pos: 0,
            wbit_buf: 0,
            wbit_pos: 0,
        }
    }

    /// Create a server packet with the given opcode pre-written.
    pub fn new_server(opcode: ServerOpcodes) -> Self {
        let mut pkt = Self::new_empty();
        pkt.data.put_u16_le(opcode.to_u16().unwrap_or(0));
        pkt
    }

    /// Create a client packet from raw data (opcode already in data).
    pub fn new_client(data: BytesMut) -> Self {
        Self {
            data,
            read_pos: 0,
            bit_buf: 0,
            bit_pos: 0,
            wbit_buf: 0,
            wbit_pos: 0,
        }
    }

    // ── Opcode access ─────────────────────────────────────────────

    /// Read the opcode from the first 2 bytes (u16 LE) without advancing read position.
    pub fn opcode_raw(&self) -> u16 {
        if self.data.len() < 2 {
            return 0;
        }
        u16::from_le_bytes([self.data[0], self.data[1]])
    }

    /// Get the client opcode, if valid.
    pub fn client_opcode(&self) -> Option<ClientOpcodes> {
        num_traits::FromPrimitive::from_u32(u32::from(self.opcode_raw()))
    }

    /// Get the server opcode, if valid.
    pub fn server_opcode(&self) -> Option<ServerOpcodes> {
        num_traits::FromPrimitive::from_u32(u32::from(self.opcode_raw()))
    }

    // ── Metadata ──────────────────────────────────────────────────

    /// Total size of the data buffer (including opcode bytes).
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Bytes remaining to read.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.read_pos)
    }

    /// Current read position.
    pub fn read_position(&self) -> usize {
        self.read_pos
    }

    /// Whether all data has been read.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Get a reference to the underlying data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Consume the packet and return the data as a Vec.
    pub fn into_data(self) -> Vec<u8> {
        self.data.to_vec()
    }

    /// Consume the packet and return the raw `BytesMut`.
    pub fn into_inner(self) -> BytesMut {
        self.data
    }

    /// Skip the opcode (2 bytes) at the start — call this before reading payload.
    pub fn skip_opcode(&mut self) {
        self.read_pos = 2.min(self.data.len());
    }

    /// Reset the read position to the beginning.
    pub fn reset_read(&mut self) {
        self.read_pos = 0;
        self.bit_buf = 0;
        self.bit_pos = 0;
    }

    // ── Primitive reads ───────────────────────────────────────────

    fn ensure_readable(&self, n: usize) -> Result<(), PacketError> {
        if self.remaining() < n {
            return Err(PacketError::ReadPastEnd {
                wanted: n,
                available: self.remaining(),
            });
        }
        Ok(())
    }

    pub fn read_uint8(&mut self) -> Result<u8, PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(1)?;
        let v = self.data[self.read_pos];
        self.read_pos += 1;
        Ok(v)
    }

    pub fn read_int8(&mut self) -> Result<i8, PacketError> {
        Ok(self.read_uint8()? as i8)
    }

    pub fn read_uint16(&mut self) -> Result<u16, PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(2)?;
        let v = u16::from_le_bytes([
            self.data[self.read_pos],
            self.data[self.read_pos + 1],
        ]);
        self.read_pos += 2;
        Ok(v)
    }

    pub fn read_int16(&mut self) -> Result<i16, PacketError> {
        Ok(self.read_uint16()? as i16)
    }

    pub fn read_uint32(&mut self) -> Result<u32, PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(4)?;
        let v = u32::from_le_bytes([
            self.data[self.read_pos],
            self.data[self.read_pos + 1],
            self.data[self.read_pos + 2],
            self.data[self.read_pos + 3],
        ]);
        self.read_pos += 4;
        Ok(v)
    }

    pub fn read_int32(&mut self) -> Result<i32, PacketError> {
        Ok(self.read_uint32()? as i32)
    }

    pub fn read_uint64(&mut self) -> Result<u64, PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.data[self.read_pos..self.read_pos + 8]);
        self.read_pos += 8;
        Ok(u64::from_le_bytes(buf))
    }

    pub fn read_int64(&mut self) -> Result<i64, PacketError> {
        Ok(self.read_uint64()? as i64)
    }

    pub fn read_float(&mut self) -> Result<f32, PacketError> {
        let bits = self.read_uint32()?;
        Ok(f32::from_bits(bits))
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>, PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(count)?;
        let v = self.data[self.read_pos..self.read_pos + count].to_vec();
        self.read_pos += count;
        Ok(v)
    }

    /// Read a length-prefixed string (the length is passed in, not read from packet).
    pub fn read_string(&mut self, len: usize) -> Result<String, PacketError> {
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes)
            .map_err(|e| PacketError::StringError(e.to_string()))
    }

    /// Read a null-terminated C string.
    pub fn read_cstring(&mut self) -> Result<String, PacketError> {
        self.reset_bit_reader();
        let start = self.read_pos;
        while self.read_pos < self.data.len() {
            if self.data[self.read_pos] == 0 {
                let s = String::from_utf8(self.data[start..self.read_pos].to_vec())
                    .map_err(|e| PacketError::StringError(e.to_string()))?;
                self.read_pos += 1; // skip null terminator
                return Ok(s);
            }
            self.read_pos += 1;
        }
        // No null terminator found — return what we have
        String::from_utf8(self.data[start..self.read_pos].to_vec())
            .map_err(|e| PacketError::StringError(e.to_string()))
    }

    /// Skip `n` bytes in the read stream.
    pub fn skip(&mut self, n: usize) -> Result<(), PacketError> {
        self.reset_bit_reader();
        self.ensure_readable(n)?;
        self.read_pos += n;
        Ok(())
    }

    // ── Primitive writes ──────────────────────────────────────────

    pub fn write_uint8(&mut self, v: u8) {
        self.flush_bits();
        self.data.put_u8(v);
    }

    pub fn write_int8(&mut self, v: i8) {
        self.write_uint8(v as u8);
    }

    pub fn write_uint16(&mut self, v: u16) {
        self.flush_bits();
        self.data.put_u16_le(v);
    }

    pub fn write_int16(&mut self, v: i16) {
        self.write_uint16(v as u16);
    }

    pub fn write_uint32(&mut self, v: u32) {
        self.flush_bits();
        self.data.put_u32_le(v);
    }

    pub fn write_int32(&mut self, v: i32) {
        self.write_uint32(v as u32);
    }

    pub fn write_uint64(&mut self, v: u64) {
        self.flush_bits();
        self.data.put_u64_le(v);
    }

    pub fn write_int64(&mut self, v: i64) {
        self.write_uint64(v as u64);
    }

    pub fn write_float(&mut self, v: f32) {
        self.write_uint32(v.to_bits());
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        self.flush_bits();
        self.data.put_slice(data);
    }

    /// Write a string WITHOUT a length prefix (caller writes length separately).
    pub fn write_string(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
    }

    /// Write a null-terminated C string.
    pub fn write_cstring(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
        self.write_uint8(0);
    }

    // ── Bit packing (read) ────────────────────────────────────────

    /// Reset the bit reader state. Called automatically before byte reads.
    fn reset_bit_reader(&mut self) {
        self.bit_buf = 0;
        self.bit_pos = 0;
    }

    /// Public reset for callers that need an explicit C#-style `ResetBitPos()`.
    /// Used by nested packet readers (e.g. SpellTargetData) that start with a
    /// fresh bit-field section after the parent stopped reading bits.
    pub fn reset_bits(&mut self) {
        self.reset_bit_reader();
    }

    /// Read a single bit (returns true if 1).
    pub fn read_bit(&mut self) -> Result<bool, PacketError> {
        if self.bit_pos == 0 {
            self.ensure_readable(1)?;
            self.bit_buf = self.data[self.read_pos];
            self.read_pos += 1;
            self.bit_pos = 8;
        }
        self.bit_pos -= 1;
        Ok((self.bit_buf >> self.bit_pos) & 1 == 1)
    }

    /// Read `n` bits as a u32 (MSB first, matches C# ReadBits).
    pub fn read_bits(&mut self, n: u32) -> Result<u32, PacketError> {
        let mut value = 0u32;
        for _ in 0..n {
            value = (value << 1) | u32::from(self.read_bit()?);
        }
        Ok(value)
    }

    /// Alias for `read_bit()` — matches C# `HasBit()`.
    pub fn has_bit(&mut self) -> Result<bool, PacketError> {
        self.read_bit()
    }

    // ── Bit packing (write) ───────────────────────────────────────

    /// Write a single bit.
    pub fn write_bit(&mut self, v: bool) {
        self.wbit_pos += 1;
        if v {
            self.wbit_buf |= 1 << (8 - self.wbit_pos);
        }
        if self.wbit_pos == 8 {
            self.data.put_u8(self.wbit_buf);
            self.wbit_buf = 0;
            self.wbit_pos = 0;
        }
    }

    /// Write `n` bits from a u32 value (MSB first).
    pub fn write_bits(&mut self, value: u32, n: u32) {
        for i in (0..n).rev() {
            self.write_bit((value >> i) & 1 == 1);
        }
    }

    /// Flush any pending bits to the buffer (pad remaining with 0).
    pub fn flush_bits(&mut self) {
        if self.wbit_pos > 0 {
            self.data.put_u8(self.wbit_buf);
            self.wbit_buf = 0;
            self.wbit_pos = 0;
        }
    }

    // ── Packed GUID ─────────────────────────────────────────────────

    /// Write a packed ObjectGuid (128-bit) to the packet.
    ///
    /// Format: `[low_mask: u8][high_mask: u8][non-zero low bytes][non-zero high bytes]`
    ///
    /// Each mask byte has bits set for non-zero bytes in the corresponding i64.
    /// Only the non-zero bytes are written, in little-endian byte order.
    pub fn write_packed_guid(&mut self, guid: &ObjectGuid) {
        self.flush_bits();

        let low = guid.low_value() as u64;
        let high = guid.high_value() as u64;

        let low_bytes = low.to_le_bytes();
        let high_bytes = high.to_le_bytes();

        let mut low_mask: u8 = 0;
        let mut high_mask: u8 = 0;

        for i in 0..8 {
            if low_bytes[i] != 0 {
                low_mask |= 1 << i;
            }
            if high_bytes[i] != 0 {
                high_mask |= 1 << i;
            }
        }

        self.data.put_u8(low_mask);
        self.data.put_u8(high_mask);

        for i in 0..8 {
            if low_bytes[i] != 0 {
                self.data.put_u8(low_bytes[i]);
            }
        }
        for i in 0..8 {
            if high_bytes[i] != 0 {
                self.data.put_u8(high_bytes[i]);
            }
        }
    }

    /// Read a packed ObjectGuid (128-bit) from the packet.
    ///
    /// Inverse of [`write_packed_guid`](Self::write_packed_guid).
    pub fn read_packed_guid(&mut self) -> Result<ObjectGuid, PacketError> {
        self.reset_bit_reader();

        let low_mask = self.read_uint8()?;
        let high_mask = self.read_uint8()?;

        let mut low_bytes = [0u8; 8];
        let mut high_bytes = [0u8; 8];

        for i in 0..8 {
            if low_mask & (1 << i) != 0 {
                low_bytes[i] = self.read_uint8()?;
            }
        }
        for i in 0..8 {
            if high_mask & (1 << i) != 0 {
                high_bytes[i] = self.read_uint8()?;
            }
        }

        let low = i64::from_le_bytes(low_bytes);
        let high = i64::from_le_bytes(high_bytes);

        Ok(ObjectGuid::new(high, low))
    }
}

impl Default for WorldPacket {
    fn default() -> Self {
        Self::new_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_primitives() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(0xAB);
        pkt.write_uint16(0x1234);
        pkt.write_uint32(0xDEAD_BEEF);
        pkt.write_uint64(0x0102_0304_0506_0708);
        pkt.write_int8(-42);
        pkt.write_int16(-1000);
        pkt.write_int32(-100_000);
        pkt.write_float(3.14);

        pkt.reset_read();
        assert_eq!(pkt.read_uint8().unwrap(), 0xAB);
        assert_eq!(pkt.read_uint16().unwrap(), 0x1234);
        assert_eq!(pkt.read_uint32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(pkt.read_uint64().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(pkt.read_int8().unwrap(), -42);
        assert_eq!(pkt.read_int16().unwrap(), -1000);
        assert_eq!(pkt.read_int32().unwrap(), -100_000);
        let f = pkt.read_float().unwrap();
        assert!((f - 3.14).abs() < 0.001);
    }

    #[test]
    fn roundtrip_bytes_and_strings() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bytes(&[1, 2, 3, 4, 5]);
        pkt.write_string("hello");
        pkt.write_cstring("world");

        pkt.reset_read();
        assert_eq!(pkt.read_bytes(5).unwrap(), vec![1, 2, 3, 4, 5]);
        assert_eq!(pkt.read_string(5).unwrap(), "hello");
        assert_eq!(pkt.read_cstring().unwrap(), "world");
    }

    #[test]
    fn bit_packing_roundtrip() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.write_bit(true);
        pkt.write_bits(0b1101, 4);
        pkt.flush_bits();

        pkt.reset_read();
        assert!(pkt.read_bit().unwrap());
        assert!(!pkt.read_bit().unwrap());
        assert!(pkt.read_bit().unwrap());
        assert_eq!(pkt.read_bits(4).unwrap(), 0b1101);
    }

    #[test]
    fn bit_then_byte_resets() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.write_uint32(42);

        pkt.reset_read();
        assert!(pkt.read_bit().unwrap());
        assert!(!pkt.read_bit().unwrap());
        // Reading a byte value resets the bit reader
        assert_eq!(pkt.read_uint32().unwrap(), 42);
    }

    #[test]
    fn server_packet_opcode() {
        let pkt = WorldPacket::new_server(ServerOpcodes::AuthChallenge);
        assert_eq!(pkt.opcode_raw(), 0x3048);
        assert_eq!(pkt.size(), 2); // just the opcode
    }

    #[test]
    fn read_past_end_error() {
        let mut pkt = WorldPacket::from_bytes(&[0x01]);
        assert!(pkt.read_uint16().is_err());
    }

    #[test]
    fn skip_and_remaining() {
        let mut pkt = WorldPacket::from_bytes(&[0; 10]);
        assert_eq!(pkt.remaining(), 10);
        pkt.skip(4).unwrap();
        assert_eq!(pkt.remaining(), 6);
        assert_eq!(pkt.read_position(), 4);
    }

    #[test]
    fn write_bits_multi_byte() {
        let mut pkt = WorldPacket::new_empty();
        // Write 10 bits: should span 2 bytes
        pkt.write_bits(0x3FF, 10);
        pkt.flush_bits();

        assert_eq!(pkt.size(), 2);

        pkt.reset_read();
        assert_eq!(pkt.read_bits(10).unwrap(), 0x3FF);
    }

    #[test]
    fn has_bit_alias() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(true);
        pkt.flush_bits();

        pkt.reset_read();
        assert!(pkt.has_bit().unwrap());
    }

    #[test]
    fn packed_guid_empty() {
        let mut pkt = WorldPacket::new_empty();
        let guid = ObjectGuid::EMPTY;
        pkt.write_packed_guid(&guid);

        pkt.reset_read();
        let read_back = pkt.read_packed_guid().unwrap();
        assert_eq!(read_back, guid);
        // Empty guid = 2 mask bytes (both 0), no data bytes
        assert_eq!(pkt.size(), 2);
    }

    #[test]
    fn packed_guid_player_roundtrip() {
        let mut pkt = WorldPacket::new_empty();
        let guid = ObjectGuid::create_player(1, 42);
        pkt.write_packed_guid(&guid);

        pkt.reset_read();
        let read_back = pkt.read_packed_guid().unwrap();
        assert_eq!(read_back, guid);
        assert!(read_back.is_player());
        assert_eq!(read_back.realm_id(), 1);
        assert_eq!(read_back.counter(), 42);
    }

    #[test]
    fn packed_guid_creature_roundtrip() {
        let mut pkt = WorldPacket::new_empty();
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0, 1, 530, 0, 1234, 5678,
        );
        pkt.write_packed_guid(&guid);

        pkt.reset_read();
        let read_back = pkt.read_packed_guid().unwrap();
        assert_eq!(read_back, guid);
        assert!(read_back.is_creature());
        assert_eq!(read_back.map_id(), 530);
        assert_eq!(read_back.entry(), 1234);
        assert_eq!(read_back.counter(), 5678);
    }

    #[test]
    fn packed_guid_fully_populated() {
        let mut pkt = WorldPacket::new_empty();
        // Create a GUID where all bytes of both halves are non-zero
        let guid = ObjectGuid::new(
            0x0807_0605_0403_0201_u64 as i64,
            0x1011_1213_1415_1617_u64 as i64,
        );
        pkt.write_packed_guid(&guid);

        // 2 mask bytes + 16 data bytes (all non-zero)
        assert_eq!(pkt.size(), 18);

        pkt.reset_read();
        let read_back = pkt.read_packed_guid().unwrap();
        assert_eq!(read_back, guid);
    }
}
