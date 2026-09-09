//! Represented equipment sets and outfits.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn with_owned_equipment_sets_like_cpp<R>(
        &self,
        mut f: impl FnMut(&BTreeMap<u64, RepresentedEquipmentSetLikeCpp>, bool) -> R,
    ) -> Option<R> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            f(&state.equipment_sets, state.equipment_sets_loaded)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(f(
                &self.represented_equipment_sets_like_cpp,
                self.represented_equipment_sets_loaded_like_cpp,
            ));
        }
        None
    }
    pub(in crate::session) fn with_owned_equipment_sets_mut_like_cpp<R>(
        &mut self,
        mut f: impl FnMut(&mut BTreeMap<u64, RepresentedEquipmentSetLikeCpp>, &mut bool) -> R,
    ) -> Option<R> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let state = player.gameplay_state_mut();
            f(&mut state.equipment_sets, &mut state.equipment_sets_loaded)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(f(
                &mut self.represented_equipment_sets_like_cpp,
                &mut self.represented_equipment_sets_loaded_like_cpp,
            ));
        }
        None
    }
    #[cfg(test)]
    pub(crate) fn insert_represented_equipment_set_like_cpp(
        &mut self,
        guid: u64,
        equipment_set: RepresentedEquipmentSetLikeCpp,
    ) {
        let _ = self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            sets.insert(guid, equipment_set.clone());
        });
    }
    pub(crate) fn clear_represented_equipment_sets_like_cpp(&mut self) {
        let _ = self.with_owned_equipment_sets_mut_like_cpp(|sets, loaded| {
            sets.clear();
            *loaded = false;
        });
    }
    pub(crate) fn mark_represented_equipment_sets_loaded_like_cpp(&mut self) {
        let _ = self.with_owned_equipment_sets_mut_like_cpp(|_, loaded| *loaded = true);
    }
    pub(crate) fn load_represented_equipment_set_row_like_cpp(
        &mut self,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        assigned_spec_index: i32,
        pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ) -> bool {
        if set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return false;
        }

        let equipment_set = RepresentedEquipmentSetLikeCpp {
            raw_set_type: RepresentedEquipmentSetTypeLikeCpp::Equipment.as_i32_like_cpp(),
            set_type: RepresentedEquipmentSetTypeLikeCpp::Equipment,
            guid,
            set_id,
            ignore_mask,
            pieces,
            appearances: [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_shoulder_appearance_id: 0,
            secondary_shoulder_slot: 0,
            secondary_weapon_appearance_id: 0,
            secondary_weapon_slot: 0,
            assigned_spec_index,
            set_name,
            set_icon,
            state: RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        };
        self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            sets.insert(guid, equipment_set.clone());
        })
        .is_some()
    }
    pub(crate) fn represented_load_equipment_set_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadEquipmentSet> {
        self.with_owned_equipment_sets_like_cpp(|stored, _| {
            let sets = stored
                .values()
                .filter(|equipment_set| {
                    equipment_set.state != RepresentedEquipmentSetUpdateStateLikeCpp::Deleted
                })
                .map(
                    |equipment_set| wow_packet::packets::misc::EquipmentSetDataLikeCpp {
                        set_type: equipment_set.raw_set_type,
                        guid: equipment_set.guid,
                        set_id: equipment_set.set_id,
                        ignore_mask: equipment_set.ignore_mask,
                        pieces: equipment_set.pieces,
                        appearances: equipment_set.appearances,
                        enchants: equipment_set.enchants,
                        secondary_shoulder_appearance_id: equipment_set
                            .secondary_shoulder_appearance_id,
                        secondary_shoulder_slot: equipment_set.secondary_shoulder_slot,
                        secondary_weapon_appearance_id: equipment_set
                            .secondary_weapon_appearance_id,
                        secondary_weapon_slot: equipment_set.secondary_weapon_slot,
                        assigned_spec_index: equipment_set.assigned_spec_index,
                        set_name: equipment_set.set_name.clone(),
                        set_icon: equipment_set.set_icon.clone(),
                    },
                )
                .collect();
            wow_packet::packets::misc::LoadEquipmentSet { sets }
        })
    }
    #[cfg(test)]
    pub(crate) fn represented_equipment_set_like_cpp(
        &self,
        guid: u64,
    ) -> Option<RepresentedEquipmentSetLikeCpp> {
        self.with_owned_equipment_sets_like_cpp(|sets, _| sets.get(&guid).cloned())?
    }
    pub(crate) fn save_represented_equipment_set_with_generator_like_cpp(
        &mut self,
        generator: &EquipmentSetGuidGeneratorLikeCpp,
        mut set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    ) -> Option<RepresentedEquipmentSetSavedLikeCpp> {
        if set.set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return None;
        }

        let set_type =
            RepresentedEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;

        for i in 0..wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP {
            let slot_bit = 1_u32 << i;
            if (set.ignore_mask & slot_bit) == 0 {
                if set_type == RepresentedEquipmentSetTypeLikeCpp::Equipment {
                    set.appearances[i] = 0;

                    let item_guid = set.pieces[i];
                    if !item_guid.is_empty() {
                        let item = self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, i as u8)?;
                        if item.guid != item_guid {
                            return None;
                        }
                    } else {
                        set.ignore_mask |= slot_bit;
                    }
                } else {
                    set.pieces[i] = ObjectGuid::EMPTY;
                    if set.appearances[i] != 0 {
                        let appearance_id = u32::try_from(set.appearances[i]).ok()?;
                        if self
                            .item_modified_appearance_store()
                            .is_some_and(|store| store.get(appearance_id).is_none())
                        {
                            return None;
                        }

                        if !self.has_item_appearance_like_cpp(appearance_id).0 {
                            return None;
                        }
                    } else {
                        set.ignore_mask |= slot_bit;
                    }
                }
            } else {
                set.pieces[i] = ObjectGuid::EMPTY;
                set.appearances[i] = 0;
            }
        }

        set.ignore_mask &= 0x7_FFFF;
        if set_type == RepresentedEquipmentSetTypeLikeCpp::Equipment {
            set.enchants = [0, 0];
        } else {
            for enchant_id in set.enchants {
                if enchant_id == 0 {
                    continue;
                }

                let enchant_id = u32::try_from(enchant_id).ok()?;
                if self.spell_item_enchantment_store().is_some_and(|store| {
                    store.get(enchant_id).is_none_or(|illusion| {
                        illusion.item_visual == 0
                            || !illusion
                                .flags
                                .contains(SpellItemEnchantmentFlags::ALLOW_TRANSMOG)
                    })
                }) {
                    return None;
                }
            }
        }

        let existing_state = self.with_owned_equipment_sets_like_cpp(|sets, _| {
            sets.get(&set.guid).map(|equipment_set| equipment_set.state)
        })?;
        if set.guid != 0 && existing_state.is_none() {
            return None;
        }

        let generated_new_guid = set.guid == 0;
        let guid = if generated_new_guid {
            generator.generate()
        } else {
            set.guid
        };
        set.guid = guid;

        let next_state = existing_state
            .map(|state| {
                if state == RepresentedEquipmentSetUpdateStateLikeCpp::New {
                    RepresentedEquipmentSetUpdateStateLikeCpp::New
                } else {
                    RepresentedEquipmentSetUpdateStateLikeCpp::Changed
                }
            })
            .unwrap_or(RepresentedEquipmentSetUpdateStateLikeCpp::New);

        let saved = represented_equipment_set_from_packet_like_cpp(set, guid, next_state)?;
        let saved_raw_type = saved.raw_set_type;
        let saved_set_id = saved.set_id;
        self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            sets.insert(guid, saved.clone());
        })?;

        Some(RepresentedEquipmentSetSavedLikeCpp {
            guid,
            set_type,
            raw_set_type: saved_raw_type,
            set_id: saved_set_id,
            generated_new_guid,
        })
    }
    pub(crate) fn assign_represented_equipment_set_to_spec_like_cpp(
        &mut self,
        set_id: u32,
        spec_index: u32,
    ) -> bool {
        if set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return false;
        }

        self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            let Some((_, equipment_set)) = sets.iter_mut().find(|(_, equipment_set)| {
                equipment_set.set_id == set_id
                    && equipment_set.set_type == RepresentedEquipmentSetTypeLikeCpp::Equipment
            }) else {
                return false;
            };

            equipment_set.assigned_spec_index = spec_index as i32;
            if equipment_set.state != RepresentedEquipmentSetUpdateStateLikeCpp::New {
                equipment_set.state = RepresentedEquipmentSetUpdateStateLikeCpp::Changed;
            }
            true
        })
        .unwrap_or(false)
    }
    pub(crate) fn delete_represented_equipment_set_like_cpp(&mut self, id: u64) -> bool {
        self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            let Some(equipment_set) = sets.get_mut(&id) else {
                return false;
            };
            if equipment_set.state == RepresentedEquipmentSetUpdateStateLikeCpp::New {
                sets.remove(&id);
            } else {
                equipment_set.state = RepresentedEquipmentSetUpdateStateLikeCpp::Deleted;
            }
            true
        })
        .unwrap_or(false)
    }
    fn ignored_equipment_set_item_guid_like_cpp() -> ObjectGuid {
        ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64)
    }
    pub(crate) fn use_represented_equipment_set_like_cpp(
        &mut self,
        request: &wow_packet::packets::misc::UseEquipmentSet,
    ) -> bool {
        let ignored_guid = Self::ignored_equipment_set_item_guid_like_cpp();
        let mut changed_equipment = false;
        let mut represented_item_mods_changed = false;

        for (slot_index, set_item) in request.items.iter().enumerate() {
            let dst = slot_index as u8;
            if set_item.item == ignored_guid {
                continue;
            }

            if self.resolved_in_combat_like_cpp() != Some(false)
                && dst != EQUIPMENT_SLOT_MAINHAND
                && dst != EQUIPMENT_SLOT_OFFHAND
            {
                continue;
            }

            if let Some((src, _item)) =
                self.represented_direct_inventory_slot_by_guid_like_cpp(set_item.item)
            {
                if src == dst {
                    continue;
                }
                if let Some(item_mods_changed) =
                    self.move_represented_direct_inventory_item_with_item_mods_like_cpp(src, dst)
                {
                    represented_item_mods_changed |= item_mods_changed;
                    changed_equipment |= dst < EQUIPMENT_SLOT_END;
                    changed_equipment |= src < EQUIPMENT_SLOT_END;
                }
                continue;
            }

            let Some(_equipped_item) = self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, dst)
            else {
                continue;
            };
            let Some(backpack_slot) = self.find_free_backpack_slot_like_cpp() else {
                continue;
            };
            if let Some(item_mods_changed) = self
                .move_represented_direct_inventory_item_with_item_mods_like_cpp(dst, backpack_slot)
            {
                represented_item_mods_changed |= item_mods_changed;
                changed_equipment = true;
            }
        }

        if changed_equipment {
            self.sync_player_registry_state_like_cpp();
        }

        represented_item_mods_changed
    }
    /// Install the process-wide C++ `sObjectMgr->GenerateEquipmentSetGuid()`
    /// mirror shared by equipment sets and transmog outfits for every player.
    #[cfg(test)]
    pub fn set_equipment_set_guid_generator_like_cpp(
        &mut self,
        generator: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    ) {
        self.equipment_set_guid_generator_like_cpp = Some(generator);
    }
    #[cfg(test)]
    pub(crate) fn equipment_set_guid_generator_for_test_like_cpp(
        &self,
    ) -> Option<Arc<EquipmentSetGuidGeneratorLikeCpp>> {
        self.equipment_set_guid_generator_like_cpp.clone()
    }
    /// Set `ItemChildEquipment.db2`, used by C++ `CanEquipChildItem` and
    /// `EquipChildItem` to move a linked child into its visible equipment slot.
    pub fn set_item_child_equipment_store(&mut self, store: Arc<ItemChildEquipmentStore>) {
        self.item_child_equipment_store = Some(store);
    }
    pub(crate) fn apply_initial_equipped_item_set_auras_like_cpp(&mut self) -> Option<usize> {
        let mut equipped: Vec<_> = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(
            equipped
                .into_iter()
                .map(|(_slot, item_guid)| self.apply_initial_item_set_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    #[cfg(test)]
    pub fn set_creature_equipment_store_like_cpp(
        &mut self,
        store: Arc<CreatureEquipmentStoreLikeCpp>,
    ) {
        self.creature_equipment_store_like_cpp = Some(store);
    }
    pub fn set_spell_equipped_items_store(&mut self, store: Arc<SpellEquippedItemsStore>) {
        self.spell_catalogs.spell_equipped_items_store = Some(store);
    }
    /// Builds the represented statement sequence for C++ `Player::_SaveSpells`.
    ///
    /// The runtime full-save path calls this only when the complete represented
    /// `PlayerSpellMap` authority can preserve inactive/disabled/temporary rows
    /// exactly; incomplete snapshots remain fail-closed.
    /// Persist a direct C++ `LearnSpell` result when Rust has not yet hydrated
    /// the complete `PlayerSpellMap`. The DB row is the missing authority:
    /// preserve its active bit if it was disabled, activate it otherwise, and
    /// clear disabled. Favorites are deliberately untouched because learning
    /// preserves them and the incomplete runtime cannot reconstruct them.
    #[cfg(test)]
    pub(in crate::session) fn mark_equipment_sets_saved_like_cpp(&mut self) {
        let _ = self.with_owned_equipment_sets_mut_like_cpp(|sets, _| {
            sets.retain(|_, equipment_set| {
                if equipment_set.state == RepresentedEquipmentSetUpdateStateLikeCpp::Deleted {
                    return false;
                }
                equipment_set.state = RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged;
                true
            });
        });
    }
}
