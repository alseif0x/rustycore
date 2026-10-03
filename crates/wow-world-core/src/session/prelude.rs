use std::sync::{Arc, Mutex};

pub const AFLAG_SCALABLE_LIKE_CPP: u32 = 0x0000_0008;
pub const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = 0x0000_0002;
pub const PLAYER_FLAGS_DND_LIKE_CPP: u32 = 0x0000_0004;
pub const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
pub const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;
pub const PLAYER_FLAGS_RESTING_LIKE_CPP: u32 = 0x0000_0020;
pub const SKILL_ENCHANTING_LIKE_CPP: u16 = 333;

pub type SharedCanonicalMapManager = Arc<Mutex<wow_map::MapManager>>;
