use wow_packet::packets::misc::{CufProfile, MAX_CUF_PROFILES_LIKE_CPP};
use wow_world_core::session::{
    HubMut, HubRef, TOY_FLAG_FAVORITE_LIKE_CPP, TOY_FLAG_HAS_FANFARE_LIKE_CPP,
    player_cuf_profile_from_packet_like_cpp,
};
use wow_world_core::session::persistence_capabilities::{
    PlayerSaveToDbSnapshotLikeCpp, loaded_character_power_snapshot_like_cpp,
};

use super::SessionLifecycleState;
use crate::{
    AccountHeirloomSaveRowLikeCpp, AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp,
};

impl SessionLifecycleState {
    /// None permits normal save preparation; Some is a completed admission decision.
    pub fn defer_player_save_for_transfer_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> Option<crate::PlayerSaveOutcomeLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return None; // Existing ownerless persistence fixtures, never production.
        }
        if hub
            .core
            .player_handle_like_cpp
            .is_none_or(|handle| hub.core.player_guid() != Some(handle.guid()))
        {
            return Some(crate::PlayerSaveOutcomeLikeCpp::Unavailable);
        }
        match hub.core.with_owned_player_mut_like_cpp(|player| {
            player.defer_save_if_transfer_pending_like_cpp()
        }) {
            Some(Some(false)) => None,
            Some(Some(true)) => Some(crate::PlayerSaveOutcomeLikeCpp::Deferred),
            Some(None) => {
                hub.core.kick("deferred player-save revision exhausted");
                Some(crate::PlayerSaveOutcomeLikeCpp::Unavailable)
            }
            None => Some(crate::PlayerSaveOutcomeLikeCpp::Unavailable),
        }
    }

    pub fn player_save_header_from_owner_like_cpp(
        &self,
        player: &wow_entities::Player,
        residence: wow_map::PlayerResidenceLikeCpp,
    ) -> PlayerSaveToDbSnapshotLikeCpp {
        let teleport = &player.gameplay_state().teleport;
        let destination = (teleport.recovery != wow_entities::PlayerTransferRecovery::Terminal)
            .then(|| {
                teleport
                    .far_destination
                    .map(|(map, position)| (u16::try_from(map).unwrap_or(u16::MAX), position))
                    .or_else(|| {
                        teleport
                            .near_pending
                            .then_some(teleport.near_destination)
                            .flatten()
                    })
            })
            .flatten();
        let (map_id, instance_id, position) = if let Some((map_id, position)) = destination {
            (map_id, 0, position)
        } else {
            // Player.cpp:19480-19514 reads the Player's location. ResetMap
            // (Object.cpp:1814) retains map/instance even while detached.
            (
                player.unit().world().map_id() as u16,
                player.unit().world().instance_id(),
                player.unit().world().position(),
            )
        };
        let unit = player.unit();
        let max_health = unit.data().max_health.clamp(1, u64::from(u32::MAX)) as u32;
        let health = match residence {
            wow_map::PlayerResidenceLikeCpp::Active(_) => {
                let health = unit.data().health.min(u64::from(u32::MAX)) as u32;
                if unit.is_alive() && health > 0 {
                    health
                } else {
                    0
                }
            }
            wow_map::PlayerResidenceLikeCpp::Detached => {
                unit.data().health.min(u64::from(max_health)) as u32
            }
        };
        PlayerSaveToDbSnapshotLikeCpp {
            guid: player.guid(),
            map_id,
            instance_id,
            position,
            level: unit.data().level as u8, // C++ Unit::GetLevel (Unit.h:733).
            xp: player.active_data().xp.max(0) as u32,
            money: player.money(),
            health,
            max_health,
            powers: loaded_character_power_snapshot_like_cpp(unit.data().power),
        }
    }

    pub fn set_player_save_interval_ms_like_cpp(&mut self, interval_ms: u32) {
        self.player_save_interval_ms_like_cpp = interval_ms;
        self.reset_player_save_timer_like_cpp();
    }

    pub fn pending_periodic_player_save_like_cpp(&self) -> bool {
        self.pending_periodic_player_save_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn next_player_save_ms_like_cpp(&self) -> u32 {
        self.next_player_save_ms_like_cpp
    }

    /// C++ `CollectionMgr::SaveAccountHeirlooms`.
    pub fn account_heirloom_save_rows_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Vec<AccountHeirloomSaveRowLikeCpp>> {
        let bnet_account_id = hub.core.battlenet_account_id();
        Some(
            hub.player_collection_state_snapshot_like_cpp()?
                .heirlooms_like_cpp()
                .into_iter()
                .map(|(item_id, data)| AccountHeirloomSaveRowLikeCpp {
                    bnet_account_id,
                    item_id: *item_id,
                    flags: data.flags,
                })
                .collect(),
        )
    }

    /// C++ `CollectionMgr::SaveAccountToys`.
    pub fn account_toy_save_rows_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Vec<AccountToySaveRowLikeCpp>> {
        let bnet_account_id = hub.core.battlenet_account_id();
        Some(
            hub.player_collection_state_snapshot_like_cpp()?
                .toys_like_cpp()
                .into_iter()
                .map(|(item_id, flags)| AccountToySaveRowLikeCpp {
                    bnet_account_id,
                    item_id: *item_id,
                    is_favorite: (*flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                    has_fanfare: (*flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                })
                .collect(),
        )
    }

    pub fn reset_player_save_timer_like_cpp(&mut self) {
        self.next_player_save_ms_like_cpp = self.player_save_interval_ms_like_cpp;
        self.pending_periodic_player_save_like_cpp = false;
    }

    pub fn update_player_save_timer_like_cpp(&mut self, diff_ms: u32) {
        if self.player_save_interval_ms_like_cpp == 0 || self.next_player_save_ms_like_cpp == 0 {
            return;
        }

        if diff_ms >= self.next_player_save_ms_like_cpp {
            self.next_player_save_ms_like_cpp = 0;
            self.pending_periodic_player_save_like_cpp = true;
        } else {
            self.next_player_save_ms_like_cpp -= diff_ms;
        }
    }

    /// C++ `CollectionMgr::SaveAccountMounts`.
    pub fn account_mount_save_rows_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Vec<AccountMountSaveRowLikeCpp>> {
        let bnet_account_id = hub.core.battlenet_account_id();
        let mut rows = hub
            .player_collection_state_snapshot_like_cpp()?
            .mounts_like_cpp()
            .into_iter()
            .filter_map(|(spell_id, flags)| {
                Some(AccountMountSaveRowLikeCpp {
                    bnet_account_id,
                    mount_spell_id: u32::try_from(*spell_id).ok()?,
                    flags: *flags,
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.mount_spell_id);
        Some(rows)
    }

    pub fn represented_save_cuf_profiles_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        profiles: Vec<CufProfile>,
    ) -> bool {
        if profiles.len() > MAX_CUF_PROFILES_LIKE_CPP {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        let fixture_profiles = profiles.clone();
        let profiles = profiles
            .into_iter()
            .map(player_cuf_profile_from_packet_like_cpp)
            .collect::<Vec<_>>();
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            // C++ `WorldSession::HandleSaveCUFProfiles` saves the sent slots
            // and then empties the rest (MiscHandler.cpp:1115-1119).
            let sent = profiles.len();
            for (slot, profile) in profiles.into_iter().enumerate() {
                player.save_cuf_profile_like_cpp(slot, Some(profile));
            }
            for slot in sent..MAX_CUF_PROFILES_LIKE_CPP {
                player.save_cuf_profile_like_cpp(slot, None);
            }
        });
        if canonical.is_some() {
            return true;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures.presentation.cuf_profiles_like_cpp =
                vec![None; MAX_CUF_PROFILES_LIKE_CPP];
            for (slot, profile) in fixture_profiles.into_iter().enumerate() {
                hub.fixtures.presentation.cuf_profiles_like_cpp[slot] = Some(profile);
            }
            return true;
        }
        false
    }
}
