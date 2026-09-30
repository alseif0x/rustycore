//! item sets for the existing items owner.

use super::*;

impl WorldSession {
    pub(crate) fn record_represented_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> bool {
        !self
            .record_represented_items_set_item_events_like_cpp(item_guid, apply)
            .is_empty()
    }
    pub(in crate::session) fn record_represented_items_set_item_events_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return Vec::new();
        };
        let Some(item_set) = self.item_set_for_item_id_like_cpp(item_entry).cloned() else {
            return Vec::new();
        };

        let events = if apply {
            self.record_represented_add_items_set_item_like_cpp(item_guid, &item_set)
        } else {
            self.record_represented_remove_items_set_item_like_cpp(item_guid, &item_set)
        };
        #[cfg(test)]
        self.player_item_test_fixture_like_cpp
            .represented_item_set_spell_events_like_cpp
            .extend(events.iter().copied());
        events
    }
    fn record_represented_add_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        item_set: &wow_data::ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        if item_set.required_skill != 0 {
            let Some(skill_value) =
                self.resolved_player_skill_value_like_cpp(item_set.required_skill as u16)
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
        if self.represented_heirloom_item_set_bonus_over_level_cap_like_cpp(item_guid) {
            return Vec::new();
        }

        let mut events = Vec::new();
        let Some(equipped_count_after) =
            self.add_player_item_set_item_like_cpp(item_set.id, item_guid)
        else {
            return Vec::new();
        };

        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let spells: Vec<_> = self
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) > equipped_count_after {
                continue;
            }
            if !self.represented_item_set_spell_exists_like_cpp(item_set_spell.spell_id) {
                continue;
            }
            let inserted = self
                .add_player_item_set_bonus_like_cpp(item_set.id, item_set_spell.id)
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
    fn record_represented_remove_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        item_set: &wow_data::ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(equipped_count_after) = self
            .remove_player_item_set_item_like_cpp(item_set.id, item_guid)
            .flatten()
        else {
            return Vec::new();
        };
        let mut events = Vec::new();

        let spells: Vec<_> = self
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) <= equipped_count_after {
                continue;
            }
            let removed = self
                .remove_player_item_set_bonus_like_cpp(item_set.id, item_set_spell.id)
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

        let _ = self.drop_player_empty_item_set_effect_like_cpp(item_set.id);

        events
    }
    pub(in crate::session) fn record_represented_offhand_item_mod_remove_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> bool {
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_OFFHAND, false) != 0
    }
}
