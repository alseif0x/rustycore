//! persistence for the existing appearance owner.

use super::*;

impl WorldSession {
    pub(crate) fn load_represented_transmog_outfit_row_like_cpp(
        &mut self,
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
        self.with_owned_equipment_sets_mut_like_cpp(|sets| {
            sets.install_loaded_set_like_cpp(equipment_set.clone());
        })
        .is_some()
    }
    /// C++ `CollectionMgr::LoadAccountItemAppearances` active-player create data order.
    pub(crate) fn account_transmog_active_player_rows_like_cpp(&self) -> Vec<u32> {
        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return Vec::new();
        };
        collections.active_appearance_blocks(|| {
            self.canonical_player_snapshot_like_cpp(|player| {
                player.transmog_blocks_like_cpp().to_vec()
            })
        })
    }

    /// C++ `CollectionMgr::LoadAccountItemAppearances`.
    pub(crate) fn load_represented_account_item_appearances_like_cpp(
        &mut self,
        known_appearance_blocks: impl IntoIterator<Item = (u32, u32)>,
        favorite_appearances: impl IntoIterator<Item = u32>,
    ) {
        let (blocks, item_appearances, item_appearance_blocks, highest_block) =
            wow_entities::PlayerCollectionStateLikeCpp::prepare_appearance_blocks(
                known_appearance_blocks,
            );
        if let Some(highest_block) = highest_block {
            self.mutate_canonical_player_like_cpp(|player| {
                wow_entities::PlayerCollectionStateLikeCpp::apply_loaded_appearance_fields(
                    player,
                    highest_block,
                    &blocks,
                );
            });
        }
        let favorite_item_appearances =
            wow_entities::PlayerCollectionStateLikeCpp::loaded_appearance_favorites(
                favorite_appearances,
            );
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.install_appearance_collection_like_cpp(
                item_appearances,
                item_appearance_blocks,
                favorite_item_appearances,
            );
        });
    }

    /// C++ `CollectionMgr::SaveAccountItemAppearances`.
    pub(crate) fn account_item_appearance_save_plan_like_cpp(
        &mut self,
    ) -> Option<AccountItemAppearanceSavePlanLikeCpp> {
        let mut collections = self.player_collection_state_snapshot_like_cpp()?;
        let plan = collections.appearance_save_plan();
        let _ = self.replace_player_collection_state_like_cpp(collections);
        Some(plan)
    }

    /// C++ `CollectionMgr::LoadAccountTransmogIllusions`.
    pub(crate) fn load_represented_account_transmog_illusions_like_cpp(
        &mut self,
        known_illusion_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let illusions = wow_entities::PlayerCollectionStateLikeCpp::loaded_illusion_ids(
            known_illusion_blocks,
        );
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.replace_transmog_illusions_like_cpp(illusions);
        });
    }

    /// C++ `CollectionMgr::HasTransmogIllusion`.
    #[allow(dead_code)]
    pub(crate) fn has_transmog_illusion_like_cpp(&self, transmog_illusion_id: u32) -> bool {
        self.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| collections.has_transmog_illusion(transmog_illusion_id))
    }

    /// C++ `CollectionMgr::SaveAccountTransmogIllusions`.
    pub(crate) fn account_transmog_illusion_save_plan_like_cpp(
        &self,
    ) -> Option<AccountTransmogIllusionSavePlanLikeCpp> {
        let collections = self.player_collection_state_snapshot_like_cpp()?;
        Some(collections.illusion_save_plan())
    }
}
