// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free Session rules, part 2 of 2.
//!
//! Moved out of `impl WorldSession` under #676. Every one was already
//! receiver-free, so it cannot read or write session state: these are rules,
//! not session behaviour. Bodies and signatures are unchanged.

use crate::entity_update_bridge::game_object_values_update_to_update_object;
#[cfg(test)]
use crate::session::NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP;
use crate::session::*;
use std::collections::HashMap;
use std::collections::HashSet;
#[cfg(test)]
use std::sync::atomic::Ordering;
use wow_constants::ClientOpcodes;
use wow_constants::InventoryType;
use wow_constants::ServerOpcodes;
use wow_constants::unit::PowerType;
use wow_core::ObjectGuid;
use wow_core::Position;
use wow_core::guid::HighGuid;
use wow_entities::AccessorObjectKind;
use wow_entities::ApplyEnchantmentEffectAction;
use wow_entities::EQUIPMENT_SLOT_BACK;
use wow_entities::EQUIPMENT_SLOT_BODY;
use wow_entities::EQUIPMENT_SLOT_CHEST;
use wow_entities::EQUIPMENT_SLOT_FEET;
use wow_entities::EQUIPMENT_SLOT_FINGER1;
use wow_entities::EQUIPMENT_SLOT_FINGER2;
use wow_entities::EQUIPMENT_SLOT_HANDS;
use wow_entities::EQUIPMENT_SLOT_HEAD;
use wow_entities::EQUIPMENT_SLOT_LEGS;
use wow_entities::EQUIPMENT_SLOT_MAINHAND;
use wow_entities::EQUIPMENT_SLOT_NECK;
use wow_entities::EQUIPMENT_SLOT_OFFHAND;
use wow_entities::EQUIPMENT_SLOT_SHOULDERS;
use wow_entities::EQUIPMENT_SLOT_TRINKET1;
use wow_entities::EQUIPMENT_SLOT_TRINKET2;
use wow_entities::EQUIPMENT_SLOT_WAIST;
use wow_entities::EQUIPMENT_SLOT_WRISTS;
use wow_entities::ITEM_DATA_BITS;
use wow_entities::ITEM_DATA_CREATOR_BIT;
use wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT;
use wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT;
use wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT;
use wow_entities::ITEM_DATA_PARENT_BIT;
use wow_entities::ITEM_DATA_PROPERTY_SEED_BIT;
use wow_entities::ITEM_DATA_RANDOM_PROPERTIES_ID_BIT;
use wow_entities::Item;
use wow_entities::ItemDataUpdate;
use wow_entities::ItemValuesUpdate;
use wow_entities::TYPEID_ITEM;
use wow_entities::UpdateMask;

pub(crate) fn movement_speed_ack_move_type_like_cpp(
    opcode: ClientOpcodes,
) -> Option<UnitMoveTypeLikeCpp> {
    match opcode {
        ClientOpcodes::MoveForceWalkSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Walk),
        ClientOpcodes::MoveForceRunSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Run),
        ClientOpcodes::MoveForceRunBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::RunBack),
        ClientOpcodes::MoveForceSwimSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Swim),
        ClientOpcodes::MoveForceSwimBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::SwimBack),
        ClientOpcodes::MoveForceTurnRateChangeAck => Some(UnitMoveTypeLikeCpp::TurnRate),
        ClientOpcodes::MoveForceFlightSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Flight),
        ClientOpcodes::MoveForceFlightBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::FlightBack),
        ClientOpcodes::MoveForcePitchRateChangeAck => Some(UnitMoveTypeLikeCpp::PitchRate),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn next_represented_battle_pet_guid_like_cpp() -> ObjectGuid {
    let counter = NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed);
    ObjectGuid::create_global(HighGuid::BattlePet, 0, counter)
}

