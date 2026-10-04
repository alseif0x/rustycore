// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_constants::{InventoryType, WeaponAttackType};
use wow_core::ObjectGuid;
use wow_data::{
    ItemStatsStore, ItemStore, ItemWeaponTemplateEntry, ScalingStatDistributionStore,
    ScalingStatValuesStore, ShieldBlockRegularGameTableLikeCpp, SpellShapeshiftFormStore,
};
use wow_entities::{
    ApplyEnchantmentEffectAction,
    item_resistance_bonus_actions_like_cpp, item_scaling_stat_bonus_actions_like_cpp,
    item_shield_block_bonus_action_like_cpp, item_stat_bonus_actions_like_cpp,
    item_weapon_damage_actions_like_cpp,
};
use wow_world_core::session::{
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp,
};

use crate::{
    InventoryState, RepresentedItemBonusActionLikeCpp,
    scaling::{
        represented_scaling_stat_context_from_selected_inputs_like_cpp,
        scaling_stat_character_level_like_cpp,
    },
};

/// The six catalog handles selected by the World session for item modifier planning.
///
/// The references keep one operation on the same catalog selection without cloning Arcs.
#[derive(Clone, Copy)]
pub struct ItemModsCatalogsViewLikeCpp<'a> {
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    scaling_stat_distribution_store: Option<&'a Arc<ScalingStatDistributionStore>>,
    scaling_stat_values_store: Option<&'a Arc<ScalingStatValuesStore>>,
    shield_block_regular_game_table: Option<&'a Arc<ShieldBlockRegularGameTableLikeCpp>>,
    spell_shapeshift_form_store: Option<&'a Arc<SpellShapeshiftFormStore>>,
}

impl<'a> ItemModsCatalogsViewLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
        scaling_stat_distribution_store: Option<&'a Arc<ScalingStatDistributionStore>>,
        scaling_stat_values_store: Option<&'a Arc<ScalingStatValuesStore>>,
        shield_block_regular_game_table: Option<&'a Arc<ShieldBlockRegularGameTableLikeCpp>>,
        spell_shapeshift_form_store: Option<&'a Arc<SpellShapeshiftFormStore>>,
    ) -> Self {
        Self {
            item_store,
            item_stats_store,
            scaling_stat_distribution_store,
            scaling_stat_values_store,
            shield_block_regular_game_table,
            spell_shapeshift_form_store,
        }
    }
}

struct InventoryItemModsCx<'a> {
    inventory: &'a mut InventoryState,
    inventory_access: &'a OwnedInventoryAccessLikeCpp<'a>,
    modifier_access: &'a OwnedItemModifiersAccessLikeCpp<'a>,
    catalogs: ItemModsCatalogsViewLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level_fixture: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    shapeshift_form_fixture: &'a u32,
}

impl InventoryState {
    /// Plan and apply one item's represented modifier actions against the selected Player owner.
    #[allow(clippy::too_many_arguments)]
    pub fn record_represented_item_mods_with_access_like_cpp(
        &mut self,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
        catalogs: ItemModsCatalogsViewLikeCpp<'_>,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
        #[cfg(any(test, feature = "test-fixtures"))] player_level_fixture: &u8,
        #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form_fixture: &u32,
        record_test_evidence: bool,
    ) -> usize {
        let mut cx = InventoryItemModsCx {
            inventory: self,
            inventory_access,
            modifier_access,
            catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            shapeshift_form_fixture,
        };
        cx.record(item_guid, slot, apply, record_test_evidence)
    }

