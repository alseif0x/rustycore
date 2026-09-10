// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free Session rules, part 1 of 2.
//!
//! Moved out of `impl WorldSession` under #676. Every one was already
//! receiver-free, so it cannot read or write session state: these are rules,
//! not session behaviour. Bodies and signatures are unchanged.

pub(crate) const CR_ARMOR_PENETRATION_LIKE_CPP: u8 = 24;
use crate::session::*;
use std::sync::OnceLock;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use wow_constants::InventoryType;
use wow_constants::ServerOpcodes;
use wow_constants::Stats;
use wow_constants::item::EnchantmentSlot;
use wow_constants::unit::UnitStandStateType;
use wow_core::ObjectGuid;
use wow_core::Position;
use wow_entities::ApplyEnchantmentEffectAction;
use wow_entities::BUYBACK_SLOT_END;
use wow_entities::BUYBACK_SLOT_START;
use wow_entities::EQUIPMENT_SLOT_BACK;
use wow_entities::EQUIPMENT_SLOT_BODY;
use wow_entities::EQUIPMENT_SLOT_CHEST;
use wow_entities::EQUIPMENT_SLOT_FEET;
use wow_entities::EQUIPMENT_SLOT_HANDS;
use wow_entities::EQUIPMENT_SLOT_HEAD;
use wow_entities::EQUIPMENT_SLOT_LEGS;
use wow_entities::EQUIPMENT_SLOT_MAINHAND;
use wow_entities::EQUIPMENT_SLOT_OFFHAND;
use wow_entities::EQUIPMENT_SLOT_SHOULDERS;
use wow_entities::EQUIPMENT_SLOT_TABARD;
use wow_entities::EQUIPMENT_SLOT_WAIST;
use wow_entities::EQUIPMENT_SLOT_WRISTS;
use wow_entities::ITEM_DATA_BITS;
use wow_entities::ITEM_DATA_CONTAINED_IN_BIT;
use wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT;
use wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT;
use wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT;
use wow_entities::ITEM_DATA_PARENT_BIT;
use wow_entities::Item;
use wow_entities::ItemDataUpdate;
use wow_entities::ItemValuesUpdate;
use wow_entities::PhaseShift;
use wow_entities::Player;
use wow_entities::SendNewItemDisplayText;
use wow_entities::SendNewItemPlan;
use wow_entities::TYPEID_ITEM;
use wow_entities::UpdateMask;
use wow_packet::packets::item::ItemInstance;
use wow_packet::packets::item::ItemMod;
use wow_packet::packets::item::ItemModList;
use wow_packet::packets::item::ItemPushResult;
use wow_packet::packets::item::ItemPushResultDisplayType;
use wow_packet::packets::misc::AccountHeirloomUpdate;

pub(crate) fn account_heirloom_update_opcode_resolved_like_cpp() -> bool {
    // The inspected 3.4.3 legacy C++ tree still declares
    // SMSG_ACCOUNT_HEIRLOOM_UPDATE as NULL_OPCODE/0xBADD. Keep the data
    // model ported, but do not send a placeholder opcode to the real client.
    <AccountHeirloomUpdate as wow_packet::ServerPacket>::OPCODE != ServerOpcodes::UpdateCapturePoint
}

pub(crate) const fn account_mount_spells_are_session_dependent_like_cpp() -> bool {
    true
}

pub(crate) fn account_transmog_update_opcode_resolved_like_cpp() -> bool {
    // The inspected 3.4.3 legacy C++ tree still declares
    // SMSG_ACCOUNT_TRANSMOG_UPDATE as NULL_OPCODE/0xBADD. Sending it during
    // login can make the client close the connection after the initial burst.
    <wow_packet::packets::collection::AccountTransmogUpdate as wow_packet::ServerPacket>::OPCODE
        != ServerOpcodes::UpdateCapturePoint
}

pub(crate) fn apply_battle_pet_calculated_stats_like_cpp(
    pet: &mut RepresentedBattlePetDataLikeCpp,
    calculated_stats: Option<RepresentedBattlePetCalculatedStatsLikeCpp>,
) {
    if let Some(calculated_stats) = calculated_stats {
        pet.max_health = calculated_stats.max_health;
        pet.power = calculated_stats.power;
        pet.speed = calculated_stats.speed;
    }
    pet.health = pet.max_health;
}

