//! Common normalized capture model shared by the PKT (C++) and Rust-dump parsers.
//!
//! Both servers log packets in different on-disk shapes (see [`crate::pkt`] and
//! [`crate::rustdump`]), but the diff engine works on one normalized form: an
//! ordered stream of [`CapturedPacket`]s, each reduced to `(direction, opcode,
//! body)` where `body` is the payload **without** the 2-byte opcode prefix.

use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use wow_constants::opcodes::{ClientOpcodes, ServerOpcodes};

/// Wire direction of a packet, matching the C++ `enum Direction` semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Client to server (`CMSG`, C++ `CLIENT_TO_SERVER`).
    C2S,
    /// Server to client (`SMSG`, C++ `SERVER_TO_CLIENT`).
    S2C,
}

impl Direction {
    /// Short wire tag used by the Rust dump `.meta` files and the CLI.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Direction::C2S => "c2s",
            Direction::S2C => "s2c",
        }
    }

    /// Parse the `c2s` / `s2c` tag emitted by the Rust dumper.
    ///
    /// The live dumper tags pre-encryption handshake packets (AuthChallenge,
    /// AuthSession, EnterEncryptedMode/Ack, the instance handshake) as
    /// `c2s-unencrypted` / `s2c-unencrypted` (`world_socket.rs:639,718`). Those
    /// are the same opcodes the C++ PKT log records as plain CMSG/SMSG, so the
    /// `-unencrypted` suffix is stripped here for alignment.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Direction> {
        match tag.trim().trim_end_matches("-unencrypted") {
            "c2s" => Some(Direction::C2S),
            "s2c" => Some(Direction::S2C),
            _ => None,
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag())
    }
}

/// One normalized packet: direction + opcode + opcode-less body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedPacket {
    /// Wire direction.
    pub direction: Direction,
    /// Numeric opcode (16-bit on the WotLK 3.4.3 wire).
    pub opcode: u16,
    /// Payload bytes **excluding** the 2-byte opcode prefix.
    pub body: Vec<u8>,
}

impl CapturedPacket {
    /// Resolve the human-readable opcode name from the RustyCore opcode tables,
    /// falling back to `Unknown` when the opcode is not in the enum.
    #[must_use]
    pub fn opcode_name(&self) -> String {
        opcode_name(self.direction, self.opcode)
    }
}

/// Resolve an opcode name from `wow-constants`, picking the table by direction.
#[must_use]
pub fn opcode_name(direction: Direction, opcode: u16) -> String {
    let resolved = match direction {
        Direction::C2S => ClientOpcodes::from_u32(u32::from(opcode)).map(|o| format!("{o:?}")),
        Direction::S2C => ServerOpcodes::from_u32(u32::from(opcode)).map(|o| format!("{o:?}")),
    };
    resolved.unwrap_or_else(|| "Unknown".to_string())
}

/// An ordered stream of packets from a single source (one capture).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// Human-readable provenance (file path, dump dir, or fixture name).
    pub source: String,
    /// Packets in capture order.
    pub packets: Vec<CapturedPacket>,
}

impl Capture {
    /// Build a capture from a source label and its packets.
    #[must_use]
    pub fn new(source: impl Into<String>, packets: Vec<CapturedPacket>) -> Capture {
        Capture {
            source: source.into(),
            packets,
        }
    }

    /// Packets of one direction, preserving order.
    #[must_use]
    pub fn by_direction(&self, direction: Direction) -> Vec<&CapturedPacket> {
        self.packets
            .iter()
            .filter(|p| p.direction == direction)
            .collect()
    }

    /// Keep only packets up to and including the first occurrence of `opcode`
    /// (in capture order). Used to trim a full-session capture to a flow
    /// boundary — e.g. the login flow ends at the first
    /// `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` (the client taking control).
    /// Returns the capture unchanged if the opcode never appears.
    #[must_use]
    pub fn truncated_after_first_opcode(&self, opcode: u16) -> Capture {
        match self.packets.iter().position(|p| p.opcode == opcode) {
            Some(pos) => Capture::new(self.source.clone(), self.packets[..=pos].to_vec()),
            None => self.clone(),
        }
    }
}
