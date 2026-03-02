// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World of Warcraft packet serialization and deserialization.
//!
//! This crate provides the core packet infrastructure for the world server:
//! - [`WorldPacket`] — Binary buffer with typed read/write and bit packing
//! - [`PacketHeader`] — 16-byte wire header (size + GCM auth tag)
//! - Compression for outbound packets > 1024 bytes
//! - Traits for typed client/server packet definitions

pub mod compression;
pub mod header;
pub mod packets;
pub mod world_packet;

pub use compression::{compress_packet, decompress_packet};
pub use header::PacketHeader;
pub use world_packet::{PacketError, WorldPacket};

use wow_constants::{ClientOpcodes, ServerOpcodes};

/// Trait for packets received from the client.
pub trait ClientPacket: Sized {
    /// The opcode identifying this packet type.
    const OPCODE: ClientOpcodes;

    /// Deserialize from a world packet buffer.
    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError>;
}

/// Trait for packets sent to the client.
pub trait ServerPacket {
    /// The opcode identifying this packet type.
    const OPCODE: ServerOpcodes;

    /// Serialize into a world packet buffer.
    fn write(&self, packet: &mut WorldPacket);

    /// Convenience: serialize to a complete byte vector (opcode + payload).
    fn to_bytes(&self) -> Vec<u8> {
        let mut pkt = WorldPacket::new_server(Self::OPCODE);
        self.write(&mut pkt);
        pkt.into_data()
    }
}
