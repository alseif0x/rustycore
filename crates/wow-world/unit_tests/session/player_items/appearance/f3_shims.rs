// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub fn transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[wow_data::TransmogSetItemEntry]> {
        self.catalogs.transmog_set_items_like_cpp(transmog_set_id)
    }
    pub fn transmog_set_item_modified_appearances_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Vec<&wow_data::ItemModifiedAppearanceEntry> {
        self.catalogs
            .transmog_set_item_modified_appearances_like_cpp(transmog_set_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_has_item_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_has_item_appearance_like_cpp(hub, item_modified_appearance_id)
    }
    #[allow(dead_code)]
    pub(crate) fn has_transmog_illusion_like_cpp(&self, transmog_illusion_id: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_transmog_illusion_like_cpp(hub, transmog_illusion_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_favorite_item_appearance_state_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<FavoriteAppearanceStateLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_favorite_item_appearance_state_like_cpp(hub, item_modified_appearance_id)
    }
    pub fn is_transmog_set_completed_like_cpp(&self, transmog_set_id: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.is_transmog_set_completed_like_cpp(hub, transmog_set_id)
    }
    pub(crate) fn account_item_appearance_save_plan_like_cpp(
        &mut self,
    ) -> Option<AccountItemAppearanceSavePlanLikeCpp> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.account_item_appearance_save_plan_like_cpp(&mut hub)
    }
    pub(crate) fn account_transmog_illusion_save_plan_like_cpp(
        &self,
    ) -> Option<AccountTransmogIllusionSavePlanLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.account_transmog_illusion_save_plan_like_cpp(hub)
    }
    pub(crate) fn account_transmog_active_player_rows_like_cpp(&self) -> Vec<u32> {
        crate::session::hub_ref(self).account_transmog_active_player_rows_like_cpp()
    }
}

impl WorldSession {
    /// Bounded represented C++ `Player::_LoadQuestStatusRewarded` item-appearance replay.
    ///
    /// This covers direct reward items plus the `GetQuestPackageItems` branch that uses
    /// `ItemTemplate::ItemSpecClassMask & Player::GetClassMask`. The mask is currently represented
    /// only through `ItemSpecOverride.db2`, matching C++'s primary `ObjectMgr::LoadItemTemplates`
    /// branch. The C++ fallback that derives `ItemSpecClassMask` from `ItemSpecStats` is intentionally
    /// not guessed here because Rust does not yet represent the sparse stat-modifier bonus fields.
    pub(crate) fn replay_rewarded_quest_item_appearances_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let mut last_update = self.replay_rewarded_quest_direct_item_appearances_like_cpp(quest);
        let player_class_mask = player_class_mask_for_transmog_like_cpp(
            crate::session::hub_ref(self).player_class_like_cpp(),
        );
        let package_item_ids = self
            .catalogs
            .quests
            .package_item_store
            .as_ref()
            .map(|store| {
                store
                    .quest_package_items_like_cpp(quest.quest_package_id)
                    .filter_map(|item| u32::try_from(item.item_id).ok())
                    .filter(|item_id| *item_id != 0)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for item_id in package_item_ids {
            let Some(item_spec_class_mask) = self
                .catalogs
                .item_spec_class_mask_from_overrides_like_cpp(item_id)
            else {
                continue;
            };
            if (item_spec_class_mask & player_class_mask) == 0 {
                continue;
            }

            if let Some(update) = self.add_item_appearance_for_item_like_cpp(item_id, 0) {
                last_update = Some(update);
            }
        }

        last_update
    }
    /// Bounded direct-item part of C++ `Player::_LoadQuestStatusRewarded`.
    ///
    /// C++ replays reward choice item ids and fixed reward item ids through
    /// `CollectionMgr::AddItemAppearance(itemId)`. Use
    /// `replay_rewarded_quest_item_appearances_like_cpp` for the combined direct + represented
    /// quest-package replay.
    pub(crate) fn replay_rewarded_quest_direct_item_appearances_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let reward_item_ids = quest
            .reward_choice_items
            .iter()
            .map(|(item_id, _quantity)| *item_id)
            .chain(quest.reward_items.iter().copied())
            .filter(|item_id| *item_id != 0)
            .collect::<Vec<_>>();

        let mut last_update = None;
        for item_id in reward_item_ids {
            if let Some(update) = self.add_item_appearance_for_item_like_cpp(item_id, 0) {
                last_update = Some(update);
            }
        }

        last_update
    }
    pub(crate) fn set_appearance_is_favorite_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_appearance_is_favorite_like_cpp(&mut hub, item_modified_appearance_id, apply)
    }
    pub(crate) fn remove_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_temporary_item_appearance_like_cpp(
            &mut hub,
            item_modified_appearance_id,
            item_guid,
        )
    }
    pub(crate) fn items_providing_temporary_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.items_providing_temporary_appearance_like_cpp(hub, item_modified_appearance_id)
    }
}
