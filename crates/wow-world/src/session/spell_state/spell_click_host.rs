//! The World session as the host of the spell-click clickee caster moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;

impl wow_world_spell::SpellClickHostLikeCpp for WorldSession {
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