pub(crate) fn apply_player_session_visibility_detection_like_cpp(
    player: &mut Player,
    never_visible_for_seer: bool,
    seer_can_never_see_target: bool,
) {
    player
        .unit_mut()
        .set_never_visible_for_seer_like_cpp(never_visible_for_seer);
    player
        .unit_mut()
        .set_seer_can_never_see_target_like_cpp(seer_can_never_see_target);
}

pub(crate) fn apply_represented_item_bonus_action_to_state_like_cpp(
    state: &mut RepresentedItemBonusStateLikeCpp,
    action: ApplyEnchantmentEffectAction,
) {
    match action {
        ApplyEnchantmentEffectAction::UnitModifier {
            unit_mod,
            modifier,
            amount,
            apply,
        } => crate::session_rules::apply_represented_unit_modifier_like_cpp(
            state, unit_mod, modifier, amount, apply,
        ),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat) => state.stat_buff_updates.push(stat),
        ApplyEnchantmentEffectAction::RatingModifier {
            rating,
            amount,
            apply,
        } => {
            if let Some(index) = represented_combat_rating_index_like_cpp(rating) {
                apply_represented_i32_delta_like_cpp(
                    &mut state.combat_ratings[index],
                    amount,
                    apply,
                );
            }
        }
        ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply } => {
            apply_represented_i32_delta_like_cpp(&mut state.mana_regen_bonus, amount, apply);
        }
        ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply } => {
            apply_represented_i32_delta_like_cpp(&mut state.spell_power_bonus, amount, apply);
        }
        ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply } => {
            apply_represented_i32_delta_like_cpp(&mut state.health_regen_bonus, amount, apply);
        }
        ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply } => {
            apply_represented_i32_delta_like_cpp(&mut state.spell_penetration_bonus, amount, apply);
        }
        ApplyEnchantmentEffectAction::BaseModFlatValue {
            base_mod: wow_entities::ApplyEnchantmentBaseMod::ShieldBlockValue,
            amount,
            apply,
        } => {
            apply_represented_i32_delta_like_cpp(&mut state.shield_block_base_mod, amount, apply);
        }
        ApplyEnchantmentEffectAction::SetShieldBlockValue { amount } => {
            state.shield_block_value = amount;
        }
        ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound,
            amount_bits,
        } => {
            let attack = attack_type as usize;
            if attack < state.weapon_damage.len() {
                let bound = match bound {
                    wow_entities::WeaponDamageBoundLikeCpp::Min => 0,
                    wow_entities::WeaponDamageBoundLikeCpp::Max => 1,
                };
                state.weapon_damage[attack][bound] = f32::from_bits(amount_bits);
            }
        }
        ApplyEnchantmentEffectAction::SetBaseAttackTime {
            attack_type,
            time_ms,
        } => {
            let attack = attack_type as usize;
            if attack < state.base_attack_time.len() {
                state.base_attack_time[attack] = time_ms;
            }
        }
        ApplyEnchantmentEffectAction::UpdateDamagePhysical { attack_type } => {
            state.damage_physical_updates.push(attack_type)
        }
        ApplyEnchantmentEffectAction::Noop
        | ApplyEnchantmentEffectAction::DeferredCombatSpell
        | ApplyEnchantmentEffectAction::DeferredUseSpell
        | ApplyEnchantmentEffectAction::UpdateDamageDoneMods { .. }
        | ApplyEnchantmentEffectAction::CastEquipSpell { .. }
        | ApplyEnchantmentEffectAction::RemoveEquipSpellAura { .. }
        | ApplyEnchantmentEffectAction::UnhandledStatModifier { .. }
        | ApplyEnchantmentEffectAction::MissingItemTemplateForAttack { .. }
        | ApplyEnchantmentEffectAction::Unknown { .. } => {}
    }
}

#[cfg(test)]
pub(crate) fn apply_represented_pct_modifier_to_u32_like_cpp(value: u32, pct: i32) -> u32 {
    let adjusted = i64::from(value) + (i64::from(value) * i64::from(pct)) / 100;
    adjusted.clamp(0, i64::from(u32::MAX)) as u32
}

