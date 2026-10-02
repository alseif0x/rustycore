//! Build-70170 world transport. Caller-owned account admission and persistence
//! precede encryption; this module has no database or gameplay dispatch.
//! Wire/crypto source: advocaite/TrinityCore 02245dcd, WorldSocket and Auth packets.

mod socket;
mod wire;

pub use socket::{ForeverSocket, ForeverSocketError};
pub use wire::Frame;

pub const BUILD: u32 = 70170;
pub const AUTH_CHALLENGE: u32 = 0x4D0000;
pub const AUTH_SESSION: u32 = 0x450001;
pub const ENTER_ENCRYPTED_MODE: u32 = 0x4D0004;
pub const ENTER_ENCRYPTED_MODE_ACK: u32 = 0x450005;
pub const AUTH_RESPONSE: u32 = 0x460001;
pub const PING: u32 = 0x450006;
// Native build-70170 descriptor RVA 0xA1AF10, not the older C++ 0x4C0009.
pub const PONG: u32 = 0x4D0009;

#[cfg(test)]
mod tests;
