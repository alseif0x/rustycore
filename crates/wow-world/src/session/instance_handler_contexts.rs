// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! WorldSession's borrowed adapters for the application-owned instance handlers.

use wow_world_application::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceRaidInfoHandlerCxLikeCpp, InstancesHandlerHostLikeCpp,
};

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn build_instance_raid_info_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> InstanceRaidInfoHandlerCxLikeCpp<'a> {
        let player = self.core.instance_player_access_like_cpp();
        let locks = self
            .core
            .instance_lock_manager_access_like_cpp(&self.config);
        let packets = self.core.packet_publication_access_like_cpp();
        let map_store = self.catalogs.map_store().map(AsRef::as_ref);
        let map_difficulty_store = self.catalogs.map_difficulty_store().map(AsRef::as_ref);
        InstanceRaidInfoHandlerCxLikeCpp::new(
            player,
            locks,
            packets,
            map_store,
            map_difficulty_store,
        )
    }

    pub(crate) fn build_instance_lock_operations_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> InstanceLockOperationsHandlerCxLikeCpp<'a> {
        let player = self.core.instance_player_access_like_cpp();
        let locks = self
            .core
            .instance_lock_manager_access_like_cpp(&self.config);
        let group_owner = self.core.player_group_owner_access_like_cpp();
        let groups = self.core.group_difficulty_access_like_cpp();
        let packets = self.core.packet_publication_access_like_cpp();
        let map_store = self.catalogs.map_store().map(AsRef::as_ref);
        let map_difficulty_store = self.catalogs.map_difficulty_store().map(AsRef::as_ref);
        let difficulty_store = self.catalogs.difficulty_store().map(AsRef::as_ref);
        #[cfg(any(test, feature = "test-fixtures"))]
        let game_master_fixture = &self.fixtures.combat.player_game_master_like_cpp;
        #[cfg(any(test, feature = "test-fixtures"))]
        let rejected_response_counter =
            &mut self.fixtures.combat.represented_repop_at_graveyard_count;
        InstanceLockOperationsHandlerCxLikeCpp::new(
            &mut self.instances,
            player,
            locks,
            group_owner,
            groups,
            &self.social,
            &self.lifecycle,
            packets,
            map_store,
            map_difficulty_store,
            difficulty_store,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            game_master_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            rejected_response_counter,
        )
    }

    pub(crate) fn build_instance_difficulty_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> InstanceDifficultyHandlerCxLikeCpp<'a> {
        let player = self.core.instance_player_access_like_cpp();
        let locks = self
            .core
            .instance_lock_manager_access_like_cpp(&self.config);
        let group_owner = self.core.player_group_owner_access_like_cpp();
        let groups = self.core.group_difficulty_access_like_cpp();
        let packets = self.core.packet_publication_access_like_cpp();
        let map_store = self.catalogs.map_store().map(AsRef::as_ref);
        let map_difficulty_store = self.catalogs.map_difficulty_store().map(AsRef::as_ref);
        let difficulty_store = self.catalogs.difficulty_store().map(AsRef::as_ref);
        InstanceDifficultyHandlerCxLikeCpp::new(
            &mut self.instances,
            player,
            locks,
            group_owner,
            groups,
            &self.social,
            &self.lifecycle,
            packets,
            map_store,
            map_difficulty_store,
            difficulty_store,
            cfg!(test),
        )
    }
}

impl InstancesHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn instance_raid_info_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> InstanceRaidInfoHandlerCxLikeCpp<'a> {
        self.build_instance_raid_info_handler_cx_like_cpp()
    }

    fn instance_lock_operations_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> InstanceLockOperationsHandlerCxLikeCpp<'a> {
        self.build_instance_lock_operations_handler_cx_like_cpp()
    }

    fn instance_difficulty_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> InstanceDifficultyHandlerCxLikeCpp<'a> {
        self.build_instance_difficulty_handler_cx_like_cpp()
    }
}
