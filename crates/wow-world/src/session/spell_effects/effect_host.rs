//! The World session as the host of the spell-effect bodies moved to
//! `wow_world_spell` (#1363 slice 9c). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;
use std::future::Future;

impl wow_world_spell::SpellEffectHostLikeCpp for WorldSession {
    fn publish_spell_pull_attack_start_like_cpp(
        &mut self,
        attacker: ObjectGuid,
        victim: ObjectGuid,
    ) {
        wow_world_spell::publish_spell_pull_attack_start_like_cpp(self, attacker, victim)
    }
    fn broadcast_creature_packet_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    ) {
        wow_world_entities::broadcast_creature_packet_to_visible_set_like_cpp(
            self,
            source_guid,
            bytes,
        )
    }
    fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_transmog_set_like_cpp(&mut hub, transmog_set_id)
    }
    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        wow_world_instances::group_membership::resolved_group_guid_like_cpp(self)
    }
    fn on_creature_killed_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::on_creature_killed_with_generator_like_cpp(
            self,
            item_guid_generator,
            creature_entry,
            creature_guid,
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
        wow_world_application::apply_damage_from_caster_like_cpp(
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
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.upgrade_account_heirloom_like_cpp(&mut hub, item_id, cast_item)
    }
    fn persist_player_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp) {
        wow_world_spell::persist_player_homebind_like_cpp(self, homebind)
    }
    fn set_represented_homebind_like_cpp(&mut self, homebind: RepresentedHomebindLikeCpp) -> bool {
        wow_world_spell::set_represented_homebind_like_cpp(self, homebind)
    }
    fn represented_visibility_source_combat_reach_like_cpp(&self) -> f32 {
        wow_world_entities::represented_visibility_source_combat_reach_like_cpp(self)
    }
}

impl wow_world_application::CreatureDamageHostLikeCpp for WorldSession {
    fn current_group_member_guids_for_tap_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        wow_world_instances::current_group_member_guids_for_tap_like_cpp(self, player_guid)
    }
}
