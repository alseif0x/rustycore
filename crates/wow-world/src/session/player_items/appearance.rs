//! Represented item appearance: enchantment, transmogrification and illusion state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;



impl WorldSession {
    /// Set the item appearance store for this session.
    pub fn set_item_appearance_store(&mut self, store: Arc<ItemAppearanceStore>) {
        self.catalogs.items.appearance_store = Some(store);
    }
    /// Get the item appearance store reference.
    pub fn item_appearance_store(&self) -> Option<&Arc<ItemAppearanceStore>> {
        self.catalogs.items.appearance_store.as_ref()
    }
    /// Set the item modified appearance store for this session.
    pub fn set_item_modified_appearance_store(&mut self, store: Arc<ItemModifiedAppearanceStore>) {
        self.catalogs.items.modified_appearance_store = Some(store);
    }
    pub fn item_modified_appearance_store(&self) -> Option<&Arc<ItemModifiedAppearanceStore>> {
        self.catalogs.item_modified_appearance_store()
    }
    /// Set the transmog set item store for this session.
    pub fn set_transmog_set_item_store(&mut self, store: Arc<TransmogSetItemStore>) {
        self.catalogs.transmog_set_item_store = Some(store);
    }
    /// Get the transmog set item store reference.
    pub fn transmog_set_item_store(&self) -> Option<&Arc<TransmogSetItemStore>> {
        self.catalogs.transmog_set_item_store.as_ref()
    }
    pub fn add_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_item_appearance_like_cpp(&mut hub, item_modified_appearance_id)
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance(uint32, uint32)`.
    ///
    /// This resolves `ItemModifiedAppearance` by `(item_id, appearance_mod_id)` like
    /// `sDB2Manager.GetItemModifiedAppearance` and runs the represented
    /// `CanAddAppearance` item-template/proficiency/quality gate, including the
    /// learned `Player::GetWeaponProficiency` mask.
    pub fn add_item_appearance_for_item_like_cpp(
        &mut self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_modified_appearance_id =
            self.item_modified_appearance_for_item(item_id, appearance_mod_id)?;
        if !self.can_add_item_appearance_represented_like_cpp(item_modified_appearance_id) {
            return None;
        }
        self.add_item_appearance_like_cpp(item_modified_appearance_id)
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance(Item*)`.
    pub(crate) fn add_item_appearance_for_runtime_item_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        if !item.is_soul_bound() {
            return None;
        }

        let item_id = item.object().entry();
        let appearance_mod_id = item.visible_appearance_mod_id(0, |appearance_id| {
            self.item_modified_appearance_ref(appearance_id)
        });
        let item_modified_appearance_id =
            self.item_modified_appearance_for_item(item_id, u32::from(appearance_mod_id))?;
        if !self.can_add_item_appearance_represented_like_cpp(item_modified_appearance_id) {
            return None;
        }

        if item.is_bop_tradeable() || item.is_refundable() {
            return self.add_temporary_item_appearance_like_cpp(
                item_modified_appearance_id,
                item.object().guid(),
            );
        }

