//! Narrow Character integration fixture operations, scoped to Session visibility.

use super::*;

mod identity;
mod progression;
mod persistence;
mod world_entry;
mod canonical_owner;

pub use canonical_owner::insert_character_fixture_player_into_canonical_map;

pub use canonical_owner::CharacterSaveRuntimeForTest;