pub(crate) fn apply_represented_unit_modifier_like_cpp(
    state: &mut RepresentedItemBonusStateLikeCpp,
    unit_mod: wow_entities::ApplyEnchantmentUnitMod,
    modifier: wow_entities::ApplyEnchantmentUnitModifier,
    amount: u32,
    apply: bool,
) {
    match (unit_mod, modifier) {
        (
            wow_entities::ApplyEnchantmentUnitMod::Mana,
            wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
        ) => apply_represented_i32_delta_like_cpp(&mut state.mana_base, amount, apply),
        (
            wow_entities::ApplyEnchantmentUnitMod::Health,
            wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
        ) => apply_represented_i32_delta_like_cpp(&mut state.health_base, amount, apply),
        (
            wow_entities::ApplyEnchantmentUnitMod::Armor,
            wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
        ) => apply_represented_i32_delta_like_cpp(&mut state.armor_base, amount, apply),
        (
            wow_entities::ApplyEnchantmentUnitMod::Armor,
            wow_entities::ApplyEnchantmentUnitModifier::TotalValue,
        ) => apply_represented_i32_delta_like_cpp(&mut state.armor_total, amount, apply),
        (
            wow_entities::ApplyEnchantmentUnitMod::AttackPower,
            wow_entities::ApplyEnchantmentUnitModifier::TotalValue,
        ) => apply_represented_i32_delta_like_cpp(&mut state.attack_power_total, amount, apply),
        (
            wow_entities::ApplyEnchantmentUnitMod::AttackPowerRanged,
            wow_entities::ApplyEnchantmentUnitModifier::TotalValue,
        ) => apply_represented_i32_delta_like_cpp(
            &mut state.ranged_attack_power_total,
            amount,
            apply,
        ),
        (wow_entities::ApplyEnchantmentUnitMod::Resistance(school), _) => {
            let school = school as usize;
            if school < state.resistances_base.len() {
                apply_represented_i32_delta_like_cpp(
                    &mut state.resistances_base[school],
                    amount,
                    apply,
                );
            }
        }
        (
            unit_mod,
            wow_entities::ApplyEnchantmentUnitModifier::BaseValue
            | wow_entities::ApplyEnchantmentUnitModifier::TotalValue,
        ) => {
            if let Some(index) = represented_unit_mod_stat_index_like_cpp(unit_mod) {
                apply_represented_i32_delta_like_cpp(&mut state.stats_base[index], amount, apply);
            }
        }
    }
}

pub(crate) fn bind_area_id_like_cpp(effect_misc_value: i32, current_area_id: u32) -> u32 {
    if effect_misc_value != 0 {
        // C++ assigns the signed SpellEffectInfo::MiscValue directly to
        // uint32 areaId, preserving the underlying 32-bit value.
        effect_misc_value as u32
    } else {
        current_area_id
    }
}

pub(crate) fn chair_stand_state_like_cpp(chair_height: u32) -> UnitStandStateType {
    let stand_state = 4_u32.saturating_add(chair_height);
    <UnitStandStateType as num_traits::FromPrimitive>::from_u32(stand_state)
        .unwrap_or(UnitStandStateType::Stand)
}

pub(crate) fn creature_message_to_set_target_allows_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    source_is_visible_like_cpp: bool,
    player_map_id: u32,
    player_instance_id: u32,
    player_position: &Position,
    player_phase_shift: &PhaseShift,
    required_3d: bool,
) -> bool {
    if !source_is_visible_like_cpp {
        return false;
    }
    if creature.map_id() != player_map_id || creature.instance_id() != player_instance_id {
        return false;
    }
    if !player_phase_shift.can_see(creature.phase_shift()) {
        return false;
    }

    let range = creature.visibility_range_like_cpp();
    if required_3d {
        position_is_in_dist_strict_3d_like_cpp(&creature.position(), player_position, range)
    } else {
        position_is_in_dist_strict_2d_like_cpp(&creature.position(), player_position, range)
    }
}