/// C++ `Player::RemoveCurrency` underflow guard for vendor costs.
pub(crate) fn plan_remove_currency_like_cpp(
    currencies: &mut HashMap<u32, PlayerCurrency>,
    currency_id: u32,
    amount: u32,
) -> bool {
    if amount == 0 {
        return true;
    }

    let Some(currency) = currencies.get_mut(&currency_id) else {
        return false;
    };
    if currency.quantity == 0 {
        return false;
    }

    let removed = amount.min(currency.quantity);
    currency.quantity -= removed;
    if currency.state != PlayerCurrencyState::New {
        currency.state = PlayerCurrencyState::Changed;
    }
    true
}

pub(crate) fn player_aura_info_like_cpp(
    aura: &AuraApplication,
    player_level: u8,
    map_id: u16,
) -> wow_packet::packets::misc::AuraInfoLikeCpp {
    let duration_ms = (aura.duration_total > 0).then_some(aura.duration_total);
    let remaining_ms = (aura.duration_remaining > 0).then_some(aura.duration_remaining);
    let points = if aura.aura_flags & AFLAG_SCALABLE_LIKE_CPP != 0 {
        aura.represented_effect_amounts
            .iter()
            .filter(|effect| {
                effect.effect_index < u32::BITS as u8
                    && aura.effect_mask & (1u32 << effect.effect_index) != 0
            })
            .map(|effect| effect.amount as f32)
            .collect()
    } else {
        Vec::new()
    };

    wow_packet::packets::misc::AuraInfoLikeCpp {
        slot: aura.slot,
        aura_data: Some(wow_packet::packets::misc::AuraDataInfoLikeCpp {
            cast_id: ObjectGuid::create_world_object(
                HighGuid::Cast,
                3,
                aura.caster_guid.realm_id().max(1),
                map_id,
                0,
                u32::try_from(aura.spell_id).unwrap_or(0),
                i64::from(aura.slot) + 1,
            ),
            spell_id: aura.spell_id,
            flags: aura.aura_flags.min(u32::from(u16::MAX)) as u16,
            active_flags: aura.effect_mask,
            caster_guid: aura.caster_guid,
            cast_level: player_level.into(),
            applications: aura.stack_count.saturating_sub(1),
            duration_ms,
            remaining_ms,
            points,
        }),
    }
}

pub(crate) fn player_homebind_update_request_like_cpp(
    homebind: RepresentedHomebindLikeCpp,
    guid_counter: u64,
) -> wow_persistence::PlayerHomebindPersistenceRequestLikeCpp {
    wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
        player_guid: guid_counter,
        map_id: homebind.map_id,
        area_id: homebind.area_id,
        x: homebind.position.x,
        y: homebind.position.y,
        z: homebind.position.z,
        orientation: homebind.position.orientation,
    }
}

pub(crate) fn player_movement_speed_opcodes_like_cpp(
    move_type: UnitMoveTypeLikeCpp,
) -> Option<(ServerOpcodes, ServerOpcodes)> {
    match move_type {
        UnitMoveTypeLikeCpp::Run => Some((
            ServerOpcodes::MoveSetRunSpeed,
            ServerOpcodes::MoveUpdateRunSpeed,
        )),
        UnitMoveTypeLikeCpp::Flight => Some((
            ServerOpcodes::MoveSetFlightSpeed,
            ServerOpcodes::MoveUpdateFlightSpeed,
        )),
        UnitMoveTypeLikeCpp::Swim => Some((
            ServerOpcodes::MoveSetSwimSpeed,
            ServerOpcodes::MoveUpdateSwimSpeed,
        )),
        UnitMoveTypeLikeCpp::RunBack => Some((
            ServerOpcodes::MoveSetRunBackSpeed,
            ServerOpcodes::MoveUpdateRunBackSpeed,
        )),
        UnitMoveTypeLikeCpp::SwimBack => Some((
            ServerOpcodes::MoveSetSwimBackSpeed,
            ServerOpcodes::MoveUpdateSwimBackSpeed,
        )),
        UnitMoveTypeLikeCpp::FlightBack => Some((
            ServerOpcodes::MoveSetFlightBackSpeed,
            ServerOpcodes::MoveUpdateFlightBackSpeed,
        )),
        _ => None,
    }
}

