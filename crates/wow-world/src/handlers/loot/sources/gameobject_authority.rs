use super::*;

impl WorldSession {
    /// World facade over the single App authority for the autostore distance,
    /// display-box and spell-lock gates.
    pub(in crate::handlers::loot) fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        wow_world_loot_ops::represented_gameobject_can_autostore_loot_item_like_cpp(
            &self.loot,
            &self.world_entities,
            crate::session::hub_ref(self),
            &self.known_spells_like_cpp(),
            guid,
            player_guid,
        )
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/handlers/loot/sources/gameobject_authority/f3_shims.rs"]
mod f3_shims;
