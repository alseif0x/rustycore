//! The World session as the host of the aura bodies moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method by path; the accessor is the split function.

use super::*;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_entities::WorldEntitiesState;

impl wow_world_spell::AuraHostLikeCpp for WorldSession {
    fn split_world_entities_ref(&self) -> (&WorldEntitiesState, HubRef<'_>) {
        crate::session::split_world_entities_ref(self)
    }
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>) {
        crate::session::split_world_entities_mut(self)
    }
    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .remove_aura_like_cpp(slot)
    }
    fn apply_loaded_ghost_wisp_aura(
        &mut self,
        aura: &wow_entities::AuraApplicationLikeCpp,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_loaded_ghost_wisp_aura(aura)
    }
    fn represented_mount_x_display_usable_like_cpp(&self, player_condition_id: u32) -> bool {
        self.represented_meets_player_condition_id_like_cpp(player_condition_id)
    }
    fn next_represented_spell_cast_guid_like_cpp(&self, spell_id: i32) -> Option<ObjectGuid> {
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(self, spell_id)
    }
    fn tick_represented_creature_auras_like_cpp(&mut self) {
        wow_world_entities::tick_represented_creature_auras_like_cpp(self)
    }
    fn represented_spell_area_quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        wow_world_quest::represented_spell_area_quest_status_like_cpp(self, quest_id)
    }
}