/// Whether one effective spell effect is inert for the deliberately
/// narrow Creature -> Player physical melee hit profile while the Creature
/// is behind the Player.
///
/// This is not a general aura safety list. In particular total stats may
/// alter avoidance, but C++ disables Player dodge/parry/block from behind
/// unless aura type 288 is present; that type is intentionally absent
/// here. The remaining accepted aura types affect visibility, outgoing
/// damage, reputation, or the victim's outgoing expertise.
pub(crate) fn player_target_spell_effect_is_hit_inert_like_cpp(
    effect: &wow_data::spell::SpellEffectInfo,
) -> bool {
    use wow_data::spell::{aura_types, spell_effect_types};

    if effect.effect == spell_effect_types::SPELL_EFFECT_NONE
        || spell_effect_types::is_cpp_null_or_unused_noop(effect.effect)
    {
        return true;
    }
    if effect.effect_trigger_spell != 0 {
        return false;
    }

    match effect.effect {
        spell_effect_types::SPELL_EFFECT_APPLY_AURA => matches!(
            effect.effect_aura,
            aura_types::SPELL_AURA_MOD_STEALTH_DETECT
                | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
                | aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                | aura_types::SPELL_AURA_MOD_REPUTATION_GAIN
                | aura_types::SPELL_AURA_MOD_XP_PCT
                | aura_types::SPELL_AURA_MOD_EXPERTISE
        ),
        // These login-time capability effects either change only the
        // Player's outgoing attacks/equipment use or the defensive
        // parry/block gates that C++ disables for a rear attacker.
        spell_effect_types::SPELL_EFFECT_PARRY
        | spell_effect_types::SPELL_EFFECT_BLOCK
        | spell_effect_types::SPELL_EFFECT_DUAL_WIELD
        | spell_effect_types::SPELL_EFFECT_PROFICIENCY => true,
        _ => false,
    }
}

pub(crate) fn position_is_within_area_trigger_box_like_cpp(
    pos: &Position,
    center: &Position,
    half_length: f32,
    half_width: f32,
    half_height: f32,
) -> bool {
    let dx = pos.x - center.x;
    let dy = pos.y - center.y;
    let cos_yaw = center.orientation.cos();
    let sin_yaw = center.orientation.sin();
    let rel_x = dx * cos_yaw + dy * sin_yaw;
    let rel_y = -dx * sin_yaw + dy * cos_yaw;

    rel_x.abs() <= half_length
        && rel_y.abs() <= half_width
        && (pos.z - center.z).abs() <= half_height
}

pub(crate) fn represented_avg_total_item_level_maybe_replace_slot_like_cpp(
    best_item_levels: &mut [(InventoryType, u32, ObjectGuid)],
    sum: &mut u32,
    slot: u8,
    inventory_type: InventoryType,
    item_level: u32,
    item_guid: ObjectGuid,
    check_duplicate_guid: bool,
) {
    if check_duplicate_guid
        && best_item_levels
            .iter()
            .any(|(_, _, existing_guid)| *existing_guid == item_guid)
    {
        return;
    }

    let slot_data = &mut best_item_levels[slot as usize];
    if item_level > slot_data.1 {
        *sum = sum.saturating_add(item_level.saturating_sub(slot_data.1));
        *slot_data = (inventory_type, item_level, item_guid);
    }
}

pub(crate) fn represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
    guid: ObjectGuid,
    bytes: &[u8],
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    guid.hash(&mut hasher);
    bytes.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn represented_gameobject_chest_loot_ids_like_cpp(
    source: wow_entities::GameObjectLootSource,
) -> [u32; 3] {
    [source.loot_id, source.personal_loot_id, source.push_loot_id]
}

