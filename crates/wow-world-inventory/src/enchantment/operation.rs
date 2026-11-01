// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::item::EnchantmentSlot;
use wow_core::ObjectGuid;
use wow_entities::{
    ApplyEnchantmentArgs, ApplyEnchantmentGemRequirementRef, ApplyEnchantmentPlan,
    ApplyEnchantmentSocketContext, EQUIPMENT_SLOT_END,
};
use wow_world_core::session::{OwnedInventoryAccessLikeCpp, OwnedItemEnchantmentAccessLikeCpp};

/// Inert selected catalogs for the full condition, socket and apply operation.
pub struct ItemEnchantmentCatalogsLikeCpp<'a> {
    enchantments: Option<&'a wow_data::SpellItemEnchantmentStore>,
    conditions: Option<&'a wow_data::SpellItemEnchantmentConditionStore>,
    item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
    item_stats: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
    gems: Option<&'a wow_data::GemPropertiesStore>,
}

impl<'a> ItemEnchantmentCatalogsLikeCpp<'a> {
    pub fn new(
        enchantments: Option<&'a wow_data::SpellItemEnchantmentStore>,
        conditions: Option<&'a wow_data::SpellItemEnchantmentConditionStore>,
        item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
        item_stats: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
        gems: Option<&'a wow_data::GemPropertiesStore>,
    ) -> Self {
        Self {
            enchantments,
            conditions,
            item_store,
            item_stats,
            gems,
        }
    }

    fn template(
        &self,
        id: i32,
        condition_fits: bool,
    ) -> Option<wow_entities::ApplyEnchantmentTemplateRef> {
        wow_world_core::session::apply_enchantment_template_from_store_like_cpp(
            self.enchantments,
            id,
            0,
            condition_fits,
        )
    }
}

pub struct ItemEnchantmentApplicationCxLikeCpp<'a> {
    state: &'a mut crate::InventoryState,
    inventory: OwnedInventoryAccessLikeCpp<'a>,
    owner: OwnedItemEnchantmentAccessLikeCpp<'a>,
    catalogs: ItemEnchantmentCatalogsLikeCpp<'a>,
}

impl<'a> ItemEnchantmentApplicationCxLikeCpp<'a> {
    pub fn new(
        state: &'a mut crate::InventoryState,
        inventory: OwnedInventoryAccessLikeCpp<'a>,
        owner: OwnedItemEnchantmentAccessLikeCpp<'a>,
        catalogs: ItemEnchantmentCatalogsLikeCpp<'a>,
    ) -> Self {
        Self {
            state,
            inventory,
            owner,
            catalogs,
        }
    }

