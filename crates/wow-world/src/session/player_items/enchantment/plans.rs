//! plans for the existing enchantment owner.

use super::*;

impl WorldSession {
    /// C++ `Player::ApplyEnchantment(item, slot, apply, ...)` bridge for a
    /// represented inventory item owned by the current player.
    ///
    /// The item runtime lives in the session inventory while the player state
    /// lives in the canonical map. This temporarily moves the item out, runs the
    /// entity-level plan against the canonical player when available, then puts
    /// the item back without clearing or setting the enchantment field itself.
    pub(crate) fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        mut args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        let enchantment_id = self
            .resolved_inventory_item_object_like_cpp(item_guid)?
            .data()
            .enchantments[slot as usize]
            .id;
        let condition_fits = u32::try_from(enchantment_id)
            .ok()
            .and_then(|id| {
                self.spell_catalogs
                    .spell_item_enchantment_store
                    .as_ref()?
                    .get(id)
            })
            .is_none_or(|entry| {
                self.enchantment_fits_requirements_like_cpp(u32::from(entry.condition_id), None)
            });
        if args.socket_context.is_none() {
            args.socket_context =
                self.current_item_enchantment_socket_context_like_cpp(item_guid, slot);
        }
        let mut item = self.remove_inventory_item_object(item_guid)?;
        let mut template = self.apply_enchantment_template_ref(enchantment_id, 0, condition_fits);
        if let Some(template) = &mut template {
            if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                template.required_skill_value =
                    self.resolved_player_skill_value_like_cpp(skill_id)?;
            }
        }

        let plan = self.mutate_canonical_player_like_cpp(|player| {
            player.apply_enchantment_plan(Some(&mut item), slot, template, args)
        });
        self.insert_inventory_item_object(item);
        plan
    }
}
