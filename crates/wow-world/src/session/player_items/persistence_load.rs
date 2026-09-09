//! Loading represented item and inventory state from persistence.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn can_use_inventory_item_represented_with_loading_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
        not_loading: bool,
    ) -> InventoryResult {
        let Some(player) = self.direct_inventory_player_snapshot() else {
            return InventoryResult::ItemNotFound;
        };
        let proto = self.item_storage_template(item.entry_id);
        let sparse = self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item.entry_id));
        let search = self
            .items
            .search_name_store
            .as_ref()
            .and_then(|store| store.get(item.entry_id));

        let flags2 = sparse.map_or(0, |template| template.flags[1]);
        let player_class_mask = self
            .player_class_like_cpp()
            .checked_sub(1)
            .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
            .unwrap_or(0);
        let player_race_mask = self
            .player_race_like_cpp()
            .checked_sub(1)
            .and_then(|shift| 1i64.checked_shl(u32::from(shift)))
            .unwrap_or(0);
        let allowable_class_matches = search
            .map(|entry| {
                entry.allowable_class == 0
                    || (entry.allowable_class & player_class_mask as i32) != 0
            })
            .unwrap_or(true);
        let allowable_race_matches = search
            .map(|entry| {
                entry.allowable_race == 0 || (entry.allowable_race & player_race_mask) != 0
            })
            .unwrap_or(true);
        let required_skill = search.map_or(0, |entry| u32::from(entry.required_skill));
        let required_skill_rank = search.map_or(0, |entry| u32::from(entry.required_skill_rank));
        let required_skill_value = match u16::try_from(required_skill).ok() {
            Some(skill) => {
                let Some(value) = self.resolved_player_skill_value_like_cpp(skill) else {
                    return InventoryResult::ItemNotFound;
                };
                u32::from(value)
            }
            None => 0,
        };
        let required_spell = search.map_or(0, |entry| entry.required_ability);
        let has_required_spell = required_spell == 0
            || i32::try_from(required_spell)
                .ok()
                .is_some_and(|spell_id| self.known_spells_like_cpp().contains(&spell_id));
        let base_required_level = search
            .and_then(|entry| u8::try_from(entry.required_level.max(0)).ok())
            .unwrap_or(0);
        let required_reputation_faction = sparse.map_or(0, |template| {
            u32::from(template.required_reputation_faction)
        });
        let required_reputation_rank = sparse
            .and_then(|template| u32::try_from(template.required_reputation_rank.max(0)).ok())
            .unwrap_or(0);
        let player_reputation_rank = if required_reputation_faction == 0 {
            0
        } else {
            match self
                .factions
                .store
                .as_ref()
                .and_then(|store| store.get(required_reputation_faction))
            {
                Some(faction) => {
                    let Some(standing) = self.with_reputation_mgr_like_cpp(|mgr| {
                        mgr.reputation_for_faction_like_cpp(
                            faction,
                            self.player_race_like_cpp(),
                            self.player_class_like_cpp(),
                        )
                    }) else {
                        return InventoryResult::ItemNotFound;
                    };
                    u32::from(
                        reputation_to_rank_like_cpp(
                            faction,
                            standing,
                            self.friendship_rep_reaction_store.as_deref(),
                        )
                        .as_u8(),
                    )
                }
                None => 0,
            }
        };
        let mut item_effect_spell_ids: Vec<(u8, i32)> = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .values()
                    .filter(|effect| effect.parent_item_id == item.entry_id)
                    .map(|effect| (effect.legacy_slot_index, effect.spell_id))
                    .collect()
            })
            .unwrap_or_default();
        item_effect_spell_ids.sort_by_key(|(slot, _)| *slot);
        let effect0_spell_id = item_effect_spell_ids
            .first()
            .and_then(|(_, spell_id)| u32::try_from(*spell_id).ok());
        let effect1_spell_id = item_effect_spell_ids
            .get(1)
            .and_then(|(_, spell_id)| u32::try_from(*spell_id).ok());
        let has_effect1_spell = effect1_spell_id
            .and_then(|spell_id| i32::try_from(spell_id).ok())
            .is_some_and(|spell_id| self.known_spells_like_cpp().contains(&spell_id));
        let quality = self.item_template_quality(item.entry_id).unwrap_or(0);

        player.can_use_item(CanUseItemArgs {
            source_item: runtime_item,
            proto: proto.as_ref(),
            not_loading,
            is_alive: true,
            player_level: self.player_level_like_cpp(),
            item_required_level: base_required_level,
            source_bop_trade_allowed_for_player: false,
            template_args: CanUseItemTemplateArgs {
                proto: proto.as_ref(),
                skip_required_level_check: false,
                player_level: self.player_level_like_cpp(),
                team: player_team_id_for_race_cpp(self.player_race_like_cpp()),
                allowable_class_matches,
                allowable_race_matches,
                internal_item: (flags2 & ItemFlags2::InternalItem as u32) != 0,
                faction_horde: (flags2 & ItemFlags2::FactionHorde as u32) != 0,
                faction_alliance: (flags2 & ItemFlags2::FactionAlliance as u32) != 0,
                required_skill,
                required_skill_rank,
                required_skill_value,
                required_spell,
                has_required_spell,
                base_required_level,
                holiday_id: 0,
                holiday_active: false,
                required_reputation_faction,
                required_reputation_rank,
                player_reputation_rank,
                effect0_spell_id,
                effect1_spell_id,
                has_effect1_spell,
                artifact_specialization: None,
                primary_specialization: player.primary_specialization_id_like_cpp(),
            },
            item_skill: 0,
            item_skill_value: 0,
            has_item_skill: false,
            player_class: self.player_class_like_cpp(),
            proto_is_heirloom: quality == ItemQuality::Heirloom as i8,
        })
    }
    /// C++ `Player::_LoadInventory` collection side effects for a loaded item.
    pub(crate) fn apply_loaded_inventory_item_collection_hooks_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) {
        let _ = self.check_account_heirloom_upgrades_like_cpp(item.object().entry());
        let _ = self.add_item_appearance_for_runtime_item_like_cpp(item);
    }
    pub(crate) fn loaded_inventory_item_visible_fields_like_cpp(
        &self,
        item: &Item,
    ) -> (i32, u16, u16) {
        (
            item.object().entry() as i32,
            0,
            item.visible_item_visual(0, |enchantment_id| {
                self.spell_item_enchantment_store()
                    .and_then(|store| store.get(enchantment_id))
                    .map(|entry| entry.item_visual)
            }),
        )
    }
    pub(in crate::session) fn loaded_inventory_item_visible_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, i32, u16, u16)> {
        let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        let slot = item.slot();
        if slot >= EQUIPMENT_SLOT_END {
            return None;
        }
        let (item_id, appearance_mod_id, item_visual) =
            self.loaded_inventory_item_visible_fields_like_cpp(&item);
        Some((slot, item_id, appearance_mod_id, item_visual))
    }
    /// Restores C++ `Player::_LoadInventory` duration tracking before the login
    /// create packet is sent. Equipped enchantments are registered later by the
    /// ordered `_ApplyAllItemMods` replay, so only non-equipped enchantments are
    /// added here.
    pub(crate) fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (Vec<PlayerItemTimeUpdate>, Vec<PlayerEnchantTimeUpdate>) {
        let mut item_updates = Vec::new();
        let mut enchantment_updates = Vec::new();

        for &item_guid in loaded_item_guids {
            let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
                continue;
            };
            let is_equipped = loaded_equipped_item_guids.contains(&item_guid);
            let Some((item_update, mut item_enchantment_updates)) = self
                .mutate_canonical_player_like_cpp(|player| {
                    let item_update = player.add_item_durations(&item);
                    let enchantment_updates = if is_equipped {
                        Vec::new()
                    } else {
                        player.add_enchantment_durations(&mut item)
                    };
                    (item_update, enchantment_updates)
                })
            else {
                continue;
            };

            self.insert_inventory_item_object(item);
            if let Some(item_update) = item_update {
                item_updates.push(item_update);
            }
            enchantment_updates.append(&mut item_enchantment_updates);
        }

        (item_updates, enchantment_updates)
    }
    /// Begin hydrating the persisted equipment/inventory source for the active
    /// Player. The proof remains incomplete on every non-empty, early-return,
    /// or query-error path; this bounded slice authorizes only a proven-empty
    /// persisted result.
    pub(crate) fn begin_player_equipment_inventory_authority_load_like_cpp(&mut self) {
        let _canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .inventory_runtime_mut_like_cpp()
                    .set_equipment_inventory_authority_complete_like_cpp(false);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.player_equipment_inventory_authority_complete_like_cpp = false;
        }
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    pub(crate) fn complete_player_equipment_inventory_authority_load_like_cpp(&mut self) {
        let _canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .inventory_runtime_mut_like_cpp()
                    .set_equipment_inventory_authority_complete_like_cpp(true);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.player_equipment_inventory_authority_complete_like_cpp = true;
        }
    }
    pub(crate) fn player_equipment_inventory_authority_complete_like_cpp(&self) -> bool {
        let canonical = self
            .with_owned_player_like_cpp(|player| {
                player
                    .inventory_runtime_like_cpp()
                    .equipment_inventory_authority_complete_like_cpp()
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.player_equipment_inventory_authority_complete_like_cpp;
        }
        canonical
    }
}
