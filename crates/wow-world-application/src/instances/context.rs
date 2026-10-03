// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed participants for the three existing instance-handler families.

use wow_data::{DifficultyStore, MapDifficultyStore, MapStore};
use wow_world_core::session::{
    GroupDifficultyAccessLikeCpp, InstanceLockManagerAccessLikeCpp,
    InstancePlayerAccessLikeCpp, PacketPublicationAccessLikeCpp,
    PlayerGroupOwnerAccessLikeCpp,
};
use wow_world_instances::InstanceState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;

pub struct InstanceRaidInfoHandlerCxLikeCpp<'a> {
    pub(super) player: InstancePlayerAccessLikeCpp<'a>,
    pub(super) locks: InstanceLockManagerAccessLikeCpp<'a>,
    pub(super) packets: PacketPublicationAccessLikeCpp<'a>,
    pub(super) map_store: Option<&'a MapStore>,
    pub(super) map_difficulty_store: Option<&'a MapDifficultyStore>,
}

impl<'a> InstanceRaidInfoHandlerCxLikeCpp<'a> {
    pub fn new(
        player: InstancePlayerAccessLikeCpp<'a>,
        locks: InstanceLockManagerAccessLikeCpp<'a>,
        packets: PacketPublicationAccessLikeCpp<'a>,
        map_store: Option<&'a MapStore>,
        map_difficulty_store: Option<&'a MapDifficultyStore>,
    ) -> Self {
        Self {
            player,
            locks,
            packets,
            map_store,
            map_difficulty_store,
        }
    }
}

pub struct InstanceLockOperationsHandlerCxLikeCpp<'a> {
    pub(super) instances: &'a mut InstanceState,
    pub(super) player: InstancePlayerAccessLikeCpp<'a>,
    pub(super) locks: InstanceLockManagerAccessLikeCpp<'a>,
    pub(super) group_owner: PlayerGroupOwnerAccessLikeCpp<'a>,
    pub(super) groups: GroupDifficultyAccessLikeCpp<'a>,
    pub(super) social: &'a SessionSocialLimits,
    pub(super) lifecycle: &'a SessionLifecycleState,
    pub(super) packets: PacketPublicationAccessLikeCpp<'a>,
    pub(super) map_store: Option<&'a MapStore>,
    pub(super) map_difficulty_store: Option<&'a MapDifficultyStore>,
    pub(super) difficulty_store: Option<&'a DifficultyStore>,
    pub(super) consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) game_master_fixture: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) rejected_response_counter: &'a mut u32,
}

impl<'a> InstanceLockOperationsHandlerCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instances: &'a mut InstanceState,
        player: InstancePlayerAccessLikeCpp<'a>,
        locks: InstanceLockManagerAccessLikeCpp<'a>,
        group_owner: PlayerGroupOwnerAccessLikeCpp<'a>,
        groups: GroupDifficultyAccessLikeCpp<'a>,
        social: &'a SessionSocialLimits,
        lifecycle: &'a SessionLifecycleState,
        packets: PacketPublicationAccessLikeCpp<'a>,
        map_store: Option<&'a MapStore>,
        map_difficulty_store: Option<&'a MapDifficultyStore>,
        difficulty_store: Option<&'a DifficultyStore>,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        game_master_fixture: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        rejected_response_counter: &'a mut u32,
    ) -> Self {
        Self {
            instances,
            player,
            locks,
            group_owner,
            groups,
            social,
            lifecycle,
            packets,
            map_store,
            map_difficulty_store,
            difficulty_store,
            consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            game_master_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            rejected_response_counter,
        }
    }

    pub(super) fn game_master_like_cpp(&self) -> Option<bool> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.player
                .game_master_with_fixture_like_cpp(self.game_master_fixture)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.player.canonical_game_master_like_cpp()
        }
    }
}

pub struct InstanceDifficultyHandlerCxLikeCpp<'a> {
    pub(super) instances: &'a mut InstanceState,
    pub(super) player: InstancePlayerAccessLikeCpp<'a>,
    pub(super) locks: InstanceLockManagerAccessLikeCpp<'a>,
    pub(super) group_owner: PlayerGroupOwnerAccessLikeCpp<'a>,
    pub(super) groups: GroupDifficultyAccessLikeCpp<'a>,
    pub(super) social: &'a SessionSocialLimits,
    pub(super) lifecycle: &'a SessionLifecycleState,
    pub(super) packets: PacketPublicationAccessLikeCpp<'a>,
    pub(super) map_store: Option<&'a MapStore>,
    pub(super) map_difficulty_store: Option<&'a MapDifficultyStore>,
    pub(super) difficulty_store: Option<&'a DifficultyStore>,
    pub(super) consumer_test: bool,
}

impl<'a> InstanceDifficultyHandlerCxLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instances: &'a mut InstanceState,
        player: InstancePlayerAccessLikeCpp<'a>,
        locks: InstanceLockManagerAccessLikeCpp<'a>,
        group_owner: PlayerGroupOwnerAccessLikeCpp<'a>,
        groups: GroupDifficultyAccessLikeCpp<'a>,
        social: &'a SessionSocialLimits,
        lifecycle: &'a SessionLifecycleState,
        packets: PacketPublicationAccessLikeCpp<'a>,
        map_store: Option<&'a MapStore>,
        map_difficulty_store: Option<&'a MapDifficultyStore>,
        difficulty_store: Option<&'a DifficultyStore>,
        consumer_test: bool,
    ) -> Self {
        Self {
            instances,
            player,
            locks,
            group_owner,
            groups,
            social,
            lifecycle,
            packets,
            map_store,
            map_difficulty_store,
            difficulty_store,
            consumer_test,
        }
    }
}

/// Supplies only the three selected operation contexts to the generic handler
/// registrar. No WorldSession or universal Hub crosses this boundary.
pub trait InstancesHandlerHostLikeCpp<C> {
    fn instance_raid_info_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> InstanceRaidInfoHandlerCxLikeCpp<'a>;

    fn instance_lock_operations_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> InstanceLockOperationsHandlerCxLikeCpp<'a>;

    fn instance_difficulty_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> InstanceDifficultyHandlerCxLikeCpp<'a>;
}
