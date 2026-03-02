// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Update types for SMSG_UPDATE_OBJECT packets.

use bitflags::bitflags;
use num_derive::{FromPrimitive, ToPrimitive};

/// Type of object update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum UpdateType {
    Values = 0,
    CreateObject = 1,
    CreateObject2 = 2,
    OutOfRangeObjects = 3,
}

bitflags! {
    /// Flags for update fields controlling visibility.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UpdateFieldFlag: u32 {
        const NONE = 0;
        const OWNER = 0x01;
        const PARTY_MEMBER = 0x02;
        const UNIT_ALL = 0x04;
        const EMPATH = 0x08;
    }
}
