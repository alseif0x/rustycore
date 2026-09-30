//! runtime effects for the existing enchantment owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn apply_loaded_enchantment_spell_action_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        match action {
            ApplyEnchantmentEffectAction::CastEquipSpell {
                spell_id,
                item_guid,
            } => {
                let Ok(spell_id) = i32::try_from(spell_id) else {
                    return false;
                };
                let Some(spell_info) = self
                    .spell_store()
                    .and_then(|store| store.get(spell_id))
                    .cloned()
                else {
                    return false;
                };
                let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
                if effect_mask == 0 {
                    return false;
                }
                self.apply_aura_with_effect_mask_without_update_like_cpp(
                    spell_id,
                    item_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            }
            ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
                spell_id,
                item_guid,
            } => {
                let Ok(spell_id) = i32::try_from(spell_id) else {
                    return false;
                };
                let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
                    return false;
                };
                let slots = visible_auras
                    .values()
                    .filter_map(|aura| {
                        (aura.spell_id == spell_id && aura.caster_guid == item_guid)
                            .then_some(aura.slot)
                    })
                    .collect::<Vec<_>>();
                let removed = !slots.is_empty();
                for slot in slots {
                    let _ = self.remove_aura(slot);
                }
                removed
            }
            _ => false,
        }
    }
    /// C++ `Player::_LoadInventory` finishes by `_ApplyAllItemMods`, which in
    /// turn calls `ApplyEnchantment(m_items[i], true)` for equipped items.
    pub(crate) fn apply_loaded_equipped_item_enchantments_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        };
        // `_ApplyAllItemMods` visits the broad top-level range, but C++
        // `ApplyEnchantment` immediately returns unless `Item::IsEquipped`.
        if !item.is_equipped() {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        }
        if !self.initial_loaded_item_mods_can_apply_like_cpp(item_guid) {
            return LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        }
        let slots = item
            .data()
            .enchantments
            .iter()
            .enumerate()
            .filter_map(|(slot_index, enchantment)| {
                if enchantment.id == 0 {
                    return None;
                }
                <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(slot_index)
            })
            .collect::<Vec<_>>();

        let mut outcome = LoadedEquippedItemEnchantmentsOutcomeLikeCpp::default();
        for slot in slots {
            if let Some(plan) = self.apply_current_player_item_enchantment_plan_like_cpp(
                item_guid,
                slot,
                ApplyEnchantmentArgs::apply(),
            ) {
                if let ApplyEnchantmentResult::Applied {
                    enchantment_id,
                    apply,
                    effects_allowed,
                    update_permanent_visible_item,
                    duration_action,
                    ..
                } = plan.result
                {
                    if let Some(ApplyEnchantmentDurationAction::Added(duration_update)) =
                        duration_action
                    {
                        outcome.duration_updates.push(duration_update);
                    }
                    if effects_allowed {
                        let (changed_stats, mut effect_actions, mut unrepresented_effect_actions) =
                            self.apply_loaded_equipped_item_enchantment_effects_like_cpp(
                                item_guid,
                                slot,
                                enchantment_id,
                                apply,
                            );
                        outcome.send_stat_update |= changed_stats;
                        outcome.effect_actions.append(&mut effect_actions);
                        outcome
                            .unrepresented_effect_actions
                            .append(&mut unrepresented_effect_actions);
                    }
                    if update_permanent_visible_item
                        && let Some(visible_item_update) =
                            self.loaded_inventory_item_visible_update_like_cpp(item_guid)
                    {
                        outcome.visible_item_changes.push(visible_item_update);
                    }
                }
                outcome.plans.push(plan);
            }
        }
        outcome
    }
    fn apply_loaded_equipped_item_enchantment_effects_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        enchantment_id: i32,
        apply: bool,
    ) -> (
        bool,
        Vec<RepresentedItemBonusActionLikeCpp>,
        Vec<RepresentedItemBonusActionLikeCpp>,
    ) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return (false, Vec::new(), Vec::new());
        };
        let Some(effects) = u32::try_from(enchantment_id)
            .ok()
            .and_then(|enchantment_id| self.apply_enchantment_effect_refs(enchantment_id))
        else {
            return (false, Vec::new(), Vec::new());
        };
        let item_template = self.item_storage_template(item.object().entry());
        let random_suffix =
            self.apply_enchantment_random_suffix_ref(item.data().random_properties_id);
        let actions = self
            .mutate_canonical_player_like_cpp(|player| {
                player.apply_enchantment_effect_actions_for_enchantment(
                    &item,
                    item_template.as_ref(),
                    slot,
                    enchantment_id,
                    random_suffix,
                    apply,
                    &effects,
                )
            })
            .unwrap_or_default();

        let mut changed_stats = false;
        let mut represented_actions = Vec::new();
        let mut unrepresented_actions = Vec::new();
        for action in actions {
            if matches!(action, ApplyEnchantmentEffectAction::Noop) {
                continue;
            }
            changed_stats |=
                wow_entities::represented_item_bonus_action_updates_stats_like_cpp(action);
            let represented_action = RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: slot as u8,
                action,
            };
            #[cfg(test)]
            self.player_item_test_fixture_like_cpp
                .represented_item_bonus_actions_like_cpp
                .push(represented_action.clone());
            let spell_action_applied = self.apply_loaded_enchantment_spell_action_like_cpp(action);
            if !spell_action_applied
                && wow_entities::loaded_enchantment_effect_action_is_unrepresented_like_cpp(action)
            {
                unrepresented_actions.push(represented_action.clone());
            }
            represented_actions.push(represented_action);
            self.apply_represented_item_bonus_action_state_like_cpp(action);
        }
        (changed_stats, represented_actions, unrepresented_actions)
    }
}
