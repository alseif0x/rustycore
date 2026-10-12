// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only gameobject entry points.
//!
//! `#1363 slice 4a`: the `CMSG_GAMEOBJ_USE` body moved to
//! `wow_world_entities::handle_game_obj_use_with_catalogs_like_cpp`; the
//! registration stays in the `ApplicationGameObject` area registrar.

mod test_shims;

impl crate::session::WorldSession {
    pub(crate) async fn handle_game_obj_use(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_entities::handle_game_obj_use_with_catalogs_like_cpp(
            self,
            &catalogs,
            generator.as_ref(),
            &item_valuation,
            pkt,
        )
        .await;
    }

    // Method-call entry points the loot and character scenarios keep using.
    pub(crate) fn use_represented_gameobject_goober_state_like_cpp(
        &mut self,
        gameobject_guid: wow_core::ObjectGuid,
        player_guid: wow_core::ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::GooberUseSource,
    ) -> bool {
        wow_world_entities::use_represented_gameobject_goober_state_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            gameobject_entry,
            source,
        )
    }

    pub(crate) fn record_represented_gameobject_owner_guid_like_cpp(
        &mut self,
        guid: wow_core::ObjectGuid,
        owner_guid: wow_core::ObjectGuid,
    ) {
        wow_world_entities::record_represented_gameobject_owner_guid_like_cpp(
            self, guid, owner_guid,
        )
    }

    pub(crate) fn use_represented_gameobject_barber_chair_like_cpp(
        &mut self,
        gameobject_guid: wow_core::ObjectGuid,
        player_guid: wow_core::ObjectGuid,
        gameobject_position: wow_core::Position,
        source: wow_entities::BarberChairUseSource,
    ) -> bool {
        wow_world_entities::use_represented_gameobject_barber_chair_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            gameobject_position,
            source,
        )
    }
}
