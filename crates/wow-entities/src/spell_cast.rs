//! Data retained by a cast or a queued player request, without packet serialization.
//! C++: Spell.h / SpellCastTargets and Player::_pendingSpellCastRequest.

use std::time::Instant;
use wow_core::{ObjectGuid, Position};

/// Additional spell cast metadata that C++ stores on `Spell` before `prepare`.
///
/// Default values preserve the represented normal-cast path: `OriginalCastID`
/// is the same as `CastID`, `CastFlagsEx` is zero, and no item entry/misc data
/// is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastBattlePetItemModifiersLikeCpp {
    /// Stable identity of the caged item consumed by C++
    /// `SPELL_EFFECT_UNCAGE_BATTLEPET`.
    pub source_item_guid: ObjectGuid,
    pub species_id: u32,
    pub breed_data: u32,
    pub level: u16,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastMetadata {
    pub from_client: bool,
    /// Overrides the visible/effect caster for represented triggered casts.
    /// Normal player casts leave this empty and use the logged-in player GUID.
    pub caster_guid_override: Option<ObjectGuid>,
    /// C++ `SpellCastData::CastFlags` for the emitted `SMSG_SPELL_GO`.
    pub cast_flags: u32,
    pub misc: [i32; 2],
    pub cast_item_entry: Option<u32>,
    pub cast_item_battle_pet_modifiers: Option<SpellCastBattlePetItemModifiersLikeCpp>,
    pub cast_flags_ex: u32,
    pub original_cast_id: ObjectGuid,
    pub unit_target_battle_pet_companion_guid: Option<ObjectGuid>,
    pub restore_last_spell_cast_time_on_power_failure: bool,
    pub previous_last_spell_cast_time_on_power_failure: Option<Instant>,
}

impl Default for SpellCastMetadata {
    fn default() -> Self {
        Self {
            from_client: false,
            caster_guid_override: None,
            cast_flags: 0,
            misc: [0, 0],
            cast_item_entry: None,
            cast_item_battle_pet_modifiers: None,
            cast_flags_ex: 0,
            original_cast_id: ObjectGuid::EMPTY,
            unit_target_battle_pet_companion_guid: None,
            restore_last_spell_cast_time_on_power_failure: false,
            previous_last_spell_cast_time_on_power_failure: None,
        }
    }
}

impl SpellCastMetadata {
    pub fn original_cast_id_or(self, cast_id: ObjectGuid) -> ObjectGuid {
        if self.original_cast_id.is_empty() {
            cast_id
        } else {
            self.original_cast_id
        }
    }
}

/// C++ SpellCastTargets location and optional transport-relative coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpellCastLocationLikeCpp {
    pub transport: ObjectGuid,
    pub position: Position,
}

/// Retained target values; decoding and encoding belong to the packet adapter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellCastTargetsLikeCpp {
    pub flags: u32,
    pub unit: ObjectGuid,
    pub item: ObjectGuid,
    pub src_location: Option<SpellCastLocationLikeCpp>,
    pub dst_location: Option<SpellCastLocationLikeCpp>,
    pub orientation: Option<f32>,
    pub map_id: Option<i32>,
    pub name: String,
}

/// C++ SpellDefines.h::SpellCastVisual plus retained legacy script-visual evidence.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCastVisualLikeCpp {
    pub spell_visual_id: u32,
    pub script_visual_id: u32,
}

#[derive(Debug, Clone)]
pub struct SpellCastState {
    pub spell_id: i32,
    pub target_guid: ObjectGuid,
    pub target_data: SpellCastTargetsLikeCpp,
    pub cast_id: ObjectGuid,
    pub cast_start_time: Instant,
    pub cast_time_ms: u32,
    pub spell_visual: SpellCastVisualLikeCpp,
    pub metadata: SpellCastMetadata,
}

/// Represented player-caster queue payload; cancellation does not cancel the active cast.
#[derive(Debug, Clone)]
pub struct PendingSpellCastRequestLikeCpp {
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub casting_unit_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub target_data: SpellCastTargetsLikeCpp,
    pub spell_visual: SpellCastVisualLikeCpp,
    pub metadata: SpellCastMetadata,
}
