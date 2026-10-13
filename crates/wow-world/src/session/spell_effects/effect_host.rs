//! The World session as the host of the spell-effect bodies moved to
//! `wow_world_spell` (#1363 slice 9c). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;
use std::future::Future;

impl wow_world_spell::SpellEffectHostLikeCpp for WorldSession {
    fn forward_heal_threat_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effective_heal: u32,
    ) {
        WorldSession::forward_heal_threat_like_cpp(
            self,
            spell_id,
            healer_guid,
            target_guid,
            effective_heal,
        )
    }
    fn forward_assisting_threat_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        assistant_guid: ObjectGuid,
        assisted_guid: ObjectGuid,
        base_amount: f32,
        ignore_modifiers: bool,
    ) {
        WorldSession::forward_assisting_threat_like_cpp(
            self,
            spell_id,
            assistant_guid,
            assisted_guid,
            base_amount,
            ignore_modifiers,
        )
    }
    fn canonical_threatened_by_me_owner_guids_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        WorldSession::canonical_threatened_by_me_owner_guids_like_cpp(self, target_guid)
    }
    fn scale_canonical_owner_threat_to_target_zero_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) {
        WorldSession::scale_canonical_owner_threat_to_target_zero_like_cpp(
            self,
            owner_guid,
            target_guid,
        )
    }
    fn apply_damage_from_caster_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        damage_amount: u32,
        cast_id: ObjectGuid,
        spell_visual_id: u32,
    ) -> impl Future<Output = Result<(), &'static str>> + Send {
        WorldSession::apply_damage_from_caster_like_cpp(
            self,
            item_guid_generator,
            spell_id,
            caster_guid,
            target_guid,
            damage_amount,
            cast_id,
            spell_visual_id,
        )
    }
    fn teleport_to(&mut self, new_map: u32, new_pos: Position) -> impl Future<Output = ()> + Send {
        WorldSession::teleport_to(self, new_map, new_pos)
    }
    fn upgrade_account_heirloom_like_cpp(
        &mut self,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        WorldSession::upgrade_account_heirloom_like_cpp(self, item_id, cast_item)
    }
    fn persist_player_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp) {
        WorldSession::persist_player_homebind_like_cpp(self, homebind)
    }
    fn represented_visibility_source_combat_reach_like_cpp(&self) -> f32 {
        WorldSession::represented_visibility_source_combat_reach_like_cpp(self)
    }
}
