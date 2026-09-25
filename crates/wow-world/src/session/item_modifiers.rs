// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item modifiers: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::TitanGripPenaltyAction;
use super::{ApplyEnchantmentEffectAction, ApplyEnchantmentPlan, Arc, BANK_SLOT_BAG_START};
use super::{
    BANK_SLOT_BAG_END, BTreeMap, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
};
use super::{INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, ItemSubClassArmor, ObjectGuid};
use super::{
    PlayerEnchantTimeUpdate, PlayerStatsStore, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START,
};
use super::{ScalingStatDistributionEntry, ShieldBlockRegularGameTableLikeCpp, WeaponAttackType};
use super::{WorldSession, two_handed_in_one_hand_like_cpp};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedItemModsReapplyEventLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedItemBonusActionLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub action: ApplyEnchantmentEffectAction,
}

pub(in crate::session) const ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP: u32 = 0x01;

pub(crate) type RepresentedItemSetEffectLikeCpp = wow_entities::PlayerItemSetEffectLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedItemSetSpellEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub threshold: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedItemSetAuraRefreshEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub apply: bool,
    pub form_change: bool,
}

pub(crate) type RepresentedItemBonusStateLikeCpp = wow_entities::PlayerItemBonusStateLikeCpp;

pub(crate) fn void_withdrawal_post_store_item_values_update_like_cpp(
    item: &wow_entities::Item,
    create_dynamic_flags: u32,
) -> Option<wow_entities::ItemValuesUpdate> {
    let mut item_data_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    let mut has_parent_field = false;
    if !item.data().creator.is_empty() {
        item_data_mask.set(wow_entities::ITEM_DATA_CREATOR_BIT);
        has_parent_field = true;
    }
    if item.data().dynamic_flags != create_dynamic_flags {
        item_data_mask.set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT);
        has_parent_field = true;
    }
    if item.data().property_seed != 0 {
        item_data_mask.set(wow_entities::ITEM_DATA_PROPERTY_SEED_BIT);
        has_parent_field = true;
    }
    if item.data().random_properties_id != 0 {
        item_data_mask.set(wow_entities::ITEM_DATA_RANDOM_PROPERTIES_ID_BIT);
        has_parent_field = true;
    }
    if has_parent_field {
        item_data_mask.set(wow_entities::ITEM_DATA_PARENT_BIT);
    }
    for (index, enchantment) in item.data().enchantments.iter().enumerate() {
        if *enchantment != wow_entities::ItemEnchantment::default() {
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT);
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + index);
        }
    }
    if !item_data_mask.is_any_set() {
        return None;
    }
    Some(wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    })
}

pub(crate) fn item_storage_fields_values_update_like_cpp(
    item: &wow_entities::Item,
    contained_in_changed: bool,
    dynamic_flags2_changed: bool,
    changed_enchantments: &[wow_constants::item::EnchantmentSlot],
) -> wow_entities::ItemValuesUpdate {
    let mut item_data_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    if contained_in_changed || dynamic_flags2_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_PARENT_BIT);
    }
    if contained_in_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_CONTAINED_IN_BIT);
    }
    if dynamic_flags2_changed {
        item_data_mask.set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT);
    }
    if !changed_enchantments.is_empty() {
        item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT);
        for slot in changed_enchantments {
            item_data_mask.set(wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + *slot as usize);
        }
    }
    wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    }
}