    /// Re-read scaling inputs at the original weapon-bounds call site.
    pub fn represented_weapon_damage_bounds_with_access_like_cpp(
        &self,
        modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
        catalogs: &ItemModsCatalogsViewLikeCpp<'_>,
        item_entry: u32,
        weapon: &ItemWeaponTemplateEntry,
        #[cfg(any(test, feature = "test-fixtures"))] player_level_fixture: &u8,
    ) -> (f32, f32) {
        let mut min_damage = f32::from(weapon.min_damage[0]);
        let mut max_damage = f32::from(weapon.max_damage[0]);
        let Some(context) = represented_scaling_stat_context_from_selected_inputs_like_cpp(
            catalogs.item_store,
            catalogs.scaling_stat_distribution_store,
            catalogs.scaling_stat_values_store,
            item_entry,
            |distribution| {
                scaling_stat_character_level_like_cpp(distribution, || {
                    player_level_from_access_like_cpp(
                        modifier_access,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        player_level_fixture,
                    )
                })
            },
        ) else {
            return (min_damage, max_damage);
        };

        if context.dps_mod != 0 {
            let average = context.dps_mod as f32 * f32::from(weapon.item_delay) / 1000.0;
            let modifier = if context.is_two_hand { 0.2 } else { 0.3 };
            min_damage = (1.0 - modifier) * average;
            max_damage = (1.0 + modifier) * average;
        }

        (min_damage, max_damage)
    }
}

