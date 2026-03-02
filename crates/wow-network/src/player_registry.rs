// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared registry of active player sessions for broadcast purposes.
//!
//! Each WorldSession registers itself here on player login and removes itself
//! on logout/disconnect. Chat, emote and movement handlers use the registry
//! to fan-out packets to nearby players on the same map.

use dashmap::DashMap;
use wow_core::{ObjectGuid, Position};

/// Information stored for each active player session.
#[derive(Clone)]
pub struct PlayerBroadcastInfo {
    /// Map ID the player is currently on.
    pub map_id: u16,
    /// Server-side world position (updated on every movement packet).
    pub position: Position,
    /// Channel used to push serialised packets to this player's socket.
    pub send_tx: flume::Sender<Vec<u8>>,
    /// Character name — used for whisper target lookups.
    pub player_name: String,
    /// Account ID — kept for future same-account filtering.
    pub account_id: u32,
    // ── Character attributes for broadcast packets ──
    /// Race (human, dwarf, etc.)
    pub race: u8,
    /// Class (warrior, mage, etc.)
    pub class: u8,
    /// Sex (0=male, 1=female)
    pub sex: u8,
    /// Character level
    pub level: u8,
    /// Display ID for model rendering
    pub display_id: u32,
    /// Equipped item display info: (item_entry, enchant_display_id, subclass) per slot 0-18
    pub visible_items: [(i32, u16, u16); 19],
}

/// Thread-safe registry of all active player sessions, keyed by player GUID.
///
/// Wrap in `Arc` and share between all `WorldSession` instances and the
/// `SessionResources` passed to `create_session`.
pub type PlayerRegistry = DashMap<ObjectGuid, PlayerBroadcastInfo>;
