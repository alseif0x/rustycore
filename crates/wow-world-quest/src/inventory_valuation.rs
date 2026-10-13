// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application coordination for represented item valuation and equipability.

use std::collections::HashMap;

use wow_constants::{InventoryResult, InventoryType, ItemFlags3};
use wow_core::ObjectGuid;
use wow_entities::{
    BagTemplateRef, CanEquipItemArgs, CanEquipItemOutcome, EQUIPMENT_SLOT_END,
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, INVENTORY_SLOT_BAG_0, Item, ItemSlotRef,
    ItemStorageRef, NULL_BAG, NULL_SLOT, PlayerInventoryItem as InventoryItem, is_buyback_slot,
};
use wow_world_core::session::{
    InventoryValuationAccessLikeCpp, InventoryValuationCatalogViewLikeCpp,
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
};
use wow_world_inventory::{InventoryState, is_represented_bag_slot};

use crate::PlayerConditionProjectionCxLikeCpp;

mod direct_storage;

impl PlayerConditionProjectionCxLikeCpp<'_> {
    pub fn item_limit_category_template_like_cpp(
        &self,
        limit_category_id: u32,
    ) -> Option<wow_entities::ItemLimitCategoryTemplate> {
        let player = self.player_access_like_cpp();
        InventoryValuationApplicationCxLikeCpp {
            inventory: self.inventory,
            player_conditions: self,
            catalogs: self.valuation_catalogs,
            inventory_access: player.owned_inventory_access_like_cpp(),
            valuation_access: player.inventory_valuation_access_like_cpp(),
            modifier_access: player.owned_item_modifiers_access_like_cpp(),
        }
        .item_limit_category_template_like_cpp(limit_category_id)
    }
}

pub const MIN_ITEM_LEVEL_LIKE_CPP: u32 = 1;
pub const MAX_ITEM_LEVEL_LIKE_CPP: u32 = 1300;

/// Selected inputs for the complete AvgTotalItemLevel application operation.
/// The context owns no Hub or mutable Player authority; all player projections
/// are requested at their original operation points.
pub struct InventoryValuationApplicationCxLikeCpp<'a> {
    inventory: &'a InventoryState,
    player_conditions: &'a PlayerConditionProjectionCxLikeCpp<'a>,
    catalogs: InventoryValuationCatalogViewLikeCpp<'a>,
    inventory_access: OwnedInventoryAccessLikeCpp<'a>,
    valuation_access: InventoryValuationAccessLikeCpp<'a>,
    modifier_access: OwnedItemModifiersAccessLikeCpp<'a>,
}

/// Run AvgTotalItemLevel at the existing PlayerCondition projection point.
/// The projection is borrowed in place; no Session or Hub is reconstructed.
pub fn represented_avg_total_item_level_like_cpp(
    player_conditions: &PlayerConditionProjectionCxLikeCpp<'_>,
) -> Option<f32> {
    let player = player_conditions.player_access_like_cpp();
    let inventory_access = player.owned_inventory_access_like_cpp();
    let valuation_access = player.inventory_valuation_access_like_cpp();
    let modifier_access = player.owned_item_modifiers_access_like_cpp();
    InventoryValuationApplicationCxLikeCpp {
        inventory: player_conditions.inventory,
        player_conditions,
        catalogs: player_conditions.valuation_catalogs,
        inventory_access,
        valuation_access,
        modifier_access,
    }
    .represented_avg_total_item_level_like_cpp()
}