    /// Target ItemHandler.cpp:1100–1116. Preserve the checked Rust slot
    /// conversion and ignore both mutation results at their original points.
    pub fn cancel_temp_enchantment_like_cpp(
        &mut self,
        cancel: wow_packet::packets::item::CancelTempEnchantment,
    ) {
        let Ok(slot) = u8::try_from(cancel.slot) else {
            return;
        };
        if !wow_entities::is_equipment_pos(wow_entities::INVENTORY_SLOT_BAG_0, slot) {
            return;
        }
        let Some(item) = self.state.get_inventory_item_by_pos_with_access_like_cpp(
            &self.inventory,
            self.catalogs.item_store,
            self.catalogs.item_stats,
            wow_entities::INVENTORY_SLOT_BAG_0,
            slot,
        ) else {
            return;
        };
        let Some(runtime_item) = self
            .state
            .resolved_player_inventory_item_object_with_access_like_cpp(&self.inventory, item.guid)
        else {
            return;
        };
        if runtime_item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id == 0
        {
            return;
        }
        let _ = self.apply_current_player_item_enchantment_plan_like_cpp(
            item.guid,
            EnchantmentSlot::EnhancementTemporary,
            ApplyEnchantmentArgs::remove(),
        );
        let _ = self
            .state
            .apply_quest_reward_item_object_updates_with_access_like_cpp(
                &self.inventory,
                item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::ClearEnchantment(
                    EnchantmentSlot::EnhancementTemporary,
                )],
            );
    }

    pub fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        mut args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        let enchantment_id = self
            .state
            .resolved_player_inventory_item_object_with_access_like_cpp(&self.inventory, item_guid)?
            .data()
            .enchantments[slot as usize]
            .id;
        let condition_fits = u32::try_from(enchantment_id)
            .ok()
            .and_then(|id| self.catalogs.enchantments?.get(id))
            .is_none_or(|entry| {
                self.enchantment_fits_requirements_like_cpp(u32::from(entry.condition_id), None)
            });
        if args.socket_context.is_none() {
            args.socket_context =
                self.current_item_enchantment_socket_context_like_cpp(item_guid, slot);
        }
        let mut item = self
            .state
            .remove_enchantment_item_object_with_access_like_cpp(&self.inventory, item_guid)?;
        let mut template = self.catalogs.template(enchantment_id, condition_fits);
        if let Some(template) = &mut template {
            if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                // Preserve the existing early return after removal when skill
                // authority is unavailable; this path does not reinsert the item.
                template.required_skill_value =
                    self.owner.resolved_player_skill_value_like_cpp(skill_id)?;
            }
        }
        let plan = self
            .owner
            .apply_enchantment_plan_like_cpp(&mut item, slot, template, args);
        self.state
            .insert_quest_reward_item_object_with_access_like_cpp(&self.inventory, item);
        plan
    }

    fn enchantment_fits_requirements_like_cpp(
        &self,
        enchantment_condition: u32,
        except_slot: Option<u8>,
    ) -> bool {
        if enchantment_condition == 0 {
            return true;
        }
        let Some(condition) = self
            .catalogs
            .conditions
            .and_then(|store| store.get(enchantment_condition))
        else {
            return true;
        };
        let mut gem_counts = [0u8; 4];
        for slot in 0..EQUIPMENT_SLOT_END {
            if except_slot == Some(slot) {
                continue;
            }
            let Some(inventory_item) = self
                .state
                .quest_reward_inventory_item_from_runtime_with_access_like_cpp(
                    &self.inventory,
                    slot,
                )
            else {
                continue;
            };
            let Some(item) = self
                .state
                .resolved_player_inventory_item_object_with_access_like_cpp(
                    &self.inventory,
                    inventory_item.guid,
                )
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
                    .catalogs
                    .item_stats
                    .and_then(|store| store.gem_properties(gem_item_id))
                    .map(u32::from)
                else {
                    continue;
                };
                let Some(gem_type) = self
                    .catalogs
                    .gems
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

    fn current_item_enchantment_socket_context_like_cpp(
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
        let item = self
            .state
            .resolved_player_inventory_item_object_with_access_like_cpp(
                &self.inventory,
                item_guid,
            )?;
        let socket_color = self
            .catalogs
            .item_stats
            .and_then(|store| store.socket_template(item.object().entry()))
            .map(|template| u32::from(template.socket_types[socket_index]))
            .unwrap_or(0);
        let gem_requirement = item
            .data()
            .gems
            .get(socket_index)
            .and_then(|gem| u32::try_from(gem.item_id).ok())
            .and_then(|gem_item_id| self.catalogs.item_stats?.socket_template(gem_item_id))
            .and_then(|gem_template| {
                Some(ApplyEnchantmentGemRequirementRef::new(
                    u32::from(gem_template.required_skill_id),
                    gem_template.required_skill_rank,
                    self.owner
                        .resolved_player_skill_value_like_cpp(gem_template.required_skill_id)?,
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
            .catalogs
            .template(prismatic_enchantment_id, true)
            .and_then(|mut template| {
                if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                    template.required_skill_value =
                        self.owner.resolved_player_skill_value_like_cpp(skill_id)?;
                }
                Some(template)
            });
        Some(ApplyEnchantmentSocketContext::prismatic(
            prismatic_enchantment,
            gem_requirement,
        ))
    }
}