pub(crate) fn represented_gameobject_dynamic_flags_update_like_cpp(
    guid: ObjectGuid,
    map_id: u16,
    dynamic_flags: u32,
) -> Option<wow_packet::packets::update::UpdateObject> {
    let mut mask = wow_entities::UpdateMask::new(wow_entities::OBJECT_DATA_BITS);
    mask.set(wow_entities::OBJECT_DATA_PARENT_BIT);
    mask.set(wow_entities::OBJECT_DATA_DYNAMIC_FLAGS_BIT);
    let values_update = wow_entities::GameObjectValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
        object_data: Some(wow_entities::ObjectDataUpdate {
            mask,
            values: wow_entities::ObjectDataValues {
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            },
        }),
        game_object_data: None,
    };
    game_object_values_update_to_update_object(guid, map_id, &values_update)
}

pub(crate) fn represented_item_bonus_action_updates_stats_like_cpp(
    action: ApplyEnchantmentEffectAction,
) -> bool {
    matches!(
        action,
        ApplyEnchantmentEffectAction::UnitModifier { .. }
            | ApplyEnchantmentEffectAction::UpdateStatBuffMod(_)
            | ApplyEnchantmentEffectAction::RatingModifier { .. }
            | ApplyEnchantmentEffectAction::ManaRegenBonus { .. }
            | ApplyEnchantmentEffectAction::SpellPowerBonus { .. }
            | ApplyEnchantmentEffectAction::HealthRegenBonus { .. }
            | ApplyEnchantmentEffectAction::SpellPenetrationBonus { .. }
            | ApplyEnchantmentEffectAction::BaseModFlatValue { .. }
            | ApplyEnchantmentEffectAction::SetShieldBlockValue { .. }
            | ApplyEnchantmentEffectAction::SetBaseWeaponDamage { .. }
            | ApplyEnchantmentEffectAction::SetBaseAttackTime { .. }
            | ApplyEnchantmentEffectAction::UpdateDamagePhysical { .. }
    )
}

pub(crate) fn represented_seer_kinds_like_cpp() -> &'static [AccessorObjectKind] {
    &[
        AccessorObjectKind::Player,
        AccessorObjectKind::Creature,
        AccessorObjectKind::Pet,
        AccessorObjectKind::DynamicObject,
    ]
}

/// Represented C++ `SpellInfo::IsPositive` (`NegativeEffects.none()`).
/// The current spell model does not yet persist C++'s calculated
/// `NegativeEffects` bitset. This mirrors the represented C++ target-check,
/// intrinsically harmful effect/aura, and sign-sensitive stat families;
/// unknown non-enemy auras remain positive, as in C++'s default branch.
pub(crate) fn represented_spell_is_positive_like_cpp(spell_info: &wow_data::SpellInfo) -> bool {
    let effects: Vec<(u32, i32, i32, u32, u32)> = if spell_info.effects().is_empty() {
        vec![(
            spell_info.effect_type,
            spell_info.aura_type.unwrap_or(0),
            spell_info.effect_base_points,
            0,
            0,
        )]
    } else {
        spell_info
            .effects()
            .iter()
            .map(|effect| {
                (
                    effect.effect,
                    effect.effect_aura,
                    effect.effect_base_points,
                    effect.implicit_target_1,
                    effect.implicit_target_2,
                )
            })
            .collect()
    };

    const fn target_checks_enemy_like_cpp(target: u32) -> bool {
        matches!(
            target,
            2 | 6 | 15 | 16 | 24 | 28 | 53 | 54 | 93 | 104 | 108 | 115 | 116 | 129 | 134 | 151
        )
    }

    !effects
        .into_iter()
        .any(|(effect, aura, amount, target_a, target_b)| {
            let targets_enemy =
                target_checks_enemy_like_cpp(target_a) || target_checks_enemy_like_cpp(target_b);
            effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                || effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT
                || (effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && (targets_enemy
                        || matches!(
                            aura,
                            wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE
                                | wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE_PERCENT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_FEAR
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_STUN
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_ROOT
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_SILENCE
                                | wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED
                                | wow_data::spell::aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB
                        )
                        || (matches!(
                        aura,
                        wow_data::spell::aura_types::SPELL_AURA_MOD_STAT
                            | wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_HEALTH
                            | wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_HEALTH_PERCENT
                            | wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE
                    ) && amount < 0)))
        })
}