pub(crate) fn creature_movement_spline_speed_opcode_like_cpp(
    move_type: UnitMoveTypeLikeCpp,
) -> Option<ServerOpcodes> {
    match move_type {
        UnitMoveTypeLikeCpp::Walk => Some(ServerOpcodes::MoveSplineSetWalkSpeed),
        UnitMoveTypeLikeCpp::Run => Some(ServerOpcodes::MoveSplineSetRunSpeed),
        UnitMoveTypeLikeCpp::RunBack => Some(ServerOpcodes::MoveSplineSetRunBackSpeed),
        UnitMoveTypeLikeCpp::Swim => Some(ServerOpcodes::MoveSplineSetSwimSpeed),
        UnitMoveTypeLikeCpp::SwimBack => Some(ServerOpcodes::MoveSplineSetSwimBackSpeed),
        UnitMoveTypeLikeCpp::TurnRate => Some(ServerOpcodes::MoveSplineSetTurnRate),
        UnitMoveTypeLikeCpp::Flight => Some(ServerOpcodes::MoveSplineSetFlightSpeed),
        UnitMoveTypeLikeCpp::FlightBack => Some(ServerOpcodes::MoveSplineSetFlightBackSpeed),
        UnitMoveTypeLikeCpp::PitchRate => Some(ServerOpcodes::MoveSplineSetPitchRate),
    }
}

pub(crate) fn current_game_time_secs_like_cpp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn dynamic_object_create_data_from_canonical_like_cpp(
    guid: ObjectGuid,
    dynamic_object: &wow_entities::DynamicObject,
) -> wow_packet::packets::update::DynamicObjectCreateData {
    let object = dynamic_object.world();
    let object_data = object.object().object_data_values();
    let data = dynamic_object.data();
    wow_packet::packets::update::DynamicObjectCreateData {
        guid,
        entry_id: u32::try_from(object_data.entry_id).unwrap_or(0),
        dynamic_flags: object_data.dynamic_flags,
        scale: object_data.scale,
        position: object.position(),
        caster: data.caster,
        dynamic_object_type: data.dynamic_object_type,
        spell_visual_id: data.spell_visual_id,
        spell_id: data.spell_id,
        radius: data.radius,
        cast_time_ms: data.cast_time_ms,
    }
}

/// Monotonic millisecond counter matching TrinityCore's `getMSTime()` scale.
pub(crate) fn game_time_ms_like_cpp() -> u32 {
    static SERVER_START: OnceLock<Instant> = OnceLock::new();
    let start = SERVER_START.get_or_init(Instant::now);
    start.elapsed().as_millis() as u32
}

pub(crate) fn ignored_equipment_set_item_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64)
}

