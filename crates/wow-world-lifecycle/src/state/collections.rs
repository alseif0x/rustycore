use tracing::warn;
use wow_constants::opcodes::ServerOpcodes;
use wow_data::HeirloomEntry;
use wow_packet::packets::misc::{
    AccountHeirloom, AccountHeirloomUpdate, AccountMount, AccountToy, AccountToyUpdate,
};
use wow_world_core::session::{
    HubMut, HubRef, TOY_FLAG_FAVORITE_LIKE_CPP, TOY_FLAG_HAS_FANFARE_LIKE_CPP,
};

use super::SessionLifecycleState;

fn account_heirloom_update_opcode_resolved_like_cpp() -> bool {
    <wow_packet::packets::misc::AccountHeirloomUpdate as wow_packet::ServerPacket>::OPCODE
        != ServerOpcodes::UpdateCapturePoint
}

pub(crate) fn heirloom_bonus_for_flags_like_cpp(heirloom: &HeirloomEntry, flags: u32) -> u32 {
    for upgrade_level in (0..heirloom.upgrade_item_id.len()).rev() {
        if flags & (1_u32 << upgrade_level) != 0 {
            return u32::from(heirloom.upgrade_item_bonus_list_id[upgrade_level]);
        }
    }

    0
}

impl SessionLifecycleState {
    /// C++ `CollectionMgr::SaveAccountHeirlooms`.
    pub fn account_heirloom_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<(u32, u32)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .map(|(item_id, data)| (*item_id, data.flags))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::GetHeirloomBonus`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn account_heirloom_bonus_like_cpp(&self, hub: HubRef<'_>, item_id: u32) -> u32 {
        hub.fixtures
            .collections
            .represented_account_heirlooms_like_cpp
            .get(&item_id)
            .map(|data| data.bonus_id)
            .unwrap_or(0)
    }

    /// C++ `CollectionMgr::GetAccountHeirlooms` full update payload.
    pub fn account_heirloom_packet_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<AccountHeirloom> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .filter_map(|(item_id, data)| {
                        Some(AccountHeirloom {
                            item_id: i32::try_from(*item_id).ok()?,
                            flags: data.flags,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::LoadHeirlooms` active-player create data order.
    pub fn account_heirloom_active_player_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<(i32, u32)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .filter_map(|(item_id, data)| Some((i32::try_from(*item_id).ok()?, data.flags)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `WorldPackets::Misc::AccountHeirloomUpdate` full login update.
    pub fn send_account_heirlooms_like_cpp(&self, hub: HubRef<'_>) {
        if !account_heirloom_update_opcode_resolved_like_cpp() {
            warn!(
                "Skipping AccountHeirloomUpdate: legacy C++ opcode is unresolved 0xBADD for 54261"
            );
            return;
        }

        hub.core.send_packet(&AccountHeirloomUpdate::full(
            self.account_heirloom_packet_rows_like_cpp(hub),
        ));
    }

    /// C++ `CollectionMgr::AddHeirloom` / `UpdateAccountHeirlooms`.
    pub fn add_account_heirloom_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_id: u32,
        flags: u32,
    ) -> bool {
        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_heirloom_like_cpp(
                item_id,
                wow_entities::PlayerAccountHeirloomDataLikeCpp { flags, bonus_id: 0 },
            )
        })
        .unwrap_or(false)
    }

    /// C++ `CollectionMgr::UpgradeHeirloom`.
    pub fn upgrade_account_heirloom_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let heirloom = hub
            .catalogs
            .heirloom_store
            .as_ref()?
            .get_by_item_id_like_cpp(item_id)?
            .clone();
        let current_flags = hub
            .shared()
            .player_collection_state_snapshot_like_cpp()?
            .heirlooms_like_cpp()
            .get(&item_id)?
            .flags;
        let active_item_id = i32::try_from(item_id).ok()?;
        let active_offset = hub.core.mutate_canonical_player_like_cpp(|player| {
            player
                .heirlooms_like_cpp()
                .iter()
                .position(|&heirloom_item_id| heirloom_item_id == active_item_id)
        })??;

        let mut flags = current_flags;
        let mut bonus_id = 0_u32;
        for (upgrade_level, &upgrade_item_id) in heirloom.upgrade_item_id.iter().enumerate() {
            if upgrade_item_id == cast_item {
                flags |= 1_u32 << upgrade_level;
                bonus_id = u32::from(heirloom.upgrade_item_bonus_list_id[upgrade_level]);
            }
        }

        let update = hub.core.mutate_canonical_player_like_cpp(|player| {
            player
                .set_heirloom_flags_like_cpp(active_offset, flags)
                .then(|| player.values_update(true))
        })??;

        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .update_heirloom_like_cpp(item_id, flags, bonus_id)
                .then_some(())
        })??;
        Some(update)
    }

    /// C++ `CollectionMgr::SaveAccountToys`.
    pub fn account_toy_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<(u32, bool, bool)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .iter()
                    .map(|(item_id, flags)| {
                        (
                            *item_id,
                            (*flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                            (*flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::GetAccountToys` full update payload.
    pub fn account_toy_packet_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<AccountToy> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .iter()
                    .map(|(item_id, flags)| AccountToy {
                        item_id: *item_id,
                        is_favorite: (flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                        has_fanfare: (flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::LoadToys` active-player create data order.
    pub fn account_toy_active_player_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<i32> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .keys()
                    .filter_map(|item_id| i32::try_from(*item_id).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `WorldPackets::Toy::AccountToyUpdate` full login update.
    pub fn send_account_toys_like_cpp(&self, hub: HubRef<'_>) {
        hub.core.send_packet(&AccountToyUpdate::full(
            self.account_toy_packet_rows_like_cpp(hub),
        ));
    }

    /// C++ `CollectionMgr::HasToy`.
    pub fn has_account_toy_like_cpp(&self, hub: HubRef<'_>, item_id: u32) -> bool {
        hub.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| collections.toys_like_cpp().contains_key(&item_id))
    }

    /// C++ `CollectionMgr::AddToy` / `UpdateAccountToys`.
    pub fn add_account_toy_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_id: u32,
        is_favorite: bool,
        has_fanfare: bool,
    ) -> bool {
        let mut flags = 0_u32;
        if is_favorite {
            flags |= TOY_FLAG_FAVORITE_LIKE_CPP;
        }
        if has_fanfare {
            flags |= TOY_FLAG_HAS_FANFARE_LIKE_CPP;
        }
        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_toy_like_cpp(item_id, flags)
        })
        .unwrap_or(false)
    }

    pub fn add_account_mount_with_faction_counterpart_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        spell_id: i32,
        flags: u8,
    ) -> bool {
        self.add_account_mount_like_cpp(hub, spell_id, flags, true)
    }

    fn add_account_mount_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        spell_id: i32,
        flags: u8,
        include_faction_counterpart: bool,
    ) -> bool {
        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return false;
        };
        if hub.catalogs.mount_store.as_ref().is_some_and(|store| {
            store
                .get_by_source_spell_id_like_cpp(spell_id_u32)
                .is_none()
        }) {
            return false;
        }

        if include_faction_counterpart
            && let Some(other_faction_spell_id) = hub
                .catalogs
                .mount_definition_store_like_cpp
                .as_ref()
                .and_then(|store| store.other_faction_spell_id_like_cpp(spell_id_u32))
            && let Ok(other_faction_spell_id) = i32::try_from(other_faction_spell_id)
        {
            self.add_account_mount_like_cpp(hub, other_faction_spell_id, flags, false);
        }

        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_mount_like_cpp(spell_id, flags)
        })
        .unwrap_or(false)
    }

    pub fn expand_account_mount_faction_definitions_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(mounts) = hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .mounts_snapshot_like_cpp()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
        else {
            return;
        };
        for (spell_id, flags) in mounts {
            self.add_account_mount_with_faction_counterpart_like_cpp(hub, spell_id, flags);
        }
    }

    pub fn account_mount_rows_like_cpp(&self, hub: HubRef<'_>) -> Vec<AccountMount> {
        let Some(mut mounts) = hub
            .player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .mounts_like_cpp()
                    .iter()
                    .map(|(spell_id, flags)| AccountMount {
                        spell_id: *spell_id,
                        flags: *flags,
                    })
                    .collect::<Vec<_>>()
            })
        else {
            return Vec::new();
        };
        mounts.sort_by_key(|mount| mount.spell_id);
        mounts
    }
}