/// Evaluate represented item usability through the same selected player and
/// catalog owners used by AvgTotalItemLevel. World keeps its existing call
/// sites and passes the original loading mode.
pub fn can_use_inventory_item_represented_with_loading_like_cpp(
    player_conditions: &PlayerConditionProjectionCxLikeCpp<'_>,
    item: &InventoryItem,
    runtime_item: Option<&Item>,
    not_loading: bool,
) -> InventoryResult {
    let player = player_conditions.player_access_like_cpp();
    let inventory_access = player.owned_inventory_access_like_cpp();
    let valuation_access = player.inventory_valuation_access_like_cpp();
    let modifier_access = player.owned_item_modifiers_access_like_cpp();
    InventoryValuationApplicationCxLikeCpp {
        inventory: player_conditions.inventory,
        player_conditions,
        catalogs: player_conditions.valuation_catalogs,
        inventory_access,
        valuation_access,
        modifier_access,
    }
    .can_use_item_like_cpp(item, runtime_item, not_loading)
}

/// Run the existing CanEquipUniqueItem decision through the selected player
/// and catalogs at the World call site.
pub fn can_equip_unique_item_like_cpp(
    player_conditions: &PlayerConditionProjectionCxLikeCpp<'_>,
    entry_id: u32,
    runtime_item: &Item,
    except_slot: u8,
) -> InventoryResult {
    let player = player_conditions.player_access_like_cpp();
    let inventory_access = player.owned_inventory_access_like_cpp();
    let valuation_access = player.inventory_valuation_access_like_cpp();
    let modifier_access = player.owned_item_modifiers_access_like_cpp();
    InventoryValuationApplicationCxLikeCpp {
        inventory: player_conditions.inventory,
        player_conditions,
        catalogs: player_conditions.valuation_catalogs,
        inventory_access,
        valuation_access,
        modifier_access,
    }
    .can_equip_unique_item_like_cpp(entry_id, runtime_item, except_slot)
}

/// Run the existing CanEquipItem decision through the selected player and
/// catalogs, preserving the handler's requested-slot and mode inputs.
#[allow(clippy::too_many_arguments)]
pub fn can_equip_inventory_item_like_cpp(
    player_conditions: &PlayerConditionProjectionCxLikeCpp<'_>,
    inventory_item: &InventoryItem,
    runtime_item: &Item,
    requested_slot: u8,
    swap: bool,
    not_loading: bool,
    is_in_combat: bool,
    can_dual_wield: bool,
    can_titan_grip: bool,
) -> CanEquipItemOutcome {
    let player = player_conditions.player_access_like_cpp();
    let inventory_access = player.owned_inventory_access_like_cpp();
    let valuation_access = player.inventory_valuation_access_like_cpp();
    let modifier_access = player.owned_item_modifiers_access_like_cpp();
    InventoryValuationApplicationCxLikeCpp {
        inventory: player_conditions.inventory,
        player_conditions,
        catalogs: player_conditions.valuation_catalogs,
        inventory_access,
        valuation_access,
        modifier_access,
    }
    .can_equip_inventory_item_like_cpp(
        inventory_item,
        runtime_item,
        requested_slot,
        swap,
        not_loading,
        is_in_combat,
        can_dual_wield,
        can_titan_grip,
    )
}

impl<'a> InventoryValuationApplicationCxLikeCpp<'a> {
    /// Preserve the existing Rust candidate scan and its recursive item-limit
    /// condition projection. C++ stores AvgItemLevel before this condition
    /// loop reads it; this Rust recursion remains an explicit F6 contrast.
    pub fn represented_avg_total_item_level_like_cpp(&self) -> Option<f32> {
        let (can_dual_wield, can_titan_grip) = self
            .inventory
            .inventory_equip_capabilities_with_access_like_cpp(&self.valuation_access)?;
        let mut best_item_levels =
            vec![(InventoryType::NonEquip, 0u32, ObjectGuid::EMPTY); EQUIPMENT_SLOT_END as usize];
        let mut sum = 0u32;

        let item_objects = self
            .inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.inventory_access)?;
        let inventory_items = self
            .inventory
            .resolved_inventory_items_with_access_like_cpp(&self.inventory_access)?;
        for (&slot, inventory_item) in &inventory_items {
            let runtime_item = item_objects.get(&inventory_item.guid);
            self.consume_candidate_like_cpp(
                &mut best_item_levels,
                &mut sum,
                Some(slot),
                inventory_item.entry_id,
                inventory_item.guid,
                runtime_item,
                can_dual_wield,
                can_titan_grip,
            );
        }