pub(crate) fn is_buyback_slot(slot: u8) -> bool {
    (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
}

#[cfg(test)]
pub(crate) fn is_non_durable_skill_tombstone_like_cpp(
    skill: &RepresentedPlayerSkillLikeCpp,
) -> bool {
    skill.step == 0
        && skill.value == 0
        && skill.max == 0
        && skill.profession_slot == -1
        && matches!(
            skill.state,
            RepresentedPlayerSkillStateLikeCpp::Unchanged
                | RepresentedPlayerSkillStateLikeCpp::Deleted
        )
}

/// C++ `sItemPriceBaseStore.LookupEntry(itemLevel)`.
pub(crate) fn item_price_base_with_catalogs_like_cpp(
    catalogs: &ItemValuationCatalogsLikeCpp,
    item_level: u32,
) -> Option<(f32, f32)> {
    catalogs
        .price_base
        .get(item_level)
        .map(|entry| (entry.armor, entry.weapon))
}

pub(crate) fn item_push_result_from_send_new_item_plan(plan: &SendNewItemPlan) -> ItemPushResult {
    ItemPushResult {
        player_guid: plan.player_guid,
        slot: plan.slot,
        slot_in_bag: i32::from(plan.slot_in_bag),
        item: ItemInstance {
            item_id: plan.item_instance.item_id as i32,
            random_properties_seed: plan.item_instance.random_properties_seed,
            random_properties_id: plan.item_instance.random_properties_id,
            item_bonus: None,
            modifications: ItemModList {
                values: plan
                    .item_instance
                    .modifications
                    .iter()
                    .map(|modifier| ItemMod::new(modifier.value, modifier.modifier_type))
                    .collect(),
            },
        },
        quest_log_item_id: plan.quest_log_item_id as i32,
        quantity: plan.quantity as i32,
        quantity_in_inventory: plan.quantity_in_inventory as i32,
        dungeon_encounter_id: plan.dungeon_encounter_id as i32,
        battle_pet_species_id: plan.battle_pet_species_id as i32,
        battle_pet_breed_id: plan.battle_pet_breed_id as i32,
        battle_pet_breed_quality: u32::from(plan.battle_pet_breed_quality),
        battle_pet_level: plan.battle_pet_level as i32,
        item_guid: plan.item_guid,
        pushed: plan.pushed,
        display_text: match plan.display_text {
            SendNewItemDisplayText::Normal => ItemPushResultDisplayType::Normal,
            SendNewItemDisplayText::EncounterLoot => ItemPushResultDisplayType::EncounterLoot,
            SendNewItemDisplayText::QuestUpdateAddItem => {
                ItemPushResultDisplayType::QuestUpdateAddItem
            }
        },
        created: plan.created,
        is_bonus_roll: false,
        is_encounter_loot: plan.is_encounter_loot,
    }
}

pub(crate) fn item_storage_fields_values_update_like_cpp(
    item: &Item,
    contained_in_changed: bool,
    dynamic_flags2_changed: bool,
    changed_enchantments: &[EnchantmentSlot],
) -> ItemValuesUpdate {
    let mut item_data_mask = UpdateMask::new(ITEM_DATA_BITS);
    if contained_in_changed || dynamic_flags2_changed {
        item_data_mask.set(ITEM_DATA_PARENT_BIT);
    }
    if contained_in_changed {
        item_data_mask.set(ITEM_DATA_CONTAINED_IN_BIT);
    }
    if dynamic_flags2_changed {
        item_data_mask.set(ITEM_DATA_DYNAMIC_FLAGS2_BIT);
    }
    if !changed_enchantments.is_empty() {
        item_data_mask.set(ITEM_DATA_ENCHANTMENT_PARENT_BIT);
        for slot in changed_enchantments {
            item_data_mask.set(ITEM_DATA_ENCHANTMENT_FIRST_BIT + *slot as usize);
        }
    }
    ItemValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_ITEM,
        object_data: None,
        item_data: Some(ItemDataUpdate {
            mask: item_data_mask,
            values: item.data().clone(),
        }),
    }
}

/// C++ `ItemTransmogrificationSlots`.
pub(crate) fn item_transmogrification_slot_like_cpp(inventory_type: u8) -> Option<usize> {
    let slot = match inventory_type {
        x if x == InventoryType::Head as u8 => EQUIPMENT_SLOT_HEAD,
        x if x == InventoryType::Shoulders as u8 => EQUIPMENT_SLOT_SHOULDERS,
        x if x == InventoryType::Body as u8 => EQUIPMENT_SLOT_BODY,
        x if x == InventoryType::Chest as u8 => EQUIPMENT_SLOT_CHEST,
        x if x == InventoryType::Waist as u8 => EQUIPMENT_SLOT_WAIST,
        x if x == InventoryType::Legs as u8 => EQUIPMENT_SLOT_LEGS,
        x if x == InventoryType::Feet as u8 => EQUIPMENT_SLOT_FEET,
        x if x == InventoryType::Wrists as u8 => EQUIPMENT_SLOT_WRISTS,
        x if x == InventoryType::Hands as u8 => EQUIPMENT_SLOT_HANDS,
        x if x == InventoryType::Weapon as u8
            || x == InventoryType::Ranged as u8
            || x == InventoryType::Weapon2Hand as u8
            || x == InventoryType::WeaponMainhand as u8
            || x == InventoryType::WeaponOffhand as u8
            || x == InventoryType::RangedRight as u8 =>
        {
            EQUIPMENT_SLOT_MAINHAND
        }
        x if x == InventoryType::Shield as u8 || x == InventoryType::Holdable as u8 => {
            EQUIPMENT_SLOT_OFFHAND
        }
        x if x == InventoryType::Cloak as u8 => EQUIPMENT_SLOT_BACK,
        x if x == InventoryType::Tabard as u8 => EQUIPMENT_SLOT_TABARD,
        x if x == InventoryType::Robe as u8 => EQUIPMENT_SLOT_CHEST,
        _ => return None,
    };

    Some(slot as usize)
}