pub(crate) fn represented_spell_power_has_power_like_cpp(
    power_costs: &[wow_data::SpellPowerCostLikeCpp],
    before_power: &[(i8, i32, i32)],
) -> bool {
    power_costs.iter().all(|cost| {
        if cost.amount <= 0 {
            return true;
        }
        let Some(power_type) = <PowerType as num_traits::FromPrimitive>::from_i8(cost.power_type)
        else {
            return true;
        };
        if matches!(
            power_type,
            PowerType::Health | PowerType::None | PowerType::Max
        ) {
            return true;
        }
        before_power
            .iter()
            .find(|(snapshot_power_type, _, _)| *snapshot_power_type == cost.power_type)
            .map(|(_, current, _)| *current >= cost.amount)
            .unwrap_or(false)
    })
}

pub(crate) fn represented_spell_valid_with_seen_like_cpp(
    spell_store: &wow_data::SpellStore,
    spell_id: i32,
    seen: &mut HashSet<i32>,
) -> bool {
    if !seen.insert(spell_id) {
        return true;
    }

    let Some(spell_info) = spell_store.get(spell_id) else {
        return false;
    };

    spell_info.effects().iter().all(|effect| {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL {
            return true;
        }
        if effect.effect_trigger_spell <= 0 {
            return false;
        }
        crate::session_rules::represented_spell_valid_with_seen_like_cpp(
            spell_store,
            effect.effect_trigger_spell,
            seen,
        )
    })
}

pub(crate) fn represented_total_avg_equipment_slot_candidates_like_cpp(
    inventory_type: InventoryType,
    can_dual_wield: bool,
    can_titan_grip: bool,
) -> Vec<(u8, bool)> {
    match inventory_type {
        InventoryType::Head => vec![(EQUIPMENT_SLOT_HEAD, false)],
        InventoryType::Neck => vec![(EQUIPMENT_SLOT_NECK, false)],
        InventoryType::Shoulders => vec![(EQUIPMENT_SLOT_SHOULDERS, false)],
        InventoryType::Body => vec![(EQUIPMENT_SLOT_BODY, false)],
        InventoryType::Robe | InventoryType::Chest => vec![(EQUIPMENT_SLOT_CHEST, false)],
        InventoryType::Waist => vec![(EQUIPMENT_SLOT_WAIST, false)],
        InventoryType::Legs => vec![(EQUIPMENT_SLOT_LEGS, false)],
        InventoryType::Feet => vec![(EQUIPMENT_SLOT_FEET, false)],
        InventoryType::Wrists => vec![(EQUIPMENT_SLOT_WRISTS, false)],
        InventoryType::Hands => vec![(EQUIPMENT_SLOT_HANDS, false)],
        InventoryType::Cloak => vec![(EQUIPMENT_SLOT_BACK, false)],
        InventoryType::Finger => {
            vec![
                (EQUIPMENT_SLOT_FINGER1, false),
                (EQUIPMENT_SLOT_FINGER2, true),
            ]
        }
        InventoryType::Trinket => {
            vec![
                (EQUIPMENT_SLOT_TRINKET1, false),
                (EQUIPMENT_SLOT_TRINKET2, true),
            ]
        }
        InventoryType::Weapon => {
            let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
            if can_dual_wield {
                slots.push((EQUIPMENT_SLOT_OFFHAND, true));
            }
            slots
        }
        InventoryType::Weapon2Hand => {
            let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
            if can_dual_wield && can_titan_grip {
                slots.push((EQUIPMENT_SLOT_OFFHAND, true));
            }
            slots
        }
        InventoryType::Ranged | InventoryType::RangedRight | InventoryType::WeaponMainhand => {
            vec![(EQUIPMENT_SLOT_MAINHAND, false)]
        }
        InventoryType::Shield | InventoryType::Holdable | InventoryType::WeaponOffhand => {
            vec![(EQUIPMENT_SLOT_OFFHAND, false)]
        }
        InventoryType::NonEquip
        | InventoryType::Bag
        | InventoryType::Tabard
        | InventoryType::Ammo
        | InventoryType::Thrown
        | InventoryType::Quiver
        | InventoryType::Relic
        | InventoryType::ProfessionTool
        | InventoryType::ProfessionGear
        | InventoryType::EquipableSpellOffensive
        | InventoryType::EquipableSpellUtility
        | InventoryType::EquipableSpellDefensive
        | InventoryType::EquipableSpellMobility => Vec::new(),
    }
}

