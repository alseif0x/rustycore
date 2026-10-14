//! The World session as the host of the aura bodies moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method by path; the accessor is the split function.

use super::*;
use wow_world_core::session::HubMut;
use wow_world_entities::WorldEntitiesState;

impl wow_world_spell::AuraHostLikeCpp for WorldSession {
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>) {
        crate::session::split_world_entities_mut(self)
    }
    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        WorldSession::remove_aura(self, slot)
    }
    fn represented_mount_capability_selection_for_type_like_cpp(
        &self,
        mount_type_id: u16,
        riding_skill: u32,
        mount_restriction_flags: Option<u8>,
        is_submerged: bool,
        is_in_water: bool,
    ) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp> {
        WorldSession::represented_mount_capability_selection_for_type_like_cpp(
            self,
            mount_type_id,
            riding_skill,
            mount_restriction_flags,
            is_submerged,
            is_in_water,
        )
    }
    fn represented_mount_x_display_usable_like_cpp(&self, player_condition_id: u32) -> bool {
        WorldSession::represented_mount_x_display_usable_like_cpp(self, player_condition_id)
    }
    fn update_player_collision_height_like_cpp(&mut self) {
        WorldSession::update_player_collision_height_like_cpp(self)
    }
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid> {
        WorldSession::next_represented_spell_cast_guid_like_cpp(self, spell_id)
    }
    fn canonical_threat_aura_snapshot_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        difficulty: u8,
        effect_mask: u32,
        represented_effect_amounts: &[RepresentedAuraEffectAmountLikeCpp],
    ) -> wow_entities::AuraThreatSnapshotLikeCpp {
        WorldSession::canonical_threat_aura_snapshot_for_difficulty_like_cpp(
            self,
            spell_id,
            difficulty,
            effect_mask,
            represented_effect_amounts,
        )
    }
    fn tick_represented_creature_auras_like_cpp(&mut self) {
        WorldSession::tick_represented_creature_auras_like_cpp(self)
    }
    fn represented_spell_area_quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        WorldSession::represented_spell_area_quest_status_like_cpp(self, quest_id)
    }
    fn player_spell_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerSpellRuntimeLikeCpp> {
        WorldSession::player_spell_runtime_snapshot_like_cpp(self)
    }
}
