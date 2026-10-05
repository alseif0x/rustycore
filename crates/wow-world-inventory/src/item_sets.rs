// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_core::ObjectGuid;
use wow_data::ItemSetEntry;
use wow_world_core::session::{
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp, OwnedItemSetAccessLikeCpp,
};

use crate::RepresentedItemSetSpellEventLikeCpp;

/// C++ `ITEM_SET_FLAG_LEGACY_INACTIVE`.
pub const ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP: u32 = 0x01;

impl crate::InventoryState {
    /// Apply or remove one item's contribution to its selected ItemSet.
    pub fn record_represented_items_set_item_like_cpp(
        &mut self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        item_modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        item_sets: &OwnedItemSetAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        apply: bool,
        consumer_test: bool,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(item_entry) = self
            .resolved_inventory_item_object_with_access_like_cpp(inventory_access, item_guid)
            .map(|item| item.object().entry())
        else {
            return Vec::new();
        };
        let Some(item_set) = item_sets.item_set_for_item_id_like_cpp(item_entry).cloned() else {
            return Vec::new();
        };

        let events = if apply {
            self.record_represented_add_items_set_item_like_cpp(
                inventory_access,
                item_modifiers,
                item_sets,
                item_guid,
                &item_set,
                consumer_test,
            )
        } else {
            self.record_represented_remove_items_set_item_with_access_like_cpp(
                item_modifiers,
                item_sets,
                item_guid,
                &item_set,
            )
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test {
            self.record_represented_item_set_spell_events_for_test_like_cpp(&events);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;

        events
    }

    pub(crate) fn represented_heirloom_item_set_bonus_over_level_cap_with_access_like_cpp(
        &self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        item_sets: &OwnedItemSetAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item_entry) = self
            .resolved_inventory_item_object_with_access_like_cpp(inventory_access, item_guid)
            .map(|item| item.object().entry())
        else {
            return false;
        };
        if !item_sets
            .heirloom_store_like_cpp()
            .is_some_and(|store| store.get_by_item_id_like_cpp(item_entry).is_some())
        {
            return false;
        }

        let Some(template) = item_sets
            .item_stats_store_like_cpp()
            .and_then(|store| store.sparse_template(item_entry))
        else {
            return false;
        };
        let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
        if curve_id == 0 {
            return false;
        }

        let Some((_min_level, max_level)) = item_sets
            .curve_store_like_cpp()
            .zip(item_sets.curve_point_store_like_cpp())
            .and_then(|(curve_store, curve_point_store)| {
                curve_store.curve_x_axis_range_like_cpp(curve_point_store, curve_id)
            })
        else {
            return false;
        };
        if !max_level.is_finite() || max_level < 0.0 {
            return false;
        }
        let mut max_level = max_level as u32;

        if let Some(content_tuning) = item_sets.content_tuning_store_like_cpp().and_then(|store| {
            store
                .content_tuning_data_like_cpp(template.scaling_stat_content_tuning_like_cpp(), true)
        }) {
            max_level = max_level.min(u32::try_from(content_tuning.max_level).unwrap_or(0));
        }

        u32::from(item_sets.player_level_like_cpp()) > max_level
    }

    fn record_represented_add_items_set_item_like_cpp(
        &mut self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        item_modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        item_sets: &OwnedItemSetAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        item_set: &ItemSetEntry,
        consumer_test: bool,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        if item_set.required_skill != 0 {
            let Some(skill_value) =
                item_sets.resolved_player_skill_value_like_cpp(item_set.required_skill as u16)
            else {
                return Vec::new();
            };
            if skill_value < item_set.required_skill_rank {
                return Vec::new();
            }
        }
        if item_set.set_flags & ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP != 0 {
            return Vec::new();
        }
        if self.represented_heirloom_item_set_bonus_over_level_cap_with_access_like_cpp(
            inventory_access,
            item_sets,
            item_guid,
        ) {
            return Vec::new();
        }

        let mut events = Vec::new();
        let Some(equipped_count_after) = self.add_player_item_set_item_with_access_like_cpp(
            item_modifiers,
            item_set.id,
            item_guid,
        ) else {
            return Vec::new();
        };

        let primary_spec = item_sets.primary_specialization_id_like_cpp(consumer_test);
        let spells: Vec<_> = item_sets
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) > equipped_count_after {
                continue;
            }
            if !item_sets.represented_item_set_spell_exists_like_cpp(item_set_spell.spell_id) {
                continue;
            }
            let inserted = self
                .add_player_item_set_bonus_with_access_like_cpp(
                    item_modifiers,
                    item_set.id,
                    item_set_spell.id,
                )
                .unwrap_or(false);
            if !inserted {
                continue;
            }
            if item_set_spell.chr_spec_id != 0
                && Some(u32::from(item_set_spell.chr_spec_id)) != primary_spec
            {
                continue;
            }
            events.push(RepresentedItemSetSpellEventLikeCpp {
                item_set_id: item_set.id,
                spell_entry_id: item_set_spell.id,
                spell_id: item_set_spell.spell_id,
                threshold: item_set_spell.threshold,
                apply: true,
            });
        }

        events
    }

    pub(crate) fn record_represented_remove_items_set_item_with_access_like_cpp(
        &mut self,
        item_modifiers: &OwnedItemModifiersAccessLikeCpp<'_>,
        item_sets: &OwnedItemSetAccessLikeCpp<'_>,
        item_guid: ObjectGuid,
        item_set: &ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(equipped_count_after) = self
            .remove_player_item_set_item_with_access_like_cpp(
                item_modifiers,
                item_set.id,
                item_guid,
            )
            .flatten()
        else {
            return Vec::new();
        };
        let mut events = Vec::new();

        let spells: Vec<_> = item_sets
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) <= equipped_count_after {
                continue;
            }
            let removed = self
                .remove_player_item_set_bonus_with_access_like_cpp(
                    item_modifiers,
                    item_set.id,
                    item_set_spell.id,
                )
                .unwrap_or(false);
            if !removed {
                continue;
            }
            events.push(RepresentedItemSetSpellEventLikeCpp {
                item_set_id: item_set.id,
                spell_entry_id: item_set_spell.id,
                spell_id: item_set_spell.spell_id,
                threshold: item_set_spell.threshold,
                apply: false,
            });
        }

        let _ = self
            .drop_player_empty_item_set_effect_with_access_like_cpp(item_modifiers, item_set.id);

        events
    }
}