pub(crate) fn sanitize_rest_bonus_like_cpp(rest_bonus: f32) -> f32 {
    if rest_bonus.is_finite() {
        rest_bonus
    } else {
        0.0
    }
}

pub(crate) const fn ui_link_player_interaction_type_like_cpp(ui_link_type: u32) -> i32 {
    match ui_link_type {
        0 => 54, // PlayerInteractionType::AdventureJournal
        1 => 39, // PlayerInteractionType::ObliterumForge
        2 => 40, // PlayerInteractionType::ScrappingMachine
        3 => 44, // PlayerInteractionType::ItemInteraction
        _ => 0,  // PlayerInteractionType::None
    }
}

pub(crate) fn valid_player_rest_state_like_cpp(rest_state: u8) -> bool {
    matches!(
        rest_state,
        REST_STATE_RESTED_LIKE_CPP | REST_STATE_NORMAL_LIKE_CPP | REST_STATE_RAF_LINKED_LIKE_CPP
    )
}

pub(crate) fn visibility_distance_allows_like_cpp(
    source_position: &Position,
    source_combat_reach: f32,
    target_position: &Position,
    target_combat_reach: f32,
    sight_range: f32,
) -> bool {
    let max_distance = sight_range + source_combat_reach.max(0.0) + target_combat_reach.max(0.0);
    source_position.distance_2d_sq(target_position) < max_distance * max_distance
}

pub(crate) fn void_withdrawal_post_store_item_values_update_like_cpp(
    item: &Item,
    create_dynamic_flags: u32,
) -> Option<ItemValuesUpdate> {
    let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
    let mut has_parent_field = false;
    if !item.data().creator.is_empty() {
        item_data_mask.set(ITEM_DATA_CREATOR_BIT);
        has_parent_field = true;
    }
    if item.data().dynamic_flags != create_dynamic_flags {
        item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS_BIT);
        has_parent_field = true;
    }
    if item.data().property_seed != 0 {
        item_data_mask.set(ITEM_DATA_PROPERTY_SEED_BIT);
        has_parent_field = true;
    }
    if item.data().random_properties_id != 0 {
        item_data_mask.set(ITEM_DATA_RANDOM_PROPERTIES_ID_BIT);
        has_parent_field = true;
    }
    if has_parent_field {
        item_data_mask.set(ITEM_DATA_PARENT_BIT);
    }
    for (index, enchantment) in item.data().enchantments.iter().enumerate() {
        if *enchantment != wow_entities::ItemEnchantment::default() {
            item_data_mask.set(ITEM_DATA_ENCHANTMENT_PARENT_BIT);
            item_data_mask.set(ITEM_DATA_ENCHANTMENT_FIRST_BIT + index);
        }
    }
    if !item_data_mask.is_any_set() {
        return None;
    }
    Some(ItemValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_ITEM,
        object_data: None,
        item_data: Some(ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    })
}

pub(crate) fn xp_in_group_rate_like_cpp(count: u32, is_raid: bool) -> f32 {
    if is_raid {
        0.99
    } else {
        match count {
            0..=2 => 1.0,
            3 => 1.166,
            4 => 1.3,
            _ => 1.4,
        }
    }
}