pub(in crate::session) fn represented_player_stat_changes_like_cpp(
    state: &RepresentedItemBonusStateLikeCpp,
) -> wow_packet::packets::update::PlayerStatChanges {
    let mut changes = wow_packet::packets::update::PlayerStatChanges {
        base_mana: state.mana_base,
        base_health: state.health_base,
        attack_power: state.attack_power_total,
        ranged_attack_power: state.ranged_attack_power_total,
        stats: state.stats_base,
        stat_pos_buff: state.stats_base,
        armor: state.armor_base + state.armor_total + state.resistances_base[0],
        combat_ratings: state.combat_ratings,
        // This fixture has no aura/stat producers, so the item accumulator is
        // the whole represented `SpellBaseDamageBonusDone`/`HealingBonusDone`.
        mod_damage_done_pos: std::array::from_fn(|school| {
            if school == 0 {
                0
            } else {
                state.spell_power_bonus
            }
        }),
        mod_damage_done_neg: [0; 7],
        mod_healing_done_pos: state.spell_power_bonus,
        mod_damage_done_percent: [1.0; 7],
        shield_block: i32::try_from(state.shield_block_value).unwrap_or(i32::MAX),
        ..Default::default()
    };

    changes.min_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][0];
    changes.max_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][1];
    changes.min_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][0];
    changes.max_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][1];
    changes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct RepresentedScalingStatContextLikeCpp {
    pub(in crate::session) stat_id: [i32; 10],
    pub(in crate::session) bonus: [i32; 10],
    pub(in crate::session) ssd_multiplier: i32,
    pub(in crate::session) spell_bonus: i32,
    pub(in crate::session) armor_mod: i32,
    pub(in crate::session) dps_mod: i32,
    pub(in crate::session) is_two_hand: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedCombatStatRecalculationLikeCpp {
    Expertise { attack: WeaponAttackType },
    Rating { combat_rating: u8 },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
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

#[derive(Debug, Clone, Default)]
pub(crate) struct InitialLoadedItemModsOutcomeLikeCpp {
    pub item_set_auras: usize,
    pub item_equip_auras: usize,
    pub enchantments: LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
}

pub(in crate::session) fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

pub(in crate::session) fn player_class_mask_for_transmog_like_cpp(class_id: u8) -> u32 {
    if class_id == 0 || class_id > 32 {
        0
    } else {
        1_u32 << u32::from(class_id - 1)
    }
}

pub(in crate::session) fn player_class_mask_for_talent_like_cpp(class_id: u8) -> Option<u32> {
    if class_id == 0 || class_id > 32 {
        None
    } else {
        Some(1_u32 << u32::from(class_id - 1))
    }
}

pub(in crate::session) fn player_class_by_armor_subclass_like_cpp(subclass: u32) -> u32 {
    match subclass {
        x if x == ItemSubClassArmor::Miscellaneous as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Cloth as u32 => {
            (1 << (5 - 1)) | (1 << (8 - 1)) | (1 << (9 - 1))
        }
        x if x == ItemSubClassArmor::Leather as u32 => {
            (1 << (4 - 1)) | (1 << (10 - 1)) | (1 << (11 - 1)) | (1 << (12 - 1))
        }
        x if x == ItemSubClassArmor::Mail as u32 => (1 << (3 - 1)) | (1 << (7 - 1)),
        x if x == ItemSubClassArmor::Plate as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (6 - 1))
        }
        x if x == ItemSubClassArmor::Cosmetic as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Shield as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (7 - 1))
        }
        x if x == ItemSubClassArmor::Libram as u32 => 1 << (2 - 1),
        x if x == ItemSubClassArmor::Idol as u32 => 1 << (11 - 1),
        x if x == ItemSubClassArmor::Totem as u32 => 1 << (7 - 1),
        x if x == ItemSubClassArmor::Sigil as u32 => 1 << (6 - 1),
        x if x == ItemSubClassArmor::Relic as u32 => {
            (1 << (2 - 1)) | (1 << (6 - 1)) | (1 << (7 - 1)) | (1 << (11 - 1))
        }
        _ => 0,
    }
}

