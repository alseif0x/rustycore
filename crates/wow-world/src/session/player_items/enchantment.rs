//! Represented item enchantment state and its loaded effects.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// Set the item random enchantment template store for this session.
    pub fn set_item_random_enchantment_template_store(
        &mut self,
        store: Arc<ItemRandomEnchantmentTemplateStore>,
    ) {
        self.catalogs.items.random_enchantment_template_store = Some(store);
    }
    pub fn item_random_enchantment_template_store(
        &self,
    ) -> Option<&Arc<ItemRandomEnchantmentTemplateStore>> {
        self.catalogs.item_random_enchantment_template_store()
    }
    /// Get the item disenchant loot store reference.
    #[cfg(test)]
    pub fn item_disenchant_loot_store(&self) -> Option<&Arc<ItemDisenchantLootStore>> {
        self.catalogs.item_disenchant_loot_store.as_ref()
    }
    pub fn apply_enchantment_random_suffix_ref(
        &self,
        random_properties_id: i32,
    ) -> Option<ApplyEnchantmentRandomSuffixRef> {
        self.catalogs
            .apply_enchantment_random_suffix_ref(random_properties_id)
    }
    /// Set the spell item enchantment store for this session.
    pub fn set_spell_item_enchantment_store(&mut self, store: Arc<SpellItemEnchantmentStore>) {
        self.catalogs.spell_catalogs.spell_item_enchantment_store = Some(store);
    }
    pub fn set_spell_item_enchantment_condition_store(
        &mut self,
        store: Arc<SpellItemEnchantmentConditionStore>,
    ) {
        self.catalogs
            .spell_catalogs
            .spell_item_enchantment_condition_store = Some(store);
    }
    pub fn spell_item_enchantment_store(&self) -> Option<&Arc<SpellItemEnchantmentStore>> {
        self.catalogs.spell_item_enchantment_store()
    }
    pub fn is_arena_allowed_enchantment(&self, enchantment_id: u32) -> bool {
        self.catalogs.is_arena_allowed_enchantment(enchantment_id)
    }
    pub fn apply_enchantment_template_ref(
        &self,
        enchantment_id: i32,
        required_skill_value: u16,
        condition_fits: bool,
    ) -> Option<ApplyEnchantmentTemplateRef> {
        self.catalogs.apply_enchantment_template_ref(
            enchantment_id,
            required_skill_value,
            condition_fits,
        )
    }
    pub fn apply_enchantment_effect_refs(
        &self,
        enchantment_id: u32,
    ) -> Option<[ApplyEnchantmentEffectRef; 3]> {
        self.catalogs.apply_enchantment_effect_refs(enchantment_id)
    }
    pub(crate) fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        self.build_item_enchantment_handler_cx_like_cpp()
            .apply_current_player_item_enchantment_plan_like_cpp(item_guid, slot, args)
    }
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
                let Some(visible_auras) =
                    crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
                else {
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
    pub fn send_item_enchant_time_update_plans(
        &self,
        owner_guid: ObjectGuid,
        updates: &[PlayerEnchantTimeUpdate],
    ) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_item_enchant_time_update_plans(hub, owner_guid, updates)
    }
    pub(crate) fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.refresh_inventory_item_enchantment_duration_refs_like_cpp(&mut hub, item_guid)
    }
    pub(crate) fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.inventory_remove_enchantment_persistence_like_cpp(hub, item_guid, clear_mainhand_only)
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
    pub(crate) fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_loaded_equipped_item_enchantment_updates_like_cpp(hub, outcome)
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
            .core
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
            self.inventory
                .record_represented_item_bonus_actions_for_test_like_cpp(
                    std::slice::from_ref(&represented_action),
                );
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



#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/enchantment/f3_shims.rs"]
mod f3_shims;
