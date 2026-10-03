// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::RepresentedItemBonusActionLikeCpp;
use wow_constants::{item::EnchantmentSlot, SpellItemEnchantmentFlags};
use wow_core::ObjectGuid;
use wow_entities::{
    ApplyEnchantmentArgs, ApplyEnchantmentGemRequirementRef, ApplyEnchantmentPlan,
    ApplyEnchantmentSocketContext, EQUIPMENT_SLOT_END, PlayerEnchantTimeUpdate,
};
use wow_packet::packets::item::ItemEnchantTimeUpdate;
use wow_world_core::session::{HubMut, HubRef, SKILL_ENCHANTING_LIKE_CPP};

#[derive(Debug, Clone, Default)]
pub struct LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub plans: Vec<ApplyEnchantmentPlan>,
    pub duration_updates: Vec<PlayerEnchantTimeUpdate>,
    pub send_stat_update: bool,
    pub visible_item_changes: Vec<(u8, i32, u16, u16)>,
    pub effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
    pub unrepresented_effect_actions: Vec<RepresentedItemBonusActionLikeCpp>,
}

impl LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
    pub(crate) fn append(&mut self, mut other: Self) {
        self.plans.append(&mut other.plans);
        self.duration_updates.append(&mut other.duration_updates);
        self.send_stat_update |= other.send_stat_update;
        self.visible_item_changes
            .append(&mut other.visible_item_changes);
        self.effect_actions.append(&mut other.effect_actions);
        self.unrepresented_effect_actions
            .append(&mut other.unrepresented_effect_actions);
    }
}

impl crate::InventoryState {
    /// C++ `Player::EnchantmentFitsRequirements` for the currently equipped gems.
    fn enchantment_fits_requirements_like_cpp(
        &self,
        hub: HubRef<'_>,
        enchantment_condition: u32,
        except_slot: Option<u8>,
    ) -> bool {
        if enchantment_condition == 0 {
            return true;
        }
        let Some(condition) = hub
            .catalogs
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
            let Some(inventory_item) = self.resolved_inventory_item_like_cpp(hub, slot) else {
                continue;
            };
            let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, inventory_item.guid)
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
                let Some(gem_properties_id) = hub
                    .catalogs
                    .items
                    .stats_store
                    .as_ref()
                    .and_then(|store| store.gem_properties(gem_item_id))
                    .map(u32::from)
                else {
                    continue;
                };
                let Some(gem_type) = hub
                    .catalogs
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

