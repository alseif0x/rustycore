//! requirements for the existing enchantment owner.

use super::*;

impl WorldSession {
    /// C++ `Player::EnchantmentFitsRequirements` for the currently equipped gems.
    pub(super) fn enchantment_fits_requirements_like_cpp(
        &self,
        enchantment_condition: u32,
        except_slot: Option<u8>,
    ) -> bool {
        if enchantment_condition == 0 {
            return true;
        }
        let Some(condition) = self
            .spell_catalogs
            .spell_item_enchantment_condition_store
            .as_ref()
            .and_then(|store| store.get(enchantment_condition))
        else {
            return true;
        };

        let mut gem_counts = [0u8; 4];
        for slot in 0..EQUIPMENT_SLOT_END {
            if except_slot == Some(slot) {
                continue;
            }
            let Some(inventory_item) = self.resolved_inventory_item_like_cpp(slot) else {
                continue;
            };
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
            else {
                continue;
            };
            if item.is_broken() {
                continue;
            }
            for gem in &item.data().gems {
                let Ok(gem_item_id) = u32::try_from(gem.item_id) else {
                    continue;
                };
                let Some(gem_properties_id) = self
                    .items
                    .stats_store
                    .as_ref()
                    .and_then(|store| store.gem_properties(gem_item_id))
                    .map(u32::from)
                else {
                    continue;
                };
                let Some(gem_type) = self
                    .gem_properties_store
                    .as_ref()
                    .and_then(|store| store.get(gem_properties_id))
                    .map(|properties| properties.gem_type)
                else {
                    continue;
                };
                for (color, count) in gem_counts.iter_mut().enumerate() {
                    if gem_type & (1 << color) != 0 {
                        *count = count.saturating_add(1);
                    }
                }
            }
        }

        let mut activate = true;
        for index in 0..5 {
            let left_type = condition.lt_operand_type[index];
            if left_type == 0 {
                continue;
            }
            let Some(&left_count) = gem_counts.get(usize::from(left_type - 1)) else {
                return false;
            };
            let right_type = condition.rt_operand_type[index];
            let right_count = if right_type == 0 {
                condition.rt_operand[index]
            } else {
                let Some(&count) = gem_counts.get(usize::from(right_type - 1)) else {
                    return false;
                };
                count
            };
            activate &= match condition.operator[index] {
                2 => left_count < right_count,
                3 => left_count > right_count,
                5 => left_count >= right_count,
                _ => true,
            };
        }
        activate
    }
    pub(super) fn current_item_enchantment_socket_context_like_cpp(
        &self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    ) -> Option<ApplyEnchantmentSocketContext> {
        let socket_index = match slot {
            EnchantmentSlot::EnhancementSocket => 0,
            EnchantmentSlot::EnhancementSocket2 => 1,
            EnchantmentSlot::EnhancementSocket3 => 2,
            _ => return None,
        };
        let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        let socket_color = self
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.socket_template(item.object().entry()))
            .map(|template| u32::from(template.socket_types[socket_index]))
            .unwrap_or(0);
        let gem_requirement = item
            .data()
            .gems
            .get(socket_index)
            .and_then(|gem| u32::try_from(gem.item_id).ok())
            .and_then(|gem_item_id| {
                self.items
                    .stats_store
                    .as_ref()?
                    .socket_template(gem_item_id)
            })
            .and_then(|gem_template| {
                Some(ApplyEnchantmentGemRequirementRef::new(
                    u32::from(gem_template.required_skill_id),
                    gem_template.required_skill_rank,
                    self.resolved_player_skill_value_like_cpp(gem_template.required_skill_id)?,
                ))
            });

        if socket_color != 0 {
            return Some(ApplyEnchantmentSocketContext::colored(
                socket_color,
                gem_requirement,
            ));
        }

        let prismatic_enchantment_id =
            item.data().enchantments[EnchantmentSlot::EnhancementSocketPrismatic as usize].id;
        let prismatic_enchantment = self
            .apply_enchantment_template_ref(prismatic_enchantment_id, 0, true)
            .and_then(|mut template| {
                if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                    template.required_skill_value =
                        self.resolved_player_skill_value_like_cpp(skill_id)?;
                }
                Some(template)
            });
        Some(ApplyEnchantmentSocketContext::prismatic(
            prismatic_enchantment,
            gem_requirement,
        ))
    }
    pub(crate) fn resolved_enchanting_skill_like_cpp(&self) -> Option<u16> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.enchanting_skill_value_like_cpp(SKILL_ENCHANTING_LIKE_CPP)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_enchanting_skill);
        }
        canonical
    }
}
