//! Reader and writer for the C++ TrinityCore **PKT 3.1** binary packet log.
//!
//! This is the format produced by `PacketLogFile` in the legacy worldserver
//! (`/home/server/woltk-trinity-legacy/src/server/game/Server/Protocol/PacketLog.cpp`).
//! Byte layout is anchored to that source:
//!
//! `LogHeader` (file preamble, `#pragma pack(1)`):
//! ```text
//!   char     Signature[3]        // "PKT"
//!   uint16   FormatVersion       // 0x0301
//!   uint8    SnifferId           // 'T'
//!   uint32   Build
//!   char     Locale[4]           // "enUS"
//!   uint8    SessionKey[40]
//!   uint32   SniffStartUnixtime
//!   uint32   SniffStartTicks
//!   uint32   OptionalDataSize    // 0 for the file header
//! ```
//!
//! `PacketHeader` (one per packet, `#pragma pack(1)`):
//! ```text
//!   uint32   Direction           // 0x47534d43 = CMSG (c2s), 0x47534d53 = SMSG (s2c)
//!   uint32   ConnectionId        // ConnectionType (realm/instance)
//!   uint32   ArrivalTicks
//!   uint32   OptionalDataSize    // 20 in TC output (sizeof OptionalData)
//!   uint32   Length              // body length + 4 (opcode)
//!   <OptionalDataSize bytes>     // { uint8 SocketIPBytes[16]; uint32 SocketPort; }
//!   uint32   Opcode              // 16-bit opcode widened to 32
//!   <Length - 4 bytes>           // body WITHOUT the opcode
//! ```
//!
//! C++ stores the opcode in the header and the body **without** the 2-byte
//! opcode prefix for both directions, so the parsed body is already normalized.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::model::{Capture, CapturedPacket, Direction};

const SIGNATURE: &[u8; 3] = b"PKT";
const FORMAT_VERSION_3_1: u16 = 0x0301;
const DIR_CMSG: u32 = 0x4753_4d43; // 'C''M''S''G'
const DIR_SMSG: u32 = 0x4753_4d53; // 'S''M''S''G'

/// A minimal little-endian byte cursor over an in-memory PKT buffer.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Cursor<'a> {
        Cursor { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            bail!(
                "unexpected end of PKT data: need {n} bytes at offset {}, have {}",
                self.pos,
                self.remaining()
            );
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Read a PKT file from disk into a normalized [`Capture`].
pub fn parse_pkt_file(path: &Path) -> Result<Capture> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading PKT capture {}", path.display()))?;
    let mut cap = parse_pkt_bytes(&bytes)
        .with_context(|| format!("parsing PKT capture {}", path.display()))?;
    cap.source = path.display().to_string();
    Ok(cap)
}

/// Parse an in-memory PKT 3.1 buffer into a normalized [`Capture`].
pub fn parse_pkt_bytes(bytes: &[u8]) -> Result<Capture> {
    let mut cur = Cursor::new(bytes);

    // ── LogHeader ────────────────────────────────────────────────────
    let sig = cur.take(3)?;
    if sig != SIGNATURE {
        bail!("not a PKT capture: bad signature {sig:02X?} (expected \"PKT\")");
    }
    let version = cur.u16()?;
    if version != FORMAT_VERSION_3_1 {
        bail!("unsupported PKT format version 0x{version:04X} (expected 0x0301)");
    }
    let _sniffer_id = cur.u8()?;
    let _build = cur.u32()?;
    let _locale = cur.take(4)?;
    let _session_key = cur.take(40)?;
    let _start_unixtime = cur.u32()?;
    let _start_ticks = cur.u32()?;
    let header_optional_size = cur.u32()? as usize;
    let _ = cur.take(header_optional_size)?;

    // ── PacketHeader records ─────────────────────────────────────────
    let mut packets = Vec::new();
    while cur.remaining() > 0 {
        let direction_raw = cur.u32()?;
        let direction = match direction_raw {
            DIR_CMSG => Direction::C2S,
            DIR_SMSG => Direction::S2C,
            other => bail!(
                "unknown PKT direction 0x{other:08X} at offset {}",
                cur.pos - 4
            ),
        };
        let _connection_id = cur.u32()?;
        let _arrival_ticks = cur.u32()?;
        let optional_size = cur.u32()? as usize;
        let length = cur.u32()? as usize;
        let _ = cur.take(optional_size)?;
        if length < 4 {
            bail!("PKT packet Length {length} < 4 (no room for opcode)");
        }
        let opcode = cur.u32()?;
        let body = cur.take(length - 4)?.to_vec();
        packets.push(CapturedPacket {
            direction,
            opcode: opcode as u16,
            body,
        });
    }

    Ok(Capture::new("<pkt bytes>", packets))
}

/// Serialize a [`Capture`] back to PKT 3.1 bytes.
///
/// Used by the fixture generator so the committed golden capture is a real,
/// byte-faithful PKT file that the parser round-trips. Timestamps are fixed
/// constants (the runtime forbids wall-clock reads), which keeps output
/// deterministic — they are metadata the diff ignores anyway.
#[must_use]
pub fn write_pkt_bytes(capture: &Capture) -> Vec<u8> {
    // OptionalData per packet mirrors TC: 16-byte IP + 4-byte port = 20 bytes.
    const OPTIONAL_SIZE: u32 = 20;

    let mut out = Vec::new();

    // LogHeader
    out.extend_from_slice(SIGNATURE);
    out.extend_from_slice(&FORMAT_VERSION_3_1.to_le_bytes());
    out.push(b'T'); // SnifferId
    out.extend_from_slice(&49_741_u32.to_le_bytes()); // Build (3.4.3 era), arbitrary
    out.extend_from_slice(b"enUS"); // Locale
    out.extend_from_slice(&[0u8; 40]); // SessionKey
    out.extend_from_slice(&0u32.to_le_bytes()); // SniffStartUnixtime (deterministic)
    out.extend_from_slice(&0u32.to_le_bytes()); // SniffStartTicks (deterministic)
    out.extend_from_slice(&0u32.to_le_bytes()); // OptionalDataSize (file header)

    for (index, pkt) in capture.packets.iter().enumerate() {
        let direction_raw = match pkt.direction {
            Direction::C2S => DIR_CMSG,
            Direction::S2C => DIR_SMSG,
        };
        out.extend_from_slice(&direction_raw.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // ConnectionId (realm)
        out.extend_from_slice(&(index as u32).to_le_bytes()); // ArrivalTicks (deterministic)
        out.extend_from_slice(&OPTIONAL_SIZE.to_le_bytes());
        let length = pkt.body.len() as u32 + 4;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&[0u8; 20]); // OptionalData: zeroed IP + port
        out.extend_from_slice(&u32::from(pkt.opcode).to_le_bytes());
        out.extend_from_slice(&pkt.body);
    }

    out
}
