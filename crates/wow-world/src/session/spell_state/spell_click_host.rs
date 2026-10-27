//! The World session as the host of the spell-click clickee caster moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;

impl wow_world_spell::SpellClickHostLikeCpp for WorldSession {
    fn apply_native_player_damage_like_cpp<'a>(
        &'a mut self,
        damage: u32,
        context: wow_entities::PlayerDeathOperationContext,
        creature_spawns: &'a wow_world_entities::CreatureSpawnCatalogsLikeCpp,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
    ) -> wow_handler::HandlerFuture<
        'a,
        Result<wow_world_core::session::PlayerDamageResultLikeCpp, &'static str>,
    > {
        wow_world_entities::PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
            self,
            damage,
            context,
            creature_spawns,
            item_guid_generator,
        )
    }
    fn sync_player_registry_state_like_cpp(&self) {
        WorldSession::sync_player_registry_state_like_cpp(self)
    }
    fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        wow_world_quest::unit_update_publication::represented_unit_values_update_to_update_object_like_cpp(
            self,
            unit_guid,
            map_id,
            values_update,
        )
    }
}