impl WorldSession {
    pub(in crate::session) fn find_free_backpack_slot_like_cpp(&self) -> Option<u8> {
        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.resolved_player_inventory_slot_count_like_cpp()?)
            .min(INVENTORY_SLOT_ITEM_END);
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        (INVENTORY_SLOT_ITEM_START..inventory_end).find(|slot| !inventory_items.contains_key(slot))
    }

    /// C++ `sImportPriceQualityStore.LookupEntry(quality + 1)`.
    #[cfg(test)]
    pub fn import_price_quality_factor_like_cpp(&self, quality: u32) -> Option<f32> {
        self.item_valuation_catalogs_for_test_like_cpp()
            .import_prices
            .quality
            .get(quality + 1)
            .map(|entry| entry.data)
    }

    pub fn set_shield_block_regular_game_table(
        &mut self,
        table: Arc<ShieldBlockRegularGameTableLikeCpp>,
    ) {
        self.shield_block_regular_game_table = Some(table);
    }

    /// Set the player stats store for this session.
    pub fn set_player_stats(&mut self, store: Arc<PlayerStatsStore>) {
        self.player_stats = Some(store);
    }

    /// Get the player stats store reference.
    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.player_stats.as_ref()
    }

    pub(in crate::session) fn represented_scaling_stat_context_like_cpp(
        &self,
        item_entry: u32,
    ) -> Option<RepresentedScalingStatContextLikeCpp> {
        let item_store = self.items.store.as_ref()?;
        let scaling_stat_distribution_id = item_store.scaling_stat_distribution_id(item_entry);
        let scaling_stat_value = item_store.scaling_stat_value(item_entry);
        if scaling_stat_distribution_id == 0 || scaling_stat_value == 0 {
            return None;
        }
        let distribution_store = self.scaling_stat_distribution_store.as_ref()?;
        let values_store = self.scaling_stat_values_store.as_ref()?;
        let distribution = distribution_store.get(u32::from(scaling_stat_distribution_id))?;
        let character_level = self.represented_scaling_stat_character_level_like_cpp(distribution);
        let values = values_store.get_for_character_level_like_cpp(character_level)?;
        let mask = scaling_stat_value as u32;
        Some(RepresentedScalingStatContextLikeCpp {
            stat_id: distribution.stat_id,
            bonus: distribution.bonus,
            ssd_multiplier: values.ssd_multiplier_like_cpp(mask),
            spell_bonus: values.spell_bonus_like_cpp(mask),
            armor_mod: values.armor_mod_like_cpp(mask),
            dps_mod: values.dps_mod_like_cpp(mask),
            is_two_hand: values.is_two_hand_like_cpp(mask),
        })
    }

    pub(in crate::session) fn represented_scaling_stat_character_level_like_cpp(
        &self,
        distribution: &ScalingStatDistributionEntry,
    ) -> u32 {
        let min_level = u32::try_from(distribution.min_level).unwrap_or(0);
        let max_level = u32::try_from(distribution.max_level).unwrap_or(min_level);
        let (min_level, max_level) = if min_level <= max_level {
            (min_level, max_level)
        } else {
            (max_level, min_level)
        };
        u32::from(self.player_level_like_cpp()).clamp(min_level, max_level)
    }

    pub(crate) fn record_represented_titan_grip_penalty_action_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let main_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_MAINHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let off_template = self
                .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_OFFHAND)
                .and_then(|item| self.item_storage_template(item.entry_id));
            let using_two_handed_weapon_in_one_hand =
                two_handed_in_one_hand_like_cpp(main_template.as_ref(), off_template.as_ref());

            let Some(action) = self.canonical_player_snapshot_like_cpp(|player| {
                let penalty_spell_id = player.titan_grip_penalty_spell_id();
                let has_penalty_aura = penalty_spell_id > 0
                    && self
                        .visible_auras
                        .values()
                        .any(|aura| aura.spell_id == penalty_spell_id as i32);

                player.check_titan_grip_penalty_action(
                    using_two_handed_weapon_in_one_hand,
                    has_penalty_aura,
                )
            }) else {
                return;
            };

            if action != TitanGripPenaltyAction::None {
                self.player_item_test_fixture_like_cpp
                    .represented_titan_grip_penalty_actions_like_cpp
                    .push(action);
            }
        }
    }

    pub(in crate::session) fn resolved_represented_total_stat_multiplier_for_stat_like_cpp(
        &self,
        stat: usize,
        uses_misc_value_b: bool,
    ) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let aura_type = wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE;
        let mut multiplier = 1.0f32;
        let mut same_effect_spell_groups = BTreeMap::<u32, i32>::new();

        for aura in visible_auras.values() {
            let Some(spell) = spell_store.get(aura.spell_id) else {
                continue;
            };

            for effect in spell.effects().iter().filter(|effect| {
                1u32.checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                    && effect.effect_aura == aura_type
                    && if uses_misc_value_b {
                        effect.effect_misc_value_2 == 0
                            || effect.effect_misc_value_2 & (1 << stat) != 0
                    } else {
                        effect.effect_misc_value_1 == -1
                            || effect.effect_misc_value_1 == stat as i32
                    }
            }) {
                let amount = aura
                    .represented_effect_amounts
                    .iter()
                    .find(|represented| u32::from(represented.effect_index) == effect.effect_index)
                    .map(|represented| represented.amount)
                    .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());

                let same_effect_group = self
                    .spell_spell_group_map_bounds_like_cpp(aura.spell_id as u32)
                    .iter()
                    .copied()
                    .find(|group_id| {
                        self.same_effect_stack_rule_aura_types_like_cpp(*group_id)
                            .is_some_and(|aura_types| aura_types.contains(&aura_type))
                    });
                if let Some(group_id) = same_effect_group {
                    same_effect_spell_groups
                        .entry(group_id)
                        .and_modify(|current| {
                            if current.unsigned_abs() < amount.unsigned_abs() {
                                *current = amount;
                            }
                        })
                        .or_insert(amount);
                } else {
                    multiplier += multiplier * amount as f32 / 100.0;
                }
            }
        }

        for amount in same_effect_spell_groups.into_values() {
            multiplier += multiplier * amount as f32 / 100.0;
        }
        Some(multiplier)
    }

    pub(crate) fn resolved_represented_total_stat_multipliers_like_cpp(&self) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, true)?;
        }
        Some(multipliers)
    }

    pub(crate) fn resolved_represented_total_stat_buff_multipliers_like_cpp(
        &self,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier =
                self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, false)?;
        }
        Some(multipliers)
    }

    #[cfg(test)]
    pub(crate) fn represented_total_stat_multipliers_like_cpp(&self) -> [f32; 5] {
        self.resolved_represented_total_stat_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn represented_total_stat_buff_multipliers_like_cpp(&self) -> [f32; 5] {
        self.resolved_represented_total_stat_buff_multipliers_like_cpp()
            .expect("test Player aura owner must resolve")
    }
}
