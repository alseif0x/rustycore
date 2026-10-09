//! World adapter for the moved gameobject and spell-click visibility refresh scans.
//!
//! The complete scans — skip gates, eligibility reads and the
//! GameObject-before-spell-click publication order — are owned by
//! `wow-world-application::quest::visibility::refresh` (#1263 F4). This adapter
//! only lends the participants and the two packet projections the application
//! crate cannot reach, and holds no scan order of its own.

use super::*;
use wow_world_application::{
    VisibilityRefreshCxLikeCpp, VisibilityRefreshHostLikeCpp,
    update_visible_gameobjects_or_spell_clicks_like_cpp,
};
#[cfg(test)]
use wow_world_application::{
    update_visible_gameobjects_like_cpp, update_visible_spell_clicks_like_cpp,
};

impl WorldSession {
    /// The selected participants the moved scans borrow: visible identities and
    /// canonical gameobject lookup from Core, the represented use-state store,
    /// and the catalogs/quest state the existing eligibility providers consume.
    fn visibility_refresh_cx_like_cpp(&self) -> VisibilityRefreshCxLikeCpp<'_> {
        VisibilityRefreshCxLikeCpp {
            core: &self.core,
            catalogs: &self.catalogs,
            quest_state: &self.quest_state,
            world_entities: &self.world_entities,
            consumer_test: cfg!(test),
        }
    }

    /// The C++ operation is the combined refresh below; these two halves stay
    /// as test entry points for the retained per-scan regressions and have no
    /// production caller of their own.
    #[cfg(test)]
    pub(crate) fn update_visible_gameobjects_like_cpp(&self) -> usize {
        update_visible_gameobjects_like_cpp(self, &self.visibility_refresh_cx_like_cpp())
    }

    pub(crate) fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        update_visible_gameobjects_or_spell_clicks_like_cpp(
            self,
            &self.visibility_refresh_cx_like_cpp(),
        )
    }

    #[cfg(test)]
    pub(crate) fn update_visible_spell_clicks_like_cpp(&self) -> usize {
        update_visible_spell_clicks_like_cpp(self, &self.visibility_refresh_cx_like_cpp())
    }
}

/// Every method delegates to the existing World operation the scan body invoked
/// at that exact point; none adds session behaviour or a second owner.
impl VisibilityRefreshHostLikeCpp for WorldSession {
    fn visibility_refresh_gameobject_dynamic_flags_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        self.represented_gameobject_dynamic_flags_for_player_like_cpp(gameobject_entry, state)
    }

    fn visibility_refresh_gameobject_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        dynamic_flags: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        crate::session::represented_gameobject_dynamic_flags_update_like_cpp(
            guid,
            map_id,
            dynamic_flags,
        )
    }

    fn visibility_refresh_unit_npc_flags_update_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        packet_update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UpdateObject {
        self.represented_unit_packet_update_to_update_object_like_cpp(guid, map_id, packet_update)
    }
}
