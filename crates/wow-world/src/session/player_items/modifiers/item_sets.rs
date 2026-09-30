//! item sets for the existing modifiers owner.

use super::*;

impl WorldSession {

    pub(in crate::session) fn add_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_item_like_cpp(item_set_id, item_guid)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .add_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub(in crate::session) fn add_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .add_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(in crate::session) fn remove_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<Option<usize>> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_item_like_cpp(item_set_id, item_guid)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .remove_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub(in crate::session) fn remove_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(in crate::session) fn drop_player_empty_item_set_effect_like_cpp(
        &mut self,
        item_set_id: u32,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.drop_empty_item_set_effect_like_cpp(item_set_id)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .drop_empty_item_set_effect_like_cpp(item_set_id),
            );
        }
        None
    }

    pub(in crate::session) fn set_player_item_level_caps_like_cpp(
        &mut self,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.set_item_level_caps_like_cpp(caps);
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_item_test_fixture_like_cpp
                .represented_item_modifier_runtime_like_cpp
                .set_item_level_caps_like_cpp(caps);
            return true;
        }
        false
    }
    pub(in crate::session) fn represented_item_set_spell_exists_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_catalogs
            .spell_store
            .as_ref()
            .is_none_or(|store| store.get(spell_id).is_some())
    }
    pub(in crate::session) fn represented_heirloom_item_set_bonus_over_level_cap_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return false;
        };
        if !self
            .heirloom_store
            .as_ref()
            .is_some_and(|store| store.get_by_item_id_like_cpp(item_entry).is_some())
        {
            return false;
        }

        let Some(template) = self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_entry))
        else {
            return false;
        };
        let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
        if curve_id == 0 {
            return false;
        }

        let Some((curve_store, curve_point_store)) = self
            .curve_store
            .as_ref()
            .zip(self.curve_point_store.as_ref())
        else {
            return false;
        };
        let Some((_min_level, max_level)) =
            curve_store.curve_x_axis_range_like_cpp(curve_point_store, curve_id)
        else {
            return false;
        };
        if !max_level.is_finite() || max_level < 0.0 {
            return false;
        }
        let mut max_level = max_level as u32;

        if let Some(content_tuning) = self.content_tuning_store.as_ref().and_then(|store| {
            store
                .content_tuning_data_like_cpp(template.scaling_stat_content_tuning_like_cpp(), true)
        }) {
            max_level = max_level.min(u32::try_from(content_tuning.max_level).unwrap_or(0));
        }

        u32::from(self.player_level_like_cpp()) > max_level
    }
    pub(crate) fn record_represented_update_item_set_auras_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        let events = self.plan_represented_update_item_set_auras_like_cpp(form_change);
        #[cfg(test)]
        self.player_item_test_fixture_like_cpp
            .represented_item_set_aura_refresh_events_like_cpp
            .extend(events.iter().cloned());
        events.len()
    }
    pub(super) fn plan_represented_update_item_set_auras_like_cpp(
        &self,
        form_change: bool,
    ) -> Vec<RepresentedItemSetAuraRefreshEventLikeCpp> {
        let mut events = Vec::new();
        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let Some(active_effects) =
            self.player_item_modifier_runtime_snapshot_like_cpp()
                .map(|state| {
                    state
                        .item_set_effects_like_cpp()
                        .values()
                        .cloned()
                        .collect::<Vec<_>>()
                })
        else {
            return events;
        };

        for effect in active_effects {
            let active_bonus_ids = effect.set_bonuses.clone();
            let spells: Vec<_> = self
                .item_set_spells_like_cpp(effect.item_set_id)
                .into_iter()
                .filter(|spell| active_bonus_ids.contains(&spell.id))
                .cloned()
                .collect();

            for item_set_spell in spells {
                if item_set_spell.chr_spec_id != 0
                    && Some(u32::from(item_set_spell.chr_spec_id)) != primary_spec
                {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: false,
                        form_change: false,
                    });
                    continue;
                }

                let fits_shapeshift =
                    self.represented_equip_spell_fits_shapeshift_like_cpp(item_set_spell.spell_id);
                if !form_change || !fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: false,
                        form_change,
                    });
                }
                if fits_shapeshift {
                    events.push(RepresentedItemSetAuraRefreshEventLikeCpp {
                        item_set_id: effect.item_set_id,
                        spell_entry_id: item_set_spell.id,
                        spell_id: item_set_spell.spell_id,
                        apply: true,
                        form_change,
                    });
                }
            }
        }

        events
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_effect_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Option<RepresentedItemSetEffectLikeCpp> {
        self.player_item_modifier_runtime_snapshot_like_cpp()?
            .item_set_effect_like_cpp(item_set_id)
            .cloned()
    }
}