        self.add_item_appearance_like_cpp(item_modified_appearance_id)
    }
    /// Bounded C++ `CollectionMgr::CanAddAppearance`.
    ///
    /// This covers the DB2/template, represented `CanUseItem` class/proficiency,
    /// transmog source, quality/flags, item class/subclass/inventory and duplicate
    /// gates that Rust currently represents. Full `Player::CanUseItem` remains
    /// wider than this helper.
    pub fn can_add_item_appearance_represented_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        let Some(item_modified_appearance) = self
            .catalogs
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(item_modified_appearance_id))
        else {
            return false;
        };

        if matches!(item_modified_appearance.transmog_source_type_enum, 6 | 9) {
            return false;
        }

        let Ok(item_id) = u32::try_from(item_modified_appearance.item_id) else {
            return false;
        };
        let Some(search_template) = self
            .catalogs
            .items
            .search_name_store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };

        let Some(item_record) = self
            .catalogs
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };
        let Some(sparse_template) = self
            .catalogs
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_id))
        else {
            return false;
        };
        let Some(template) = self.item_storage_template(item_id) else {
            return false;
        };
        if self.player_guid().is_none() {
            return false;
        }

        // C++ `CollectionMgr::CanAddAppearance` first runs
        // `Player::CanUseItem(ItemTemplate const*)` (`Player.cpp:11069-11125`).
        // The represented template gates are the internal/faction flags, the
        // allowable class/race masks and the required skill, spell and level
        // checks; the holiday, reputation, learning-effect and artifact
        // specialization gates remain separate slices.
        let use_flags2 = sparse_template.flags[1];
        if (use_flags2 & ItemFlags2::InternalItem as u32) != 0 {
            return false;
        }
        let player_team =
            player_team_id_for_race_cpp(crate::session::hub_ref(self).player_race_like_cpp());
        if (use_flags2 & ItemFlags2::FactionHorde as u32) != 0
            && player_team != wow_entities::TEAM_HORDE_ID
        {
            return false;
        }
        if (use_flags2 & ItemFlags2::FactionAlliance as u32) != 0
            && player_team != wow_entities::TEAM_ALLIANCE_ID
        {
            return false;
        }
        let player_race_mask = crate::session::hub_ref(self)
            .player_race_like_cpp()
            .checked_sub(1)
            .and_then(|shift| 1i64.checked_shl(u32::from(shift)))
            .unwrap_or(0);
        if search_template.allowable_race != 0
            && (search_template.allowable_race & player_race_mask) == 0
        {
            return false;
        }
        if search_template.required_level > 0
            && crate::session::hub_ref(self).player_level_like_cpp()
                < u8::try_from(search_template.required_level).unwrap_or(u8::MAX)
        {
            return false;
        }
        if search_template.required_skill != 0 {
            let Some(skill_value) =
                u16::try_from(search_template.required_skill)
                    .ok()
                    .and_then(|skill| {
                        crate::session::hub_ref(self).resolved_player_skill_value_like_cpp(skill)
                    })
            else {
                return false;
            };
            if u32::from(skill_value) < u32::from(search_template.required_skill_rank) {
                return false;
            }
        }
        if search_template.required_ability != 0
            && !i32::try_from(search_template.required_ability)
                .ok()
                .is_some_and(|spell_id| self.known_spells_like_cpp().contains(&spell_id))
        {
            return false;
        }
        if sparse_template.required_reputation_faction != 0 {
            let required_rank =
                u32::try_from(sparse_template.required_reputation_rank.max(0)).unwrap_or(0);
            if self
                .represented_item_reputation_rank_like_cpp(u32::from(
                    sparse_template.required_reputation_faction,
                ))
                .unwrap_or(0)
                < required_rank
            {
                return false;
            }
        }
        // C++ `CanUseItem` learning-effect pair (`Player.cpp:11110-11113`): a
        // recipe, mount or pet item whose second effect is already known cannot
        // be used again.
        let effect_spell_ids = self
            .catalogs
            .represented_item_effect_spell_ids_like_cpp(item_id);
        if let (Some((_, first)), Some((_, second))) =
            (effect_spell_ids.first(), effect_spell_ids.get(1))
            && matches!(*first, 483 | 55_884)
            && i32::try_from(*second)
                .ok()
                .is_some_and(|spell_id| self.known_spells_like_cpp().contains(&spell_id))
        {
            return false;
        }

        let player_class_mask = player_class_mask_for_transmog_like_cpp(
            crate::session::hub_ref(self).player_class_like_cpp(),
        );
        if sparse_template.allowable_class != 0
            && (u32::try_from(sparse_template.allowable_class).unwrap_or(0) & player_class_mask)
                == 0
        {
            return false;
        }

        let flags2 = sparse_template.flags[1];
        let flags3 = sparse_template.flags[2];
        if (flags2 & ItemFlags2::NoSourceForItemVisual as u32) != 0 {
            return false;
        }
        let Some(quality) = self.item_template_quality(item_id) else {
            return false;
        };
        if quality == ItemQuality::Artifact as i8 {
            return false;
        }

        match template.class_id {
            ItemClass::Weapon => {
                let subclass = u32::from(item_record.subclass_id);
                if subclass >= 32 {
                    return false;
                }
                // C++ `CollectionMgr::CanAddAppearance` reads the learned
                // `Player::GetWeaponProficiency` mask, not the class default
                // the client receives at creation.
                let weapon_proficiency = self
                    .core
                    .represented_player_weapon_proficiency_like_cpp()
                    .unwrap_or(0);
                if (weapon_proficiency & (1_u32 << subclass)) == 0 {
                    return false;
                }
                if matches!(
                    subclass,
                    x if x == ItemSubClassWeapon::Exotic as u32
                        || x == ItemSubClassWeapon::Exotic2 as u32
                        || x == ItemSubClassWeapon::Miscellaneous as u32
                        || x == ItemSubClassWeapon::Thrown as u32
                        || x == ItemSubClassWeapon::Spear as u32
                        || x == ItemSubClassWeapon::FishingPole as u32
                ) {
                    return false;
                }
            }
            ItemClass::Armor => {
                let subclass = u32::from(item_record.subclass_id);
                match template.inventory_type {
                    InventoryType::Body
                    | InventoryType::Shield
                    | InventoryType::Cloak
                    | InventoryType::Tabard
                    | InventoryType::Holdable => {}
                    InventoryType::Head
                    | InventoryType::Shoulders
                    | InventoryType::Chest
                    | InventoryType::Waist
                    | InventoryType::Legs
                    | InventoryType::Feet
                    | InventoryType::Wrists
                    | InventoryType::Hands
                    | InventoryType::Robe => {
                        if subclass == ItemSubClassArmor::Miscellaneous as u32 {
                            return false;
                        }
                    }
                    _ => return false,
                }

                if template.inventory_type != InventoryType::Cloak
                    && (player_class_by_armor_subclass_like_cpp(subclass) & player_class_mask) == 0
                {
                    return false;
                }
            }
            _ => return false,
        }

        if quality < ItemQuality::Uncommon as i8
            && ((flags2 & ItemFlags2::IgnoreQualityForItemVisualSource as u32) == 0
                || (flags3 & ItemFlags3::ActsAsTransmogHiddenVisualOption as u32) == 0)
        {
            return false;
        }

        crate::session::hub_ref(self)
            .player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| {
                !collections
                    .item_appearances_like_cpp()
                    .contains(&item_modified_appearance_id)
            })
    }
    /// Bounded represented C++ `Player::_LoadQuestStatusRewarded` item-appearance replay.
    ///
    /// This covers direct reward items plus the `GetQuestPackageItems` branch that uses
    /// `ItemTemplate::ItemSpecClassMask & Player::GetClassMask`. The mask is currently represented
    /// only through `ItemSpecOverride.db2`, matching C++'s primary `ObjectMgr::LoadItemTemplates`
    /// branch. The C++ fallback that derives `ItemSpecClassMask` from `ItemSpecStats` is intentionally
    /// not guessed here because Rust does not yet represent the sparse stat-modifier bonus fields.
    pub fn replay_rewarded_quest_item_appearances_like_cpp(
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
    pub fn replay_rewarded_quest_direct_item_appearances_like_cpp(
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
    pub fn has_item_appearance_like_cpp(&self, item_modified_appearance_id: u32) -> (bool, bool) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_item_appearance_like_cpp(hub, item_modified_appearance_id)
    }
    pub(crate) fn load_represented_account_item_appearances_like_cpp(
        &mut self,
        known_appearance_blocks: impl IntoIterator<Item = (u32, u32)>,
        favorite_appearances: impl IntoIterator<Item = u32>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_account_item_appearances_like_cpp(
            &mut hub,
            known_appearance_blocks,
            favorite_appearances,
        )
    }
    pub fn set_appearance_is_favorite_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_appearance_is_favorite_like_cpp(&mut hub, item_modified_appearance_id, apply)
    }
    pub fn send_favorite_appearances_like_cpp(&self) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_favorite_appearances_like_cpp(hub)
    }
    pub(crate) fn load_represented_account_transmog_illusions_like_cpp(
        &mut self,
        known_illusion_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_account_transmog_illusions_like_cpp(&mut hub, known_illusion_blocks)
    }
    pub fn add_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_temporary_item_appearance_like_cpp(
            &mut hub,
            item_modified_appearance_id,
            item_guid,
        )
    }
    pub fn remove_temporary_item_appearance_like_cpp(
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
    pub fn items_providing_temporary_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.items_providing_temporary_appearance_like_cpp(hub, item_modified_appearance_id)
    }
    pub fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_transmog_set_like_cpp(&mut hub, transmog_set_id)
    }
    pub fn item_modified_appearance_ref(&self, id: u32) -> Option<(u32, u16)> {
        self.catalogs.item_modified_appearance_ref(id)
    }
    pub fn item_modified_appearance_for_item(
        &self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<u32> {
        self.catalogs
            .item_modified_appearance_for_item(item_id, appearance_mod_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_alter_appearance_requests_like_cpp(
        &self,
    ) -> &[RepresentedAlterAppearanceLikeCpp] {
        &self
            .fixtures
            .presentation
            .represented_alter_appearance_requests_like_cpp
    }
}



#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/appearance/f3_shims.rs"]
mod f3_shims;