        for item in item_objects.values() {
            if item.is_in_trade()
                || item.container_guid().is_empty()
                || !item_objects.contains_key(&item.container_guid())
            {
                continue;
            }

            self.consume_candidate_like_cpp(
                &mut best_item_levels,
                &mut sum,
                None,
                item.object().entry(),
                item.object().guid(),
                Some(item),
                can_dual_wield,
                can_titan_grip,
            );
        }

        if !can_titan_grip
            && best_item_levels[EQUIPMENT_SLOT_MAINHAND as usize].0 == InventoryType::Weapon2Hand
        {
            sum = sum.saturating_add(best_item_levels[EQUIPMENT_SLOT_MAINHAND as usize].1);
        }

        Some(sum as f32 / 16.0)
    }

    fn consume_candidate_like_cpp(
        &self,
        best_item_levels: &mut [(InventoryType, u32, ObjectGuid)],
        sum: &mut u32,
        direct_slot: Option<u8>,
        entry_id: u32,
        item_guid: ObjectGuid,
        runtime_item: Option<&Item>,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) {
        let Some(storage_template) = self.catalogs.item_storage_template_like_cpp(entry_id) else {
            return;
        };
        let Some(item_level) = self.inventory.represented_item_level_with_access_like_cpp(
            &self.valuation_access,
            &self.modifier_access,
            &self.catalogs,
            entry_id,
            runtime_item,
            MIN_ITEM_LEVEL_LIKE_CPP,
            MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_conditions
                .player_access_like_cpp()
                .fixture_player_level_like_cpp(),
        ) else {
            return;
        };
        let inventory_type = storage_template.inventory_type;

        if let Some(slot) = direct_slot.filter(|slot| *slot < EQUIPMENT_SLOT_END) {
            wow_entities::represented_avg_total_item_level_maybe_replace_slot_like_cpp(
                best_item_levels,
                sum,
                slot,
                inventory_type,
                item_level,
                item_guid,
                false,
            );
            return;
        }

        if let Some(runtime_item) = runtime_item {
            let represented_item = InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(inventory_type as u8),
            };
            if self.can_use_item_like_cpp(&represented_item, Some(runtime_item), false)
                != InventoryResult::Ok
            {
                return;
            }
            if self.can_equip_unique_item_like_cpp(entry_id, runtime_item, NULL_SLOT)
                != InventoryResult::Ok
            {
                return;
            }
            if self.can_equip_for_avg_total_like_cpp(
                entry_id,
                runtime_item,
                can_dual_wield,
                can_titan_grip,
            ) != InventoryResult::Ok
            {
                return;
            }
        }

        for (candidate_slot, check_duplicate_guid) in
            wow_entities::represented_total_avg_equipment_slot_candidates_like_cpp(
                inventory_type,
                can_dual_wield,
                can_titan_grip,
            )
        {
            wow_entities::represented_avg_total_item_level_maybe_replace_slot_like_cpp(
                best_item_levels,
                sum,
                candidate_slot,
                inventory_type,
                item_level,
                item_guid,
                check_duplicate_guid,
            );
        }
    }

    fn can_use_item_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
        not_loading: bool,
    ) -> InventoryResult {
        let Some(player) = self
            .inventory
            .direct_inventory_player_snapshot_with_access_like_cpp(
                &self.inventory_access,
                self.catalogs.item_store_like_cpp(),
                self.catalogs.item_stats_store_like_cpp(),
            )
        else {
            return InventoryResult::ItemNotFound;
        };
        let proto = self.catalogs.item_storage_template_like_cpp(item.entry_id);
        let sparse = self
            .catalogs
            .item_stats_store_like_cpp()
            .and_then(|store| store.sparse_template(item.entry_id));
        let search = self
            .catalogs
            .item_search_name_store_like_cpp()
            .and_then(|store| store.get(item.entry_id));

        let flags2 = sparse.map_or(0, |template| template.flags[1]);
        let player_access = self.player_conditions.player_access_like_cpp();
        let player_class_mask = player_access
            .player_class_like_cpp()
            .checked_sub(1)
            .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
            .unwrap_or(0);
        let player_race_mask = player_access
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
                let Some(skill_values) = player_access.resolved_skill_values_like_cpp() else {
                    return InventoryResult::ItemNotFound;
                };
                u32::from(skill_values.get(&skill).copied().unwrap_or(0))
            }
            None => 0,
        };
        let required_spell = search.map_or(0, |entry| entry.required_ability);
        let has_required_spell = required_spell == 0
            || i32::try_from(required_spell).ok().is_some_and(|spell_id| {
                self.player_conditions
                    .known_spells_for_item_use_like_cpp()
                    .contains(&spell_id)
            });
        let base_required_level = search
            .and_then(|entry| u8::try_from(entry.required_level.max(0)).ok())
            .unwrap_or(0);
        let required_reputation_faction = sparse.map_or(0, |template| {
            u32::from(template.required_reputation_faction)
        });
        let required_reputation_rank = sparse
            .and_then(|template| u32::try_from(template.required_reputation_rank.max(0)).ok())
            .unwrap_or(0);
        let Some(player_reputation_rank) =
            self.represented_item_reputation_rank_like_cpp(required_reputation_faction)
        else {
            return InventoryResult::ItemNotFound;
        };
        let item_effect_spell_ids = self
            .catalogs
            .represented_item_effect_spell_ids_like_cpp(item.entry_id);
        let effect0_spell_id = item_effect_spell_ids
            .first()
            .and_then(|(_, spell_id)| u32::try_from(*spell_id).ok());
        let effect1_spell_id = item_effect_spell_ids
            .get(1)
            .and_then(|(_, spell_id)| u32::try_from(*spell_id).ok());
        let has_effect1_spell = effect1_spell_id
            .and_then(|spell_id| i32::try_from(spell_id).ok())
            .is_some_and(|spell_id| {
                self.player_conditions
                    .known_spells_for_item_use_like_cpp()
                    .contains(&spell_id)
            });
        let quality = self
            .catalogs
            .item_template_quality_like_cpp(item.entry_id)
            .unwrap_or(0);

        player.can_use_item_like_cpp(wow_entities::CanUseItemArgs {
            source_item: runtime_item,
            proto: proto.as_ref(),
            not_loading,
            is_alive: true,
            player_level: player_access.player_level_like_cpp(),
            item_required_level: base_required_level,
            source_bop_trade_allowed_for_player: false,
            template_args: wow_entities::CanUseItemTemplateArgs {
                proto: proto.as_ref(),
                skip_required_level_check: false,
                player_level: player_access.player_level_like_cpp(),
                team: team_id_for_race_like_cpp(player_access.player_race_like_cpp()),
                allowable_class_matches,
                allowable_race_matches,
                internal_item: (flags2 & wow_constants::ItemFlags2::InternalItem as u32) != 0,
                faction_horde: (flags2 & wow_constants::ItemFlags2::FactionHorde as u32) != 0,
                faction_alliance: (flags2 & wow_constants::ItemFlags2::FactionAlliance as u32) != 0,
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
            player_class: player_access.player_class_like_cpp(),
            proto_is_heirloom: quality == wow_constants::ItemQuality::Heirloom as i8,
        })
    }

    fn represented_item_reputation_rank_like_cpp(&self, faction_id: u32) -> Option<u32> {
        if faction_id == 0 {
            return Some(0);
        }
        let Some(faction) = self
            .catalogs
            .faction_store_like_cpp()
            .and_then(|store| store.get(faction_id))
        else {
            return Some(0);
        };
        let access = &self.valuation_access;
        let player = self.player_conditions.player_access_like_cpp();
        let race = player.player_race_like_cpp();
        let class = player.player_class_like_cpp();
        access.reputation_rank_for_faction_like_cpp(
            faction,
            race,
            class,
            self.catalogs
                .friendship_rep_reaction_store_like_cpp()
                .map(AsRef::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_conditions.reputation_state,
        )
    }

    fn item_limit_category_template_like_cpp(
        &self,
        limit_category_id: u32,
    ) -> Option<wow_entities::ItemLimitCategoryTemplate> {
        if limit_category_id == 0 {
            return None;
        }
        let entry = self.inventory.item_limit_category_entry_like_cpp(
            limit_category_id,
            self.catalogs
                .item_limit_category_store_like_cpp()
                .map(AsRef::as_ref),
        )?;
        let condition_store = self
            .catalogs
            .item_limit_category_condition_store_like_cpp()
            .map(AsRef::as_ref);
        let player_condition_store = self
            .catalogs
            .player_condition_store_like_cpp()
            .map(AsRef::as_ref);
        let context_holder = if condition_store.is_some() {
            Some(self.player_conditions.project_like_cpp()?)
        } else {
            None
        };
        let context = match context_holder.as_ref() {
            Some(values) => Some(self.player_conditions.condition_context_like_cpp(values)?),
            None => None,
        };
        self.inventory
            .item_limit_category_template_with_context_like_cpp(
                entry,
                condition_store,
                player_condition_store,
                context,
            )
    }

    fn can_equip_unique_item_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: &Item,
        except_slot: u8,
    ) -> InventoryResult {
        let Some(player) = self
            .inventory
            .direct_inventory_player_snapshot_with_access_like_cpp(
                &self.inventory_access,
                self.catalogs.item_store_like_cpp(),
                self.catalogs.item_stats_store_like_cpp(),
            )
        else {
            return InventoryResult::ItemNotFound;
        };
        let Some(proto) = self.catalogs.item_storage_template_like_cpp(entry_id) else {
            return InventoryResult::ItemNotFound;
        };
        let Some(item_objects) = self
            .inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.inventory_access)
        else {
            return InventoryResult::ItemNotFound;
        };
        let Some(inventory_items) = self
            .inventory
            .resolved_inventory_items_with_access_like_cpp(&self.inventory_access)
        else {
            return InventoryResult::ItemNotFound;
        };
        let mut equipped_templates = Vec::new();
        let mut equipped_gems = Vec::new();
        for (&slot, inventory_item) in &inventory_items {
            if slot >= EQUIPMENT_SLOT_END {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let Some(template) = self
                .catalogs
                .item_storage_template_like_cpp(inventory_item.entry_id)
            else {
                continue;
            };
            for gem in &item.data().gems {
                let Ok(gem_entry) = u32::try_from(gem.item_id) else {
                    continue;
                };
                let Some(gem_template) = self.catalogs.item_storage_template_like_cpp(gem_entry)
                else {
                    continue;
                };
                equipped_gems.push(wow_entities::EquippedGemRef::new(
                    slot,
                    gem_entry,
                    gem_template.item_limit_category,
                ));
            }
            equipped_templates.push((slot, item, template));
        }
        let equipped_items_with_templates = equipped_templates
            .iter()
            .map(|(slot, item, template)| {
                wow_entities::ItemStorageRef::new(
                    wow_entities::INVENTORY_SLOT_BAG_0,
                    *slot,
                    *item,
                    Some(template),
                )
            })
            .collect::<Vec<_>>();

        let mut socketed_gem_templates = Vec::new();
        for gem in &runtime_item.data().gems {
            let Ok(gem_entry) = u32::try_from(gem.item_id) else {
                continue;
            };
            let Some(gem_template) = self.catalogs.item_storage_template_like_cpp(gem_entry) else {
                continue;
            };
            let source_limit_category_count = if gem_template.item_limit_category == 0 {
                1
            } else {
                runtime_item
                    .data()
                    .gems
                    .iter()
                    .filter_map(|source_gem| u32::try_from(source_gem.item_id).ok())
                    .filter_map(|source_gem_entry| {
                        self.catalogs
                            .item_storage_template_like_cpp(source_gem_entry)
                    })
                    .filter(|source_gem_template| {
                        source_gem_template.item_limit_category == gem_template.item_limit_category
                    })
                    .count() as u32
            };
            let unique_equippable = gem_template
                .flags
                .contains(wow_constants::ItemFlags::UNIQUE_EQUIPPABLE);
            let limit_category =
                self.item_limit_category_template_like_cpp(gem_template.item_limit_category);
            socketed_gem_templates.push((
                gem_template,
                unique_equippable,
                limit_category,
                source_limit_category_count,
            ));
        }
        let socketed_gems = socketed_gem_templates
            .iter()
            .map(|(template, unique, limit_category, source_count)| {
                wow_entities::SocketedGemUniqueRef::new(
                    Some(template),
                    *unique,
                    limit_category.as_ref(),
                    *source_count,
                )
            })
            .collect::<Vec<_>>();
        let unique_equippable = proto
            .flags
            .contains(wow_constants::ItemFlags::UNIQUE_EQUIPPABLE);
        let limit_category = self.item_limit_category_template_like_cpp(proto.item_limit_category);
        player.can_equip_unique_item_like_cpp(wow_entities::CanEquipUniqueItemArgs {
            source_item: Some(runtime_item),
            proto: Some(&proto),
            except_slot,
            limit_count: 1,
            unique_equippable,
            limit_category: limit_category.as_ref(),
            equipped_items: &equipped_items_with_templates,
            equipped_gems: &equipped_gems,
            socketed_gems: &socketed_gems,
        })
    }

    fn can_equip_for_avg_total_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: &Item,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) -> InventoryResult {
        let inventory_item = InventoryItem {
            guid: runtime_item.object().guid(),
            entry_id,
            db_guid: runtime_item.object().guid().counter() as u64,
            inventory_type: self
                .catalogs
                .item_storage_template_like_cpp(entry_id)
                .map(|template| template.inventory_type as u8)
                .filter(|&inventory_type| inventory_type != InventoryType::NonEquip as u8),
        };
        self.can_equip_inventory_item_like_cpp(
            &inventory_item,
            runtime_item,
            NULL_SLOT,
            true,
            false,
            false,
            can_dual_wield,
            can_titan_grip,
        )
        .result
    }

    fn can_equip_inventory_item_like_cpp(
        &self,
        inventory_item: &InventoryItem,
        runtime_item: &Item,
        requested_slot: u8,
        swap: bool,
        not_loading: bool,
        is_in_combat: bool,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) -> wow_entities::CanEquipItemOutcome {
        let Some(mut player) = self
            .inventory
            .direct_inventory_player_snapshot_with_access_like_cpp(
                &self.inventory_access,
                self.catalogs.item_store_like_cpp(),
                self.catalogs.item_stats_store_like_cpp(),
            )
        else {
            return item_not_found_can_equip_outcome_like_cpp();
        };
        let entry_id = inventory_item.entry_id;
        let Some(proto) = self.catalogs.item_storage_template_like_cpp(entry_id) else {
            return item_not_found_can_equip_outcome_like_cpp();
        };

        player.set_can_dual_wield_like_cpp(can_dual_wield);
        player.set_can_titan_grip_like_cpp(can_titan_grip, 0);

        let Some(item_objects) = self
            .inventory
            .resolved_inventory_item_objects_with_access_like_cpp(&self.inventory_access)
        else {
            return item_not_found_can_equip_outcome_like_cpp();
        };
        let Some(inventory_items) = self
            .inventory
            .resolved_inventory_items_with_access_like_cpp(&self.inventory_access)
        else {
            return item_not_found_can_equip_outcome_like_cpp();
        };
        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let item_entry = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) =
                template_cache.entry(item_entry)
            {
                if let Some(template) = self.catalogs.item_storage_template_like_cpp(item_entry) {
                    entry.insert(template);
                }
            }
        }

        let mut represented_bag_slots_by_guid = HashMap::new();
        for (&slot, inventory_item) in &inventory_items {
            if is_buyback_slot(slot) {
                continue;
            }
            if is_represented_bag_slot(slot) && item_objects.contains_key(&inventory_item.guid) {
                represented_bag_slots_by_guid.insert(inventory_item.guid, slot);
            }
        }

        let mut storage_rows = Vec::new();
        let mut equipped_items = Vec::new();
        for (&slot, inventory_item) in &inventory_items {
            if is_buyback_slot(slot) {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let Some(template) = template_cache.get(&inventory_item.entry_id) else {
                continue;
            };
            storage_rows.push((INVENTORY_SLOT_BAG_0, slot, item, template));
            if slot < EQUIPMENT_SLOT_END {
                equipped_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, slot, item));
            }
        }
        for item in item_objects.values() {
            if item.is_in_trade() {
                continue;
            }
            let container_guid = item.container_guid();
            if container_guid.is_empty() {
                continue;
            }
            let Some(&bag_slot) = represented_bag_slots_by_guid.get(&container_guid) else {
                continue;
            };
            let item_entry = item.object().entry();
            let Some(template) = template_cache.get(&item_entry) else {
                continue;
            };
            storage_rows.push((bag_slot, item.slot(), item, template));
        }
        let stored_items: Vec<_> = storage_rows
            .iter()
            .map(|(bag, slot, item, template)| {
                ItemStorageRef::new(*bag, *slot, *item, Some(template))
            })
            .collect();

        let mainhand_item = self
            .inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &self.inventory_access,
                self.catalogs.item_store_like_cpp(),
                self.catalogs.item_stats_store_like_cpp(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
            );
        let mainhand_template = mainhand_item
            .as_ref()
            .and_then(|item| self.catalogs.item_storage_template_like_cpp(item.entry_id));
        let is_two_hand_used =
            player.is_two_hand_used_template_like_cpp(mainhand_template.as_ref());
        let can_use_result =
            self.can_use_item_like_cpp(inventory_item, Some(runtime_item), not_loading);
        let proto_always_allow_dual_wield = self
            .catalogs
            .item_stats_store_like_cpp()
            .and_then(|store| store.sparse_template(entry_id))
            .is_some_and(|template| {
                (template.flags[2] & ItemFlags3::AlwaysAllowDualWield as u32) != 0
            });
        let limit_category = self.item_limit_category_template_like_cpp(proto.item_limit_category);
        let (is_stunned, is_charmed) = self.valuation_access.stunned_and_charmed_like_cpp();
        let is_in_progress_arena = self.is_in_progress_arena_like_cpp();

        let offhand_item = self
            .inventory
            .get_inventory_item_by_pos_with_access_like_cpp(
                &self.inventory_access,
                self.catalogs.item_store_like_cpp(),
                self.catalogs.item_stats_store_like_cpp(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_OFFHAND,
            );
        let offhand_runtime = offhand_item
            .as_ref()
            .and_then(|item| item_objects.get(&item.guid));
        let offhand_proto = offhand_item
            .as_ref()
            .and_then(|item| template_cache.get(&item.entry_id));
        let source_is_not_empty_bag = offhand_item.as_ref().is_some_and(|item| {
            self.inventory
                .direct_item_contains_items_with_access_like_cpp(&self.inventory_access, item.guid)
        });
        let offhand_can_unequip_result = self.can_unequip_inventory_item_like_cpp(
            offhand_runtime,
            offhand_proto,
            source_is_not_empty_bag,
        );
        let offhand_can_store_result = offhand_item
            .as_ref()
            .and_then(|_| {
                self.inventory
                    .get_inventory_item_by_pos_with_access_like_cpp(
                        &self.inventory_access,
                        self.catalogs.item_store_like_cpp(),
                        self.catalogs.item_stats_store_like_cpp(),
                        INVENTORY_SLOT_BAG_0,
                        EQUIPMENT_SLOT_OFFHAND,
                    )
            })
            .and_then(|item| {
                self.inventory
                    .resolved_player_inventory_item_object_with_access_like_cpp(
                        &self.inventory_access,
                        item.guid,
                    )
                    .and_then(|source_item| {
                        self.player_conditions
                            .plan_store_direct_inventory_item_like_cpp(
                                self.valuation_access.realm_id_like_cpp(),
                                item.entry_id,
                                source_item.count(),
                                NULL_BAG,
                                NULL_SLOT,
                                Some(&source_item),
                                false,
                                &[],
                                &[],
                            )
                    })
            })
            .map_or(InventoryResult::Ok, |(result, _, _)| result);

        let make_args = |can_equip_unique_result| CanEquipItemArgs {
            slot: requested_slot,
            proto: Some(&proto),
            source_item: Some(runtime_item),
            source_bop_trade_allowed_for_player: false,
            swap,
            not_loading,
            is_stunned,
            is_charmed,
            is_in_combat,
            is_in_progress_arena,
            weapon_change_timer_active: false,
            current_generic_spell_allows_equip: None,
            current_channeled_spell_allows_equip: None,
            heirloom_required_level_failed: false,
            can_use_result,
            can_equip_unique_result,
            can_dual_wield,
            can_titan_grip,
            is_two_hand_used,
            proto_always_allow_dual_wield,
            has_required_profession_skill: false,
            profession_slot: None,
            offhand_can_unequip_result,
            offhand_can_store_result,
            limit_category: limit_category.as_ref(),
            equipped_items: &equipped_items,
            stored_items: &stored_items,
        };

        let initial = player.can_equip_item_like_cpp(make_args(InventoryResult::Ok));
        if initial.result != InventoryResult::Ok {
            return initial;
        }
        let unique_result = self.can_equip_unique_item_like_cpp(
            entry_id,
            runtime_item,
            initial.unique_ignore_slot.unwrap_or(NULL_SLOT),
        );
        player.can_equip_item_like_cpp(make_args(unique_result))
    }

    fn can_unequip_inventory_item_like_cpp(
        &self,
        source_item: Option<&Item>,
        proto: Option<&wow_entities::ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        self.inventory
            .can_unequip_inventory_item_at_with_access_like_cpp(
                &self.valuation_access,
                &self.inventory_access,
                &self.catalogs,
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_OFFHAND,
                false,
                source_item,
                proto,
                source_is_not_empty_bag,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_conditions.battleground_fixture,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_conditions.in_combat,
            )
    }

    fn is_in_progress_arena_like_cpp(&self) -> bool {
        let in_battleground = self
            .valuation_access
            .battleground_state_snapshot_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_conditions.battleground_fixture,
            )
            .is_some_and(|state| state.battleground_status_like_cpp() == Some(3));
        in_battleground
            && self
                .catalogs
                .map_store_like_cpp()
                .and_then(|store| {
                    store.get(u32::from(self.valuation_access.player_map_id_like_cpp()))
                })
                .is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA)
    }
}

fn item_not_found_can_equip_outcome_like_cpp() -> CanEquipItemOutcome {
    CanEquipItemOutcome {
        result: InventoryResult::ItemNotFound,
        dest: 0,
        unique_ignore_slot: None,
    }
}

fn team_id_for_race_like_cpp(race: u8) -> u32 {
    match wow_world_core::session::player_team_for_race_cpp(race) {
        wow_constants::Team::Horde => 1,
        _ => 0,
    }
}
