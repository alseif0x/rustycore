//! Represented item appearance: enchantment, transmogrification and illusion state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

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
        self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            sets.insert(guid, equipment_set.clone());
        })
        .is_some()
    }
    /// Set the item appearance store for this session.
    pub fn set_item_appearance_store(&mut self, store: Arc<ItemAppearanceStore>) {
        self.items.appearance_store = Some(store);
    }
    /// Get the item appearance store reference.
    pub fn item_appearance_store(&self) -> Option<&Arc<ItemAppearanceStore>> {
        self.items.appearance_store.as_ref()
    }
    /// Set the item modified appearance store for this session.
    pub fn set_item_modified_appearance_store(&mut self, store: Arc<ItemModifiedAppearanceStore>) {
        self.items.modified_appearance_store = Some(store);
    }
    /// Get the item modified appearance store reference.
    pub fn item_modified_appearance_store(&self) -> Option<&Arc<ItemModifiedAppearanceStore>> {
        self.items.modified_appearance_store.as_ref()
    }
    /// Set the transmog set item store for this session.
    pub fn set_transmog_set_item_store(&mut self, store: Arc<TransmogSetItemStore>) {
        self.transmog_set_item_store = Some(store);
    }
    /// Get the transmog set item store reference.
    pub fn transmog_set_item_store(&self) -> Option<&Arc<TransmogSetItemStore>> {
        self.transmog_set_item_store.as_ref()
    }
    /// C++ `DB2Manager::GetTransmogSetItems`.
    pub fn transmog_set_items_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Option<&[wow_data::TransmogSetItemEntry]> {
        self.transmog_set_item_store
            .as_ref()
            .and_then(|store| store.get_transmog_set_items_like_cpp(transmog_set_id))
    }
    /// C++ `DB2Manager::GetTransmogSetsForItemModifiedAppearance`.
    pub fn transmog_sets_for_item_modified_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<&[TransmogSetEntry]> {
        self.transmog_set_item_store.as_ref().and_then(|store| {
            store.get_transmog_sets_for_item_modified_appearance_like_cpp(
                item_modified_appearance_id,
            )
        })
    }
    /// C++ `CollectionMgr::AddTransmogSet` expansion before `AddItemAppearance`.
    pub fn transmog_set_item_modified_appearances_like_cpp(
        &self,
        transmog_set_id: u32,
    ) -> Vec<&wow_data::ItemModifiedAppearanceEntry> {
        let Some(items) = self.transmog_set_items_like_cpp(transmog_set_id) else {
            return Vec::new();
        };
        let Some(item_modified_appearance_store) = self.items.modified_appearance_store.as_ref()
        else {
            return Vec::new();
        };

        items
            .iter()
            .filter_map(|item| item_modified_appearance_store.get(item.item_modified_appearance_id))
            .collect()
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance`.
    pub fn add_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let block_index = usize::try_from(item_modified_appearance_id / 32).ok()?;
        let bit_index = item_modified_appearance_id % 32;
        let flag = 1_u32.checked_shl(bit_index)?;
        let had_temporary = self
            .player_collection_state_snapshot_like_cpp()?
            .temporary_item_appearances
            .contains_key(&item_modified_appearance_id);

        let result = self.mutate_canonical_player_like_cpp(|player| {
            while player.transmog_blocks_like_cpp().len() <= block_index {
                player.add_transmog_block_like_cpp(0);
            }

            let added_flag = player.add_transmog_flag_like_cpp(block_index, flag);
            if had_temporary {
                player.remove_conditional_transmog_like_cpp(item_modified_appearance_id);
            }

            added_flag.then(|| player.values_update(true))
        })??;

        self.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .item_appearances
                .insert(item_modified_appearance_id);
            if had_temporary {
                collections
                    .temporary_item_appearances
                    .remove(&item_modified_appearance_id);
            }
        })?;
        self.update_represented_transmog_criteria_like_cpp(item_modified_appearance_id);
        Some(result)
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance(uint32, uint32)`.
    ///
    /// This resolves `ItemModifiedAppearance` by `(item_id, appearance_mod_id)` like
    /// `sDB2Manager.GetItemModifiedAppearance`; the full C++ `CanAddAppearance`
    /// item-template/proficiency/quality gate remains a separate slice.
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
        if self
            .items
            .search_name_store
            .as_ref()
            .and_then(|store| store.get(item_id))
            .is_none()
        {
            return false;
        }

        let Some(item_record) = self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };
        let Some(sparse_template) = self
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

        let player_class_mask =
            player_class_mask_for_transmog_like_cpp(self.player_class_like_cpp());
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
                let weapon_proficiency =
                    wow_packet::packets::misc::SetProficiency::default_weapons(
                        self.player_class_like_cpp(),
                    )
                    .proficiency_mask;
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

        self.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| {
                !collections
                    .item_appearances
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
        let player_class_mask =
            player_class_mask_for_transmog_like_cpp(self.player_class_like_cpp());
        let package_item_ids = self
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
            let Some(item_spec_class_mask) =
                self.item_spec_class_mask_from_overrides_like_cpp(item_id)
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
    #[cfg(test)]
    pub(crate) fn represented_has_item_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        self.represented_item_appearances_like_cpp
            .contains(&item_modified_appearance_id)
    }
    /// C++ `CollectionMgr::HasItemAppearance`.
    pub fn has_item_appearance_like_cpp(&self, item_modified_appearance_id: u32) -> (bool, bool) {
        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return (false, false);
        };
        if collections
            .item_appearances
            .contains(&item_modified_appearance_id)
        {
            return (true, false);
        }

        if collections
            .temporary_item_appearances
            .contains_key(&item_modified_appearance_id)
        {
            return (true, true);
        }

        (false, false)
    }
    /// C++ `CollectionMgr::LoadAccountItemAppearances` active-player create data order.
    pub(crate) fn account_transmog_active_player_rows_like_cpp(&self) -> Vec<u32> {
        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return Vec::new();
        };
        if !collections.item_appearance_blocks.is_empty() {
            return collections.item_appearance_blocks;
        }

        if let Some(blocks) = self
            .canonical_player_snapshot_like_cpp(|player| player.transmog_blocks_like_cpp().to_vec())
        {
            return blocks;
        }

        let Some(highest_appearance) = collections.item_appearances.iter().max() else {
            return Vec::new();
        };

        let mut blocks = vec![0_u32; (highest_appearance / 32 + 1) as usize];
        for &item_modified_appearance_id in &collections.item_appearances {
            let block_index = (item_modified_appearance_id / 32) as usize;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                blocks[block_index] |= flag;
            }
        }

        blocks
    }
    /// C++ `CollectionMgr::LoadAccountItemAppearances`.
    pub(crate) fn load_represented_account_item_appearances_like_cpp(
        &mut self,
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

            self.mutate_canonical_player_like_cpp(|player| {
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
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.item_appearances = item_appearances;
            collections.item_appearance_blocks = item_appearance_blocks;
            collections.favorite_item_appearances = favorite_item_appearances;
        });
    }
    /// C++ `CollectionMgr::SaveAccountItemAppearances`.
    pub(crate) fn account_item_appearance_save_plan_like_cpp(
        &mut self,
    ) -> Option<AccountItemAppearanceSavePlanLikeCpp> {
        let mut collections = self.player_collection_state_snapshot_like_cpp()?;
        let mut blocks = BTreeMap::<u32, u32>::new();
        for &item_modified_appearance_id in &collections.item_appearances {
            let block_index = item_modified_appearance_id / 32;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                *blocks.entry(block_index).or_default() |= flag;
            }
        }

        let mut favorite_inserts = Vec::new();
        let mut favorite_deletes = Vec::new();
        let favorite_states = collections
            .favorite_item_appearances
            .iter()
            .map(|(&appearance, &state)| (appearance, state))
            .collect::<BTreeMap<_, _>>();
        for (item_modified_appearance_id, state) in favorite_states {
            match state {
                FavoriteAppearanceStateLikeCpp::New => {
                    favorite_inserts.push(item_modified_appearance_id);
                    collections.favorite_item_appearances.insert(
                        item_modified_appearance_id,
                        FavoriteAppearanceStateLikeCpp::Unchanged,
                    );
                }
                FavoriteAppearanceStateLikeCpp::Removed => {
                    favorite_deletes.push(item_modified_appearance_id);
                    collections
                        .favorite_item_appearances
                        .remove(&item_modified_appearance_id);
                }
                FavoriteAppearanceStateLikeCpp::Unchanged => {}
            }
        }

        let plan = AccountItemAppearanceSavePlanLikeCpp {
            appearance_blocks: blocks
                .into_iter()
                .filter(|(_, appearance_mask)| *appearance_mask != 0)
                .collect(),
            favorite_inserts,
            favorite_deletes,
        };
        let _ = self.replace_player_collection_state_like_cpp(collections);
        Some(plan)
    }
    /// C++ `CollectionMgr::SetAppearanceIsFavorite`.
    pub fn set_appearance_is_favorite_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        use FavoriteAppearanceStateLikeCpp::{New, Removed, Unchanged};
        use std::collections::hash_map::Entry;

        let changed = self
            .mutate_player_collection_state_like_cpp(|collections| {
                if apply {
                    match collections
                        .favorite_item_appearances
                        .entry(item_modified_appearance_id)
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
                        .favorite_item_appearances
                        .entry(item_modified_appearance_id)
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
            if !crate::session_rules::account_transmog_update_opcode_resolved_like_cpp() {
                warn!(
                    "Skipping AccountTransmogUpdate favorite delta: legacy C++ opcode is unresolved 0xBADD for 54261"
                );
                return changed;
            }

            self.send_packet(
                &wow_packet::packets::collection::AccountTransmogUpdate::favorite_delta(
                    item_modified_appearance_id,
                    apply,
                ),
            );
        }

        changed
    }
    /// C++ `CollectionMgr::SendFavoriteAppearances`.
    pub fn send_favorite_appearances_like_cpp(&self) {
        if !crate::session_rules::account_transmog_update_opcode_resolved_like_cpp() {
            warn!(
                "Skipping AccountTransmogUpdate full update: legacy C++ opcode is unresolved 0xBADD for 54261"
            );
            return;
        }

        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return;
        };
        let favorite_appearances = collections
            .favorite_item_appearances
            .into_iter()
            .filter_map(|(appearance, state)| {
                (state != FavoriteAppearanceStateLikeCpp::Removed).then_some(appearance)
            })
            .collect::<Vec<_>>();

        self.send_packet(&wow_packet::packets::collection::AccountTransmogUpdate {
            is_full_update: true,
            is_set_favorite: false,
            favorite_appearances,
            new_appearances: Vec::new(),
        });
    }
    /// C++ `CollectionMgr::LoadAccountTransmogIllusions`.
    pub(crate) fn load_represented_account_transmog_illusions_like_cpp(
        &mut self,
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
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.transmog_illusions = illusions;
        });
    }
    /// C++ `CollectionMgr::HasTransmogIllusion`.
    #[allow(dead_code)]
    pub(crate) fn has_transmog_illusion_like_cpp(&self, transmog_illusion_id: u32) -> bool {
        self.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| {
                collections
                    .transmog_illusions
                    .contains(&transmog_illusion_id)
            })
    }
    /// C++ `CollectionMgr::SaveAccountTransmogIllusions`.
    pub(crate) fn account_transmog_illusion_save_plan_like_cpp(
        &self,
    ) -> Option<AccountTransmogIllusionSavePlanLikeCpp> {
        let mut blocks = BTreeMap::<u32, u32>::new();
        let collections = self.player_collection_state_snapshot_like_cpp()?;
        for &illusion_id in &collections.transmog_illusions {
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
    #[cfg(test)]
    pub(crate) fn represented_favorite_item_appearance_state_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<FavoriteAppearanceStateLikeCpp> {
        self.represented_favorite_item_appearances_like_cpp
            .get(&item_modified_appearance_id)
            .copied()
    }
    /// C++ `CollectionMgr::AddTemporaryAppearance`.
    pub fn add_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let was_empty = self.mutate_player_collection_state_like_cpp(|collections| {
            let items = collections
                .temporary_item_appearances
                .entry(item_modified_appearance_id)
                .or_default();
            let was_empty = items.is_empty();
            items.insert(item_guid);
            was_empty
        })?;

        was_empty.then_some(())?;
        self.mutate_canonical_player_like_cpp(|player| {
            player.add_conditional_transmog_like_cpp(item_modified_appearance_id);
            player.values_update(true)
        })
    }
    /// C++ `CollectionMgr::RemoveTemporaryAppearance`.
    pub fn remove_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let removed_last = self.mutate_player_collection_state_like_cpp(|collections| {
            let items = collections
                .temporary_item_appearances
                .get_mut(&item_modified_appearance_id)?;
            if !items.remove(&item_guid) || !items.is_empty() {
                return None;
            }
            collections
                .temporary_item_appearances
                .remove(&item_modified_appearance_id);
            Some(())
        })??;
        let _ = removed_last;
        self.mutate_canonical_player_like_cpp(|player| {
            player.remove_conditional_transmog_like_cpp(item_modified_appearance_id);
            player.values_update(true)
        })
    }
    /// C++ `CollectionMgr::GetItemsProvidingTemporaryAppearance`.
    pub fn items_providing_temporary_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        self.player_collection_state_snapshot_like_cpp()
            .and_then(|collections| {
                collections
                    .temporary_item_appearances
                    .get(&item_modified_appearance_id)
                    .cloned()
            })
            .unwrap_or_default()
    }
    /// C++ `CollectionMgr::AddItemAppearance` criteria side effects.
    fn update_represented_transmog_criteria_like_cpp(&mut self, item_modified_appearance_id: u32) {
        let item_id = self
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(item_modified_appearance_id))
            .and_then(|appearance| u32::try_from(appearance.item_id).ok());

        if let Some(_transmog_slot) = item_id
            .and_then(|item_id| {
                self.items
                    .store
                    .as_ref()
                    .and_then(|store| store.inventory_type(item_id))
            })
            .and_then(crate::session_rules::item_transmogrification_slot_like_cpp)
        {
            #[cfg(test)]
            self.represented_transmog_criteria_events.push(
                RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                    equipment_slot: _transmog_slot as u32,
                    item_modified_appearance_id,
                },
            );
        }

        let transmog_sets = self
            .transmog_sets_for_item_modified_appearance_like_cpp(item_modified_appearance_id)
            .map(|sets| {
                sets.iter()
                    .map(|set| (set.id, set.transmog_set_group_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for (transmog_set_id, _transmog_set_group_id) in transmog_sets {
            if self.is_transmog_set_completed_like_cpp(transmog_set_id) {
                #[cfg(test)]
                self.represented_transmog_criteria_events.push(
                    RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                        transmog_set_group_id: _transmog_set_group_id,
                    },
                );
            }
        }
    }
    /// C++ `CollectionMgr::IsSetCompleted`.
    pub fn is_transmog_set_completed_like_cpp(&self, transmog_set_id: u32) -> bool {
        let Some(transmog_set_items) = self.transmog_set_items_like_cpp(transmog_set_id) else {
            return false;
        };

        let mut known_pieces = [-1_i8; EQUIPMENT_SLOT_END as usize];
        for transmog_set_item in transmog_set_items {
            let Some(item_modified_appearance) = self
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
            let Some(inventory_type) = self
                .items
                .store
                .as_ref()
                .and_then(|store| store.inventory_type(item_id))
            else {
                continue;
            };
            let Some(transmog_slot) =
                crate::session_rules::item_transmogrification_slot_like_cpp(inventory_type)
            else {
                continue;
            };
            if known_pieces[transmog_slot] == 1 {
                continue;
            }

            let (has_appearance, is_temporary) =
                self.has_item_appearance_like_cpp(transmog_set_item.item_modified_appearance_id);
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
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let appearance_ids = self
            .transmog_set_item_modified_appearances_like_cpp(transmog_set_id)
            .into_iter()
            .map(|appearance| appearance.id)
            .collect::<Vec<_>>();

        let mut last_update = None;
        for appearance_id in appearance_ids {
            if let Some(update) = self.add_item_appearance_like_cpp(appearance_id) {
                last_update = Some(update);
            }
        }

        last_update
    }
    /// Build the closure result expected by `Item::visible_entry` and
    /// `Item::visible_appearance_mod_id` from `ItemModifiedAppearance.db2`.
    pub fn item_modified_appearance_ref(&self, id: u32) -> Option<(u32, u16)> {
        self.items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(id))
            .and_then(|entry| {
                Some((
                    u32::try_from(entry.item_id).ok()?,
                    u16::try_from(entry.item_appearance_modifier_id).ok()?,
                ))
            })
    }
    /// C++ `DB2Manager::GetItemModifiedAppearance`.
    pub fn item_modified_appearance_for_item(
        &self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<u32> {
        self.items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get_for_item(item_id, appearance_mod_id))
            .map(|entry| entry.id)
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_alter_appearance_like_cpp(
        &mut self,
        request: RepresentedAlterAppearanceLikeCpp,
    ) {
        #[cfg(test)]
        {
            self.represented_alter_appearance_requests_like_cpp
                .push(request);
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_alter_appearance_requests_like_cpp(
        &self,
    ) -> &[RepresentedAlterAppearanceLikeCpp] {
        &self.represented_alter_appearance_requests_like_cpp
    }
}
