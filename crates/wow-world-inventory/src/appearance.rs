// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeMap, HashSet};

use tracing::warn;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedTransmogCriteriaEvent;
use crate::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
    DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
};
use wow_core::ObjectGuid;
use wow_entities::{
    EQUIPMENT_SLOT_END, PlayerEquipmentSetLikeCpp as RepresentedEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
    PlayerFavoriteAppearanceStateLikeCpp as FavoriteAppearanceStateLikeCpp,
};
use wow_world_core::session::OwnedCollectionsAccessLikeCpp;
use wow_world_core::session::{HubMut, HubRef, RepresentedAlterAppearanceLikeCpp};

fn account_transmog_update_opcode_resolved_like_cpp() -> bool {
    <wow_packet::packets::collection::AccountTransmogUpdate as wow_packet::ServerPacket>::OPCODE
        != wow_constants::opcodes::ServerOpcodes::UpdateCapturePoint
}

impl crate::InventoryState {
    pub fn load_represented_transmog_outfit_row_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        appearances: [i32; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
        enchants: [i32; 2],
    ) -> bool {
        if set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return false;
        }

        let equipment_set = RepresentedEquipmentSetLikeCpp {
            raw_set_type: RepresentedEquipmentSetTypeLikeCpp::Transmog.as_i32_like_cpp(),
            set_type: RepresentedEquipmentSetTypeLikeCpp::Transmog,
            guid,
            set_id,
            ignore_mask,
            pieces: [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances,
            enchants,
            secondary_shoulder_appearance_id: 0,
            secondary_shoulder_slot: 0,
            secondary_weapon_appearance_id: 0,
            secondary_weapon_slot: 0,
            assigned_spec_index: -1,
            set_name,
            set_icon,
            state: RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        };
        self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| {
            sets.install_loaded_set_like_cpp(equipment_set.clone());
        })
        .is_some()
    }

    /// Bounded C++ `CollectionMgr::AddItemAppearance`.
    pub fn add_item_appearance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_modified_appearance_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let block_index = usize::try_from(item_modified_appearance_id / 32).ok()?;
        let bit_index = item_modified_appearance_id % 32;
        let flag = 1_u32.checked_shl(bit_index)?;
        let had_temporary = hub
            .shared()
            .player_collection_state_snapshot_like_cpp()?
            .has_temporary_item_appearance_like_cpp(item_modified_appearance_id);

        let result = hub.core.mutate_canonical_player_like_cpp(|player| {
            while player.transmog_blocks_like_cpp().len() <= block_index {
                player.add_transmog_block_like_cpp(0);
            }

            let added_flag = player.add_transmog_flag_like_cpp(block_index, flag);
            if had_temporary {
                player.remove_conditional_transmog_like_cpp(item_modified_appearance_id);
            }

            added_flag.then(|| player.values_update(true))
        })??;

        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_item_appearance_like_cpp(item_modified_appearance_id);
        })?;
        self.update_represented_transmog_criteria_like_cpp(hub, item_modified_appearance_id);
        Some(result)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_has_item_appearance_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_modified_appearance_id: u32,
    ) -> bool {
        hub.fixtures
            .collections
            .represented_item_appearances_like_cpp
            .contains(&item_modified_appearance_id)
    }

    /// C++ `CollectionMgr::HasItemAppearance`.
    pub fn has_item_appearance_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_modified_appearance_id: u32,
    ) -> (bool, bool) {
        #[cfg(any(test, feature = "test-fixtures"))]
        let access = hub
            .core
            .owned_collections_access_like_cpp()
            .with_fixture_collections(&hub.fixtures.collections);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let access = hub.core.owned_collections_access_like_cpp();
        self.has_item_appearance_with_collections_access_like_cpp(
            &access,
            item_modified_appearance_id,
        )
    }

    pub(crate) fn has_item_appearance_with_collections_access_like_cpp(
        &self,
        access: &OwnedCollectionsAccessLikeCpp<'_>,
        item_modified_appearance_id: u32,
    ) -> (bool, bool) {
        let Some(collections) = access.player_collection_state_snapshot_like_cpp() else {
            return (false, false);
        };
        if collections
            .item_appearances_like_cpp()
            .contains(&item_modified_appearance_id)
        {
            return (true, false);
        }

        if collections.has_temporary_item_appearance_like_cpp(item_modified_appearance_id) {
            return (true, true);
        }

        (false, false)
    }

    /// C++ `CollectionMgr::LoadAccountItemAppearances`.
    pub fn load_represented_account_item_appearances_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        known_appearance_blocks: impl IntoIterator<Item = (u32, u32)>,
        favorite_appearances: impl IntoIterator<Item = u32>,
    ) {
        let mut blocks = BTreeMap::new();
        for (block_index, appearance_mask) in known_appearance_blocks {
            if appearance_mask != 0 {
                blocks.insert(block_index, appearance_mask);
            }
        }

        let mut item_appearances = HashSet::new();
        for (&block_index, &appearance_mask) in &blocks {
            for bit_index in 0..32 {
                if (appearance_mask & (1_u32 << bit_index)) != 0 {
                    item_appearances.insert(block_index * 32 + bit_index);
                }
            }
        }

        let mut item_appearance_blocks = Vec::new();
        if let Some((&highest_block, _)) = blocks.iter().next_back() {
            item_appearance_blocks = vec![0; highest_block as usize + 1];
            for (&block_index, &appearance_mask) in &blocks {
                item_appearance_blocks[block_index as usize] = appearance_mask;
            }

            hub.core.mutate_canonical_player_like_cpp(|player| {
                while player.transmog_blocks_like_cpp().len() <= highest_block as usize {
                    player.add_transmog_block_like_cpp(0);
                }

                for (&block_index, &appearance_mask) in &blocks {
                    if appearance_mask != 0 {
                        player.add_transmog_flag_like_cpp(block_index as usize, appearance_mask);
                    }
                }
            });
        }

        let favorite_item_appearances = favorite_appearances
            .into_iter()
            .map(|appearance| (appearance, FavoriteAppearanceStateLikeCpp::Unchanged))
            .collect();
        let _ = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.install_appearance_collection_like_cpp(
                item_appearances,
                item_appearance_blocks,
                favorite_item_appearances,
            );
        });
    }

    /// C++ `CollectionMgr::SaveAccountItemAppearances`.
    pub fn account_item_appearance_save_plan_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> Option<AccountItemAppearanceSavePlanLikeCpp> {
        let mut collections = hub.shared().player_collection_state_snapshot_like_cpp()?;
        let mut blocks = BTreeMap::<u32, u32>::new();
        for &item_modified_appearance_id in collections.item_appearances_like_cpp() {
            let block_index = item_modified_appearance_id / 32;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                *blocks.entry(block_index).or_default() |= flag;
            }
        }

        let (favorite_inserts, favorite_deletes) =
            collections.settle_favorite_item_appearance_saves_like_cpp();

        let plan = AccountItemAppearanceSavePlanLikeCpp {
            appearance_blocks: blocks
                .into_iter()
                .filter(|(_, appearance_mask)| *appearance_mask != 0)
                .collect(),
            favorite_inserts,
            favorite_deletes,
        };
        let _ = hub.replace_player_collection_state_like_cpp(collections);
        Some(plan)
    }

    /// C++ `CollectionMgr::SetAppearanceIsFavorite`.
    pub fn set_appearance_is_favorite_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        use FavoriteAppearanceStateLikeCpp::{New, Removed, Unchanged};
        use std::collections::hash_map::Entry;

        let changed = hub
            .mutate_player_collection_state_like_cpp(|collections| {
                if apply {
                    match collections
                        .favorite_item_appearance_entry_like_cpp(item_modified_appearance_id)
                    {
                        Entry::Vacant(entry) => {
                            entry.insert(New);
                            true
                        }
                        Entry::Occupied(mut entry) if *entry.get() == Removed => {
                            entry.insert(Unchanged);
                            true
                        }
                        Entry::Occupied(_) => false,
                    }
                } else {
                    match collections
                        .favorite_item_appearance_entry_like_cpp(item_modified_appearance_id)
                    {
                        Entry::Occupied(entry) if *entry.get() == New => {
                            entry.remove();
                            true
                        }
                        Entry::Occupied(mut entry) => {
                            entry.insert(Removed);
                            true
                        }
                        Entry::Vacant(_) => false,
                    }
                }
            })
            .unwrap_or(false);

        if changed {
            if !account_transmog_update_opcode_resolved_like_cpp() {
                warn!(
                    "Skipping AccountTransmogUpdate favorite delta: legacy C++ opcode is unresolved 0xBADD for 54261"
                );
                return changed;
            }

            hub.core.send_packet(
                &wow_packet::packets::collection::AccountTransmogUpdate::favorite_delta(
                    item_modified_appearance_id,
                    apply,
                ),
            );
        }

        changed
    }

    /// C++ `CollectionMgr::SendFavoriteAppearances`.
    pub fn send_favorite_appearances_like_cpp(&self, hub: HubRef<'_>) {
        if !account_transmog_update_opcode_resolved_like_cpp() {
            warn!(
                "Skipping AccountTransmogUpdate full update: legacy C++ opcode is unresolved 0xBADD for 54261"
            );
            return;
        }

        let Some(collections) = hub.player_collection_state_snapshot_like_cpp() else {
            return;
        };
        let favorite_appearances = collections
            .favorite_item_appearances_like_cpp()
            .into_iter()
            .filter_map(|(appearance, state)| {
                (*state != FavoriteAppearanceStateLikeCpp::Removed).then_some(*appearance)
            })
            .collect::<Vec<_>>();

        hub.core
            .send_packet(&wow_packet::packets::collection::AccountTransmogUpdate {
                is_full_update: true,
                is_set_favorite: false,
                favorite_appearances,
                new_appearances: Vec::new(),
            });
    }

    /// C++ `CollectionMgr::LoadAccountTransmogIllusions`.
    pub fn load_represented_account_transmog_illusions_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        known_illusion_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let mut illusions = HashSet::new();
        for (block_index, illusion_mask) in known_illusion_blocks {
            for bit_index in 0..32 {
                if (illusion_mask & (1_u32 << bit_index)) != 0 {
                    illusions.insert(block_index * 32 + bit_index);
                }
            }
        }

        for illusion_id in DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP {
            illusions.insert(illusion_id);
        }
        let _ = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.replace_transmog_illusions_like_cpp(illusions);
        });
    }

    /// C++ `CollectionMgr::HasTransmogIllusion`.
    #[allow(dead_code)]
    pub fn has_transmog_illusion_like_cpp(
        &self,
        hub: HubRef<'_>,
        transmog_illusion_id: u32,
    ) -> bool {
        hub.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| {
                collections
                    .transmog_illusions_like_cpp()
                    .contains(&transmog_illusion_id)
            })
    }

    /// C++ `CollectionMgr::SaveAccountTransmogIllusions`.
    pub fn account_transmog_illusion_save_plan_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<AccountTransmogIllusionSavePlanLikeCpp> {
        let mut blocks = BTreeMap::<u32, u32>::new();
        let collections = hub.player_collection_state_snapshot_like_cpp()?;
        for &illusion_id in collections.transmog_illusions_like_cpp() {
            let block_index = illusion_id / 32;
            let bit_index = illusion_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                *blocks.entry(block_index).or_default() |= flag;
            }
        }

        Some(AccountTransmogIllusionSavePlanLikeCpp {
            illusion_blocks: blocks
                .into_iter()
                .filter(|(_, illusion_mask)| *illusion_mask != 0)
                .collect(),
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_favorite_item_appearance_state_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_modified_appearance_id: u32,
    ) -> Option<FavoriteAppearanceStateLikeCpp> {
        hub.fixtures
            .collections
            .represented_favorite_item_appearances_like_cpp
            .get(&item_modified_appearance_id)
            .copied()
    }

    /// C++ `CollectionMgr::AddTemporaryAppearance`.
    pub fn add_temporary_item_appearance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let was_empty = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .add_temporary_item_appearance_like_cpp(item_modified_appearance_id, item_guid)
        })?;

        was_empty.then_some(())?;
        hub.core.mutate_canonical_player_like_cpp(|player| {
            player.add_conditional_transmog_like_cpp(item_modified_appearance_id);
            player.values_update(true)
        })
    }

    /// C++ `CollectionMgr::RemoveTemporaryAppearance`.
    pub fn remove_temporary_item_appearance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let removed_last = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .remove_temporary_item_appearance_like_cpp(item_modified_appearance_id, item_guid)
                .then_some(())
        })??;
        let _ = removed_last;
        hub.core.mutate_canonical_player_like_cpp(|player| {
            player.remove_conditional_transmog_like_cpp(item_modified_appearance_id);
            player.values_update(true)
        })
    }

    /// C++ `CollectionMgr::GetItemsProvidingTemporaryAppearance`.
    pub fn items_providing_temporary_appearance_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        hub.player_collection_state_snapshot_like_cpp()
            .and_then(|collections| {
                collections
                    .temporary_item_appearances_like_cpp()
                    .get(&item_modified_appearance_id)
                    .cloned()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::AddItemAppearance` criteria side effects.
    fn update_represented_transmog_criteria_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_modified_appearance_id: u32,
    ) {
        let item_id = hub
            .catalogs
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(item_modified_appearance_id))
            .and_then(|appearance| u32::try_from(appearance.item_id).ok());

        if let Some(_transmog_slot) = item_id
            .and_then(|item_id| {
                hub.catalogs
                    .items
                    .store
                    .as_ref()
                    .and_then(|store| store.inventory_type(item_id))
            })
            .and_then(wow_entities::item_transmogrification_slot_like_cpp)
        {
            #[cfg(any(test, feature = "test-fixtures"))]
            self.represented_transmog_criteria_events.push(
                RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                    equipment_slot: _transmog_slot as u32,
                    item_modified_appearance_id,
                },
            );
        }

        let transmog_sets = hub
            .catalogs
            .transmog_sets_for_item_modified_appearance_like_cpp(item_modified_appearance_id)
            .map(|sets| {
                sets.iter()
                    .map(|set| (set.id, set.transmog_set_group_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for (transmog_set_id, _transmog_set_group_id) in transmog_sets {
            if self.is_transmog_set_completed_like_cpp(hub.shared(), transmog_set_id) {
                #[cfg(any(test, feature = "test-fixtures"))]
                self.represented_transmog_criteria_events.push(
                    RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                        transmog_set_group_id: _transmog_set_group_id,
                    },
                );
            }
        }
    }

    /// C++ `CollectionMgr::IsSetCompleted`.
    pub fn is_transmog_set_completed_like_cpp(
        &self,
        hub: HubRef<'_>,
        transmog_set_id: u32,
    ) -> bool {
        let Some(transmog_set_items) = hub.catalogs.transmog_set_items_like_cpp(transmog_set_id)
        else {
            return false;
        };

        let mut known_pieces = [-1_i8; EQUIPMENT_SLOT_END as usize];
        for transmog_set_item in transmog_set_items {
            let Some(item_modified_appearance) = hub
                .catalogs
                .items
                .modified_appearance_store
                .as_ref()
                .and_then(|store| store.get(transmog_set_item.item_modified_appearance_id))
            else {
                continue;
            };
            let Some(item_id) = u32::try_from(item_modified_appearance.item_id).ok() else {
                continue;
            };
            let Some(inventory_type) = hub
                .catalogs
                .items
                .store
                .as_ref()
                .and_then(|store| store.inventory_type(item_id))
            else {
                continue;
            };
            let Some(transmog_slot) =
                wow_entities::item_transmogrification_slot_like_cpp(inventory_type)
            else {
                continue;
            };
            if known_pieces[transmog_slot] == 1 {
                continue;
            }

            let (has_appearance, is_temporary) = self
                .has_item_appearance_like_cpp(hub, transmog_set_item.item_modified_appearance_id);
            known_pieces[transmog_slot] = if has_appearance && !is_temporary {
                1
            } else {
                0
            };
        }

        !known_pieces.contains(&0)
    }

    /// Bounded C++ `CollectionMgr::AddTransmogSet`.
    pub fn add_transmog_set_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let appearance_ids = hub
            .catalogs
            .transmog_set_item_modified_appearances_like_cpp(transmog_set_id)
            .into_iter()
            .map(|appearance| appearance.id)
            .collect::<Vec<_>>();

        let mut last_update = None;
        for appearance_id in appearance_ids {
            if let Some(update) = self.add_item_appearance_like_cpp(hub, appearance_id) {
                last_update = Some(update);
            }
        }

        last_update
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_alter_appearance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        request: RepresentedAlterAppearanceLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            hub.fixtures
                .presentation
                .represented_alter_appearance_requests_like_cpp
                .push(request);
        }
    }
}