    fn current_item_enchantment_socket_context_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
    ) -> Option<ApplyEnchantmentSocketContext> {
        let socket_index = match slot {
            EnchantmentSlot::EnhancementSocket => 0,
            EnchantmentSlot::EnhancementSocket2 => 1,
            EnchantmentSlot::EnhancementSocket3 => 2,
            _ => return None,
        };
        let item = self.resolved_inventory_item_object_like_cpp(hub, item_guid)?;
        let socket_color = hub
            .catalogs
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
                hub.catalogs
                    .items
                    .stats_store
                    .as_ref()?
                    .socket_template(gem_item_id)
            })
            .and_then(|gem_template| {
                Some(ApplyEnchantmentGemRequirementRef::new(
                    u32::from(gem_template.required_skill_id),
                    gem_template.required_skill_rank,
                    hub.resolved_player_skill_value_like_cpp(gem_template.required_skill_id)?,
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
        let prismatic_enchantment = hub
            .catalogs
            .apply_enchantment_template_ref(prismatic_enchantment_id, 0, true)
            .and_then(|mut template| {
                if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                    template.required_skill_value =
                        hub.resolved_player_skill_value_like_cpp(skill_id)?;
                }
                Some(template)
            });
        Some(ApplyEnchantmentSocketContext::prismatic(
            prismatic_enchantment,
            gem_requirement,
        ))
    }

    /// C++ `Player::ApplyEnchantment(item, slot, apply, ...)` bridge for a
    /// represented inventory item owned by the current player.
    ///
    /// The item runtime lives in the session inventory while the player state
    /// lives in the canonical map. This temporarily moves the item out, runs the
    /// entity-level plan against the canonical player when available, then puts
    /// the item back without clearing or setting the enchantment field itself.
    pub fn apply_current_player_item_enchantment_plan_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
        slot: EnchantmentSlot,
        mut args: ApplyEnchantmentArgs,
    ) -> Option<ApplyEnchantmentPlan> {
        let enchantment_id = self
            .resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)?
            .data()
            .enchantments[slot as usize]
            .id;
        let condition_fits = u32::try_from(enchantment_id)
            .ok()
            .and_then(|id| {
                hub.catalogs
                    .spell_catalogs
                    .spell_item_enchantment_store
                    .as_ref()?
                    .get(id)
            })
            .is_none_or(|entry| {
                self.enchantment_fits_requirements_like_cpp(
                    hub.shared(),
                    u32::from(entry.condition_id),
                    None,
                )
            });
        if args.socket_context.is_none() {
            args.socket_context = self.current_item_enchantment_socket_context_like_cpp(
                hub.shared(),
                item_guid,
                slot,
            );
        }
        let mut item = self.remove_inventory_item_object(hub, item_guid)?;
        let mut template =
            hub.catalogs
                .apply_enchantment_template_ref(enchantment_id, 0, condition_fits);
        if let Some(template) = &mut template {
            if let Ok(skill_id) = u16::try_from(template.required_skill_id) {
                template.required_skill_value = hub
                    .shared()
                    .resolved_player_skill_value_like_cpp(skill_id)?;
            }
        }

        let plan = hub.core.mutate_canonical_player_like_cpp(|player| {
            player.apply_enchantment_plan(Some(&mut item), slot, template, args)
        });
        self.insert_inventory_item_object(hub, item);
        plan
    }

    /// C++ `_StoreItem` merge branch calls `AddEnchantmentDurations(pItem2)`
    /// without calling `AddItemDurations` for the existing destination stack.
    pub fn refresh_inventory_item_enchantment_duration_refs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_guid: ObjectGuid,
    ) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(hub.shared(), item_guid)
        else {
            return;
        };
        let Some((owner_guid, enchantment_updates)) =
            hub.core.mutate_canonical_player_like_cpp(|player| {
                (player.guid(), player.add_enchantment_durations(&mut item))
            })
        else {
            return;
        };

        self.insert_inventory_item_object(hub, item);
        self.send_item_enchant_time_update_plans(hub.shared(), owner_guid, &enchantment_updates);
    }

    /// C++ `Player::RemoveItem` clears main-hand-only enchantments when the
    /// main-hand item leaves that slot. Return both the post-remove DB value
    /// and the runtime slots to clear, without mutating live state before the
    /// caller's transaction commits.
    pub fn inventory_remove_enchantment_persistence_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
        clear_mainhand_only: bool,
    ) -> Option<(String, Vec<EnchantmentSlot>)> {
        let item = self.resolved_inventory_item_object_like_cpp(hub, item_guid)?;
        let current_durations = hub
            .core
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .enchant_durations()
                    .iter()
                    .filter(|duration| duration.item_guid == item_guid)
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut cleared = Vec::new();
        let mut persisted = String::new();

        for (index, enchantment) in item.data().enchantments.iter().enumerate() {
            let Some(slot) = <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(index)
            else {
                continue;
            };
            let enchantment_entry = u32::try_from(enchantment.id)
                .ok()
                .and_then(|id| {
                    hub.catalogs
                        .spell_catalogs
                        .spell_item_enchantment_store
                        .as_ref()?
                        .get(id)
                })
                .copied();
            let clear_mainhand = clear_mainhand_only
                && enchantment_entry.is_some_and(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::MAINHAND_ONLY)
                });
            if clear_mainhand {
                cleared.push(slot);
            }
            if clear_mainhand
                || enchantment_entry.is_none_or(|entry| {
                    entry
                        .flags
                        .contains(SpellItemEnchantmentFlags::DO_NOT_SAVE_TO_DB)
                })
            {
                persisted.push_str("0 0 0 ");
            } else {
                let duration = current_durations
                    .iter()
                    .find(|duration| duration.slot == slot)
                    .map_or(enchantment.duration, |duration| duration.left_duration_ms);
                persisted.push_str(&format!(
                    "{} {} {} ",
                    enchantment.id, duration, enchantment.charges
                ));
            }
        }

        Some((persisted, cleared))
    }

    pub fn send_loaded_equipped_item_enchantment_updates_like_cpp(
        &self,
        hub: HubRef<'_>,
        outcome: &LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
    ) {
        if let Some(owner_guid) = hub.core.player_guid() {
            self.send_item_enchant_time_update_plans(hub, owner_guid, &outcome.duration_updates);
        }
        if !outcome.visible_item_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                hub,
                &[],
                &outcome.visible_item_changes,
                &[],
                &[],
                None,
            );
        }
    }
}

impl crate::InventoryState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn item_disenchant_loot_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
        quality: u32,
        item_level: u32,
        can_disenchant_bonus: bool,
    ) -> Option<(u32, u16)> {
        hub.catalogs.item_disenchant_loot_with_catalogs_like_cpp(
            &hub.catalogs.item_valuation_catalogs_for_test_like_cpp(),
            item_id,
            quality,
            item_level,
            can_disenchant_bonus,
        )
    }

    pub fn send_item_enchant_time_update_plan(
        &self,
        hub: HubRef<'_>,
        owner_guid: ObjectGuid,
        update: &PlayerEnchantTimeUpdate,
    ) {
        hub.core.send_packet(&ItemEnchantTimeUpdate {
            owner_guid,
            item_guid: update.item_guid,
            duration_left: update.duration_secs,
            slot: update.slot as u32,
        });
    }

    pub fn send_item_enchant_time_update_plans(
        &self,
        hub: HubRef<'_>,
        owner_guid: ObjectGuid,
        updates: &[PlayerEnchantTimeUpdate],
    ) {
        for update in updates {
            self.send_item_enchant_time_update_plan(hub, owner_guid, update);
        }
    }

    pub fn resolved_enchanting_skill_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u16> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            player.enchanting_skill_value_like_cpp(SKILL_ENCHANTING_LIKE_CPP)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(hub.fixtures.progression.represented_enchanting_skill);
        }
        canonical
    }
}