pub(crate) fn loaded_enchantment_effect_action_is_unrepresented_like_cpp(
    action: ApplyEnchantmentEffectAction,
) -> bool {
    matches!(
        action,
        ApplyEnchantmentEffectAction::DeferredCombatSpell
            | ApplyEnchantmentEffectAction::DeferredUseSpell
            | ApplyEnchantmentEffectAction::UpdateDamageDoneMods { .. }
            | ApplyEnchantmentEffectAction::CastEquipSpell { .. }
            | ApplyEnchantmentEffectAction::RemoveEquipSpellAura { .. }
            | ApplyEnchantmentEffectAction::UnhandledStatModifier { .. }
            | ApplyEnchantmentEffectAction::MissingItemTemplateForAttack { .. }
            | ApplyEnchantmentEffectAction::Unknown { .. }
    )
}

pub(crate) fn apply_represented_i32_delta_like_cpp(target: &mut i32, amount: u32, apply: bool) {
    let amount = i32::try_from(amount).unwrap_or(i32::MAX);
    if apply {
        *target = target.saturating_add(amount);
    } else {
        *target = target.saturating_sub(amount);
    }
}

pub(crate) fn represented_unit_mod_stat_index_like_cpp(
    unit_mod: wow_entities::ApplyEnchantmentUnitMod,
) -> Option<usize> {
    match unit_mod {
        wow_entities::ApplyEnchantmentUnitMod::StatStrength => Some(Stats::Strength as usize),
        wow_entities::ApplyEnchantmentUnitMod::StatAgility => Some(Stats::Agility as usize),
        wow_entities::ApplyEnchantmentUnitMod::StatStamina => Some(Stats::Stamina as usize),
        wow_entities::ApplyEnchantmentUnitMod::StatIntellect => Some(Stats::Intellect as usize),
        wow_entities::ApplyEnchantmentUnitMod::StatSpirit => Some(Stats::Spirit as usize),
        _ => None,
    }
}

pub(crate) fn represented_combat_rating_index_like_cpp(
    rating: wow_entities::ApplyEnchantmentCombatRating,
) -> Option<usize> {
    match rating {
        wow_entities::ApplyEnchantmentCombatRating::DefenseSkill => Some(1),
        wow_entities::ApplyEnchantmentCombatRating::Dodge => Some(2),
        wow_entities::ApplyEnchantmentCombatRating::Parry => Some(3),
        wow_entities::ApplyEnchantmentCombatRating::Block => Some(4),
        wow_entities::ApplyEnchantmentCombatRating::HitMelee => Some(5),
        wow_entities::ApplyEnchantmentCombatRating::HitRanged => Some(6),
        wow_entities::ApplyEnchantmentCombatRating::HitSpell => Some(7),
        wow_entities::ApplyEnchantmentCombatRating::CritMelee => Some(8),
        wow_entities::ApplyEnchantmentCombatRating::CritRanged => Some(9),
        wow_entities::ApplyEnchantmentCombatRating::CritSpell => Some(10),
        wow_entities::ApplyEnchantmentCombatRating::HasteMelee => Some(17),
        wow_entities::ApplyEnchantmentCombatRating::HasteRanged => Some(18),
        wow_entities::ApplyEnchantmentCombatRating::HasteSpell => Some(19),
        wow_entities::ApplyEnchantmentCombatRating::Expertise => Some(23),
        wow_entities::ApplyEnchantmentCombatRating::ArmorPenetration => {
            Some(CR_ARMOR_PENETRATION_LIKE_CPP as usize)
        }
    }
}

pub(crate) fn position_is_in_dist_strict_3d_like_cpp(
    position: &Position,
    other: &Position,
    dist: f32,
) -> bool {
    position.distance_sq(other) < dist * dist
}

pub(crate) fn position_is_in_dist_strict_2d_like_cpp(
    position: &Position,
    other: &Position,
    dist: f32,
) -> bool {
    position.distance_2d_sq(other) < dist * dist
}
