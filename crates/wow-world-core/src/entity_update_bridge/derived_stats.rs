//! Derived Player values absent from the canonical Unit/ActivePlayer DTOs.

use wow_entities::PlayerDerivedStatValuesUpdate;
use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UnitDataValuesDeltaUpdate};

pub(super) fn merge(
    update: &PlayerDerivedStatValuesUpdate,
    unit: Option<&mut UnitDataValuesDeltaUpdate>,
    active: Option<&mut ActivePlayerDataValuesUpdate>,
) {
    let stats = &update.effective_combat_stats;
    if let Some(unit) = unit {
        unit.min_damage = stats.min_damage;
        unit.max_damage = stats.max_damage;
        unit.min_off_hand_damage = stats.weapon_damage[1][0];
        unit.max_off_hand_damage = stats.weapon_damage[1][1];
        unit.base_health = stats.base_health;
        unit.attack_power = stats.attack_power;
        unit.attack_power_mod_pos = stats.attack_power_mod_pos;
        unit.attack_power_mod_neg = stats.attack_power_mod_neg;
        unit.attack_power_multiplier = stats.attack_power_multiplier;
        unit.ranged_attack_power = stats.ranged_attack_power;
        unit.ranged_attack_power_mod_pos = stats.ranged_attack_power_mod_pos;
        unit.ranged_attack_power_mod_neg = stats.ranged_attack_power_mod_neg;
        unit.ranged_attack_power_multiplier = stats.ranged_attack_power_multiplier;
        unit.min_ranged_damage = stats.min_ranged_damage;
        unit.max_ranged_damage = stats.max_ranged_damage;
        unit.stats = stats.stats;
        unit.stat_pos_buff = stats.stat_pos_buff;
        unit.stat_neg_buff = stats.stat_neg_buff;
        unit.resistances = stats.resistances;

        if let Some(index) = update.mana_power_index {
            if let Some(value) = unit.power_regen_flat_modifier.get_mut(index) {
                *value = stats.mana_regen;
            }
            if let Some(value) = unit.power_regen_interrupted_flat_modifier.get_mut(index) {
                *value = stats.mana_regen_combat;
            }
        }
    }

    if let Some(active) = active {
        active.mainhand_expertise = stats.mainhand_expertise;
        active.offhand_expertise = stats.offhand_expertise;
        active.ranged_expertise = stats.ranged_expertise;
        active.combat_rating_expertise = stats.combat_rating_expertise;
        active.block_percentage = stats.block_pct;
        active.dodge_percentage = stats.dodge_pct;
        active.dodge_percentage_from_attribute = stats.dodge_from_attr;
        active.parry_percentage = stats.parry_pct;
        active.parry_percentage_from_attribute = stats.parry_from_attr;
        active.crit_percentage = stats.crit_pct;
        active.ranged_crit_percentage = stats.ranged_crit_pct;
        active.offhand_crit_percentage = stats.offhand_crit_pct;
        active.shield_block = stats.shield_block;
        active.shield_block_crit_percentage = stats.shield_block_crit_pct;
        active.versatility_bonus = stats.versatility_bonus;
        active.mod_healing_done_pos = stats.mod_healing_done_pos;
        active.mod_healing_done_percent = stats.mod_healing_done_percent;
        active.override_spell_power_by_ap_percent = stats.override_spell_power_by_ap_percent;
        active.override_ap_by_spell_power_percent = stats.override_ap_by_spell_power_percent;
        active.mod_target_resistance = stats.mod_target_resistance;
        active.mod_target_physical_resistance = stats.mod_target_physical_resistance;
        active.spell_crit_percentage = stats.spell_crit_pct;
        active.mod_damage_done_pos = stats.mod_damage_done_pos;
        active.mod_damage_done_neg = stats.mod_damage_done_neg;
        active.mod_damage_done_percent = stats.mod_damage_done_percent;
        active.combat_ratings = stats.combat_ratings;
    }
}