impl InventoryItemModsCx<'_> {
    fn record(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
        record_test_evidence: bool,
    ) -> usize {
        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.inventory
                .record_represented_item_mod_reapply_event_for_test_like_cpp(
                    item_guid, slot, apply,
                );
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = record_test_evidence;

        let Some(item_entry) = self
            .resolved_inventory_item_object(item_guid)
            .map(|item| item.object().entry())
        else {
            return 0;
        };
        let Some(item_stats_store) = self.catalogs.item_stats_store.cloned() else {
            return 0;
        };
        let mut planned_actions = Vec::new();

        let scaling_context = self.scaling_context(item_entry);
        if let Some(context) = scaling_context {
            planned_actions.extend(
                item_scaling_stat_bonus_actions_like_cpp(
                    &context.stat_id,
                    &context.bonus,
                    context.ssd_multiplier,
                    apply,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
            if context.spell_bonus > 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::SpellPowerBonus {
                        amount: context.spell_bonus as u32,
                        apply,
                    },
                });
            } else if context.spell_bonus < 0 {
                planned_actions.push(RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action: ApplyEnchantmentEffectAction::UnhandledStatModifier {
                        item_mod: wow_constants::ItemModType::SpellPower,
                        amount: context.spell_bonus.unsigned_abs(),
                        apply,
                    },
                });
            }
        } else if let Some(stat_entry) = item_stats_store.get(item_entry) {
            planned_actions.extend(
                item_stat_bonus_actions_like_cpp(&stat_entry.stats, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(stat_entry) = item_stats_store.get(item_entry) {
            let resistances = OwnedItemModifiersAccessLikeCpp::represented_resistances_with_scaling_armor_like_cpp(
                &stat_entry.resistances,
                scaling_context,
            );
            planned_actions.extend(
                item_resistance_bonus_actions_like_cpp(&resistances, apply)
                    .into_iter()
                    .map(|action| RepresentedItemBonusActionLikeCpp {
                        item_guid,
                        slot,
                        action,
                    }),
            );
        }

        if let Some(action) = OwnedItemModifiersAccessLikeCpp::item_shield_block_value_from_selected_inputs_like_cpp(
            self.catalogs.item_store,
            Some(&item_stats_store),
            self.catalogs.shield_block_regular_game_table,
            item_entry,
        )
        .and_then(|value| item_shield_block_bonus_action_like_cpp(value, true, apply))
        {
            planned_actions.push(RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot,
                action,
            });
        }

        if let (Some(weapon), Some(inventory_type)) = (
            item_stats_store.weapon_template(item_entry),
            self.inventory.represented_item_inventory_type_with_access_like_cpp(
                self.inventory_access,
                self.catalogs.item_store,
                item_entry,
                item_guid,
            ),
        ) {
            let (min_damage, max_damage) = self
                .inventory
                .represented_weapon_damage_bounds_with_access_like_cpp(
                    self.modifier_access,
                    &self.catalogs,
                    item_entry,
                    weapon,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    self.player_level_fixture,
                );
            // C++ `Player::_ApplyWeaponDamage` (`Player.cpp:7979-8020`) skips the
            // disarm gate in feral form and keeps the existing attack time while
            // the active form carries a `CombatRoundTime`.
            let is_in_feral_form = self
                .modifier_access
                .is_in_feral_form_like_cpp()
                .unwrap_or(false);
            // The Rust bridge preserves `!= Some(false)` when canonical-owner
            // lookup is unavailable. C++ `_ApplyWeaponDamage` has the disarm
            // and feral gates (Player.cpp:7975–8020), but assumes a Player.
            let attack_type = attack_type_for_slot_like_cpp(slot, Some(inventory_type));
            let can_use_attack_type = self
                .modifier_access
                .can_use_weapon_attack_type_like_cpp(attack_type)
                != Some(false);
            let has_shapeshift_combat_round_time = self
                .shapeshift_combat_round_time()
                .is_some();
            planned_actions.extend(
                item_weapon_damage_actions_like_cpp(
                    slot,
                    inventory_type,
                    min_damage,
                    max_damage,
                    weapon.item_delay,
                    apply,
                    is_in_feral_form,
                    can_use_attack_type,
                    has_shapeshift_combat_round_time,
                    true,
                )
                .into_iter()
                .map(|action| RepresentedItemBonusActionLikeCpp {
                    item_guid,
                    slot,
                    action,
                }),
            );
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if record_test_evidence {
            self.inventory
                .record_represented_item_bonus_actions_for_test_like_cpp(&planned_actions);
        }

        let action_count = planned_actions.len();
        for planned in planned_actions {
            self.inventory
                .apply_represented_item_bonus_action_with_access_like_cpp(
                    self.modifier_access,
                    planned.action,
                );
        }
        action_count
    }

    fn resolved_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::Item> {
        self.inventory
            .resolved_player_inventory_runtime_with_access_like_cpp(self.inventory_access)?
            .item_objects()
            .get(&item_guid)
            .cloned()
    }

    fn scaling_context(
        &self,
        item_entry: u32,
    ) -> Option<wow_world_core::session::RepresentedScalingStatContextLikeCpp> {
        represented_scaling_stat_context_from_selected_inputs_like_cpp(
            self.catalogs.item_store,
            self.catalogs.scaling_stat_distribution_store,
            self.catalogs.scaling_stat_values_store,
            item_entry,
            |distribution| {
                scaling_stat_character_level_like_cpp(distribution, || self.player_level())
            },
        )
    }

    fn player_level(&self) -> u8 {
        player_level_from_access_like_cpp(
            self.modifier_access,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_level_fixture,
        )
    }

    fn shapeshift_combat_round_time(&self) -> Option<f32> {
        let canonical = self.modifier_access.shapeshift_form_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let form_id = canonical.or_else(|| {
            if self.modifier_access.owner_handle_absent_like_cpp() {
                Some(*self.shapeshift_form_fixture)
            } else {
                None
            }
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let form_id = canonical;

        let form = self.catalogs.spell_shapeshift_form_store?.get(form_id?)?;
        (form.combat_round_time > 0).then(|| f32::from(form.combat_round_time))
    }
}

fn player_level_from_access_like_cpp(
    modifier_access: &OwnedItemModifiersAccessLikeCpp<'_>,
    #[cfg(any(test, feature = "test-fixtures"))] player_level_fixture: &u8,
) -> u8 {
    if let Some(level) = modifier_access.player_level_snapshot_like_cpp() {
        return level;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    {
        return *player_level_fixture;
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    modifier_access.normal_player_level_fallback_like_cpp()
}

pub(super) fn attack_type_for_slot_like_cpp(
    slot: u8,
    inventory_type: Option<InventoryType>,
) -> WeaponAttackType {
    match slot {
        wow_entities::EQUIPMENT_SLOT_MAINHAND
            if matches!(
                inventory_type,
                Some(InventoryType::Ranged | InventoryType::RangedRight)
            ) =>
        {
            WeaponAttackType::RangedAttack
        }
        wow_entities::EQUIPMENT_SLOT_MAINHAND => WeaponAttackType::BaseAttack,
        wow_entities::EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
        _ => WeaponAttackType::Max,
    }
}
