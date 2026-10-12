// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private pvp capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 remaining families`: the `CMSG_BATTLEMASTER_JOIN_SKIRMISH` and
//! `CMSG_ACCEPT_WARGAME_INVITE` registrations moved to the
//! `ApplicationBattleground` area registrar; #1363 slice 4b moved the bodies to
//! `wow_world_social::battlemaster_skirmish`. Only the test entry points stay.

impl crate::session::WorldSession {
    #[cfg(test)]
    fn battlemaster_list_catalog_for_test_like_cpp(
        &self,
    ) -> std::sync::Arc<wow_data::BattlemasterListStore> {
        self.battlemaster_list_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                std::sync::Arc::new(wow_data::BattlemasterListStore::from_entries([]))
            })
    }

    #[cfg(test)]
    pub(crate) async fn handle_battlemaster_join_skirmish(&mut self, pkt: wow_packet::WorldPacket) {
        let catalog = self.battlemaster_list_catalog_for_test_like_cpp();
        wow_world_social::handle_battlemaster_join_skirmish_with_catalog_like_cpp(
            self,
            catalog.as_ref(),
            pkt,
        )
        .await;
    }
}
