//! Evaluate packets.
//!
//! Separated from player_condition.rs under #691.

use super::*;

pub fn player_condition_compare_like_cpp(comparison_type: i32, value1: i32, value2: i32) -> bool {
    match comparison_type {
        1 => value1 == value2,
        2 => value1 != value2,
        3 => value1 > value2,
        4 => value1 >= value2,
        5 => value1 < value2,
        6 => value1 <= value2,
        _ => false,
    }
}

pub fn player_condition_logic_like_cpp<const N: usize>(logic: u32, mut results: [bool; N]) -> bool {
    debug_assert!(N < 16);
    for (i, result) in results.iter_mut().enumerate() {
        if ((logic >> (16 + i)) & 1) != 0 {
            *result = !*result;
        }
    }

    let mut result = results[0];
    for (i, value) in results.iter().enumerate().skip(1) {
        match (logic >> (2 * (i - 1))) & 3 {
            1 => result = result && *value,
            2 => result = result || *value,
            _ => {}
        }
    }
    result
}

pub fn is_player_meeting_condition_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.race_mask != 0
        && (condition.race_mask & (1i64 << (context.race.saturating_sub(1)))) == 0
    {
        return false;
    }
    if condition.class_mask != 0 && (context.class_mask & condition.class_mask as u32) == 0 {
        return false;
    }
    if condition.gender >= 0 && context.gender != condition.gender as u8 {
        return false;
    }
    if condition.native_gender >= 0 && context.native_gender != condition.native_gender as u8 {
        return false;
    }
    if condition.power_type != -1 && condition.power_type_comp != 0 {
        let required = if (condition.flags & 4) != 0 {
            context.max_power
        } else {
            i32::from(condition.power_type_value)
        };
        if condition.power_type != context.power_type
            || !player_condition_compare_like_cpp(
                i32::from(condition.power_type_comp),
                context.power,
                required,
            )
        {
            return false;
        }
    }

    if (condition.chr_specialization_index >= 0 || condition.chr_specialization_role >= 0)
        && let (Some(spec_id), Some(store)) = (
            context.primary_specialization_id,
            context.chr_specializations,
        )
        && let Some(spec) = store.get(spec_id)
    {
        if condition.chr_specialization_index >= 0
            && spec.order_index != condition.chr_specialization_index
        {
            return false;
        }
        if condition.chr_specialization_role >= 0 && spec.role != condition.chr_specialization_role
        {
            return false;
        }
    }

    if condition.skill_id.iter().any(|id| *id != 0) {
        let mut results = [true; 4];
        for (i, id) in condition.skill_id.iter().enumerate() {
            if *id != 0 {
                let value = skill_value(context, *id);
                results[i] =
                    value != 0 && value > condition.min_skill[i] && value < condition.max_skill[i];
            }
        }
        if !player_condition_logic_like_cpp(condition.skill_logic, results) {
            return false;
        }
    }

    if condition.language_id != 0 {
        if condition.min_language != 0 && context.language_skill < i32::from(condition.min_language)
        {
            return false;
        }
        if condition.max_language != 0 && context.language_skill > condition.max_language {
            return false;
        }
    }

    if !check_reputation_like_cpp(condition, context) {
        return false;
    }
    if condition.current_pvp_faction != 0
        && condition.current_pvp_faction - 1 != context.current_pvp_faction
    {
        return false;
    }
    if condition.pvp_medal != 0
        && ((1u32 << (condition.pvp_medal - 1)) & context.pvp_medals_mask) == 0
    {
        return false;
    }
    if condition.lifetime_max_pvp_rank != 0
        && context.lifetime_max_pvp_rank != condition.lifetime_max_pvp_rank
    {
        return false;
    }
    if condition.movement_flags[0] != 0
        && (context.movement_flags[0] & condition.movement_flags[0]) == 0
    {
        return false;
    }
    if condition.movement_flags[1] != 0
        && (context.movement_flags[1] & condition.movement_flags[1]) == 0
    {
        return false;
    }
    if condition.weapon_subclass_mask != 0 {
        let Some(subclass) = context.mainhand_weapon_subclass else {
            return false;
        };
        if ((1i32 << subclass) & condition.weapon_subclass_mask) == 0 {
            return false;
        }
    }

    if !check_party_status_like_cpp(condition.party_status, context.party_status) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.prev_quest_id,
        condition.prev_quest_logic,
        context.completed_quests,
    ) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.curr_quest_id,
        condition.curr_quest_logic,
        context.current_quests,
    ) {
        return false;
    }
    if !check_id_array4_like_cpp(
        condition.current_completed_quest_id,
        condition.current_completed_quest_logic,
        context.complete_quests,
    ) {
        return false;
    }
    if !check_signed_id_array4_like_cpp(condition.spell_id, condition.spell_logic, context.spells) {
        return false;
    }
    if !check_count_array4_like_cpp(
        condition.item_id,
        condition.item_count,
        condition.item_logic,
        context.items,
    ) {
        return false;
    }
    if !check_currency_array_like_cpp(condition, context) {
        return false;
    }
    if condition.explored.iter().any(|id| *id != 0)
        && condition
            .explored
            .iter()
            .filter(|id| **id != 0)
            .any(|id| !context.explored_area_ids.contains(id))
    {
        return false;
    }
    if !check_auras_like_cpp(condition, context) {
        return false;
    }
    if condition.world_state_expression_id != 0 {
        let Some(store) = context.world_state_expressions else {
            return false;
        };
        let Some(entry) = store.get(u32::from(condition.world_state_expression_id)) else {
            return false;
        };
        let Some(wse_context) = context.world_state_expression_context.as_ref() else {
            return false;
        };
        if !is_meeting_world_state_expression_like_cpp(entry, wse_context) {
            return false;
        }
    }
    if condition.weather_id != 0 && context.weather_id != condition.weather_id {
        return false;
    }
    if !check_u16_array4_like_cpp(
        condition.achievement,
        condition.achievement_logic,
        context.achievements,
    ) {
        return false;
    }
    if !check_lfg_like_cpp(condition, context) {
        return false;
    }
    if !check_area_like_cpp(condition, context) {
        return false;
    }
    if condition.min_expansion_level != -1 && context.expansion < condition.min_expansion_level {
        return false;
    }
    if condition.max_expansion_level != -1 && context.expansion > condition.max_expansion_level {
        return false;
    }
    if condition.min_expansion_level != -1
        && condition.min_expansion_tier != -1
        && !context.is_game_master
        && ((condition.min_expansion_level == context.server_expansion
            && condition.min_expansion_tier > 0)
            || condition.min_expansion_level > context.server_expansion)
    {
        return false;
    }
    if (condition.phase_id != 0 || condition.phase_group_id != 0 || condition.phase_use_flags != 0)
        && !context.phase_satisfied
    {
        return false;
    }
    if !check_quest_kills_like_cpp(condition, context) {
        return false;
    }
    if condition.min_avg_item_level != 0
        && (context.avg_item_level.floor() as i32) < condition.min_avg_item_level
    {
        return false;
    }
    if condition.max_avg_item_level != 0
        && context.avg_item_level.floor() as i32 > condition.max_avg_item_level
    {
        return false;
    }
    if condition.min_avg_equipped_item_level != 0
        && (context.avg_equipped_item_level.floor() as u32)
            < u32::from(condition.min_avg_equipped_item_level)
    {
        return false;
    }
    if condition.max_avg_equipped_item_level != 0
        && context.avg_equipped_item_level.floor() as u32
            > u32::from(condition.max_avg_equipped_item_level)
    {
        return false;
    }
    if condition.modifier_tree_id != 0
        && !context
            .modifier_tree_ids
            .contains(&condition.modifier_tree_id)
    {
        return false;
    }

    true
}

fn skill_value(context: &PlayerConditionContextLikeCpp<'_>, id: u16) -> u16 {
    context
        .skills
        .iter()
        .find(|skill| skill.id == id)
        .map(|skill| skill.value)
        .unwrap_or(0)
}

fn reputation_rank(context: &PlayerConditionContextLikeCpp<'_>, id: u32) -> u8 {
    context
        .reputations
        .iter()
        .find(|rep| rep.faction_id == id)
        .map(|rep| rep.rank)
        .unwrap_or(0)
}

fn count_for(values: &[PlayerConditionCountLikeCpp], id: u32) -> u32 {
    values
        .iter()
        .find(|value| value.id == id)
        .map(|value| value.count)
        .unwrap_or(0)
}

fn check_reputation_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.min_faction_id.iter().all(|id| *id == 0) && condition.max_faction_id == 0 {
        return true;
    }
    if condition.min_faction_id.iter().all(|id| *id == 0) {
        return reputation_rank(context, u32::from(condition.max_faction_id))
            <= condition.max_reputation;
    }

    let mut results = [true; 4];
    for (i, id) in condition.min_faction_id.iter().enumerate() {
        if *id != 0 {
            results[i] = reputation_rank(context, *id) >= condition.min_reputation[i];
        }
    }
    if condition.max_faction_id != 0 {
        results[3] = reputation_rank(context, u32::from(condition.max_faction_id))
            <= condition.max_reputation;
    }
    player_condition_logic_like_cpp(condition.reputation_logic, results)
}

fn check_party_status_like_cpp(required: u8, current: PlayerConditionPartyStatusLikeCpp) -> bool {
    match required {
        0 => true,
        1 => current == PlayerConditionPartyStatusLikeCpp::Solo,
        2 => current != PlayerConditionPartyStatusLikeCpp::Solo,
        3 => current == PlayerConditionPartyStatusLikeCpp::InParty,
        4 => current == PlayerConditionPartyStatusLikeCpp::InRaid,
        5 => current != PlayerConditionPartyStatusLikeCpp::InRaid,
        _ => true,
    }
}

fn check_id_array4_like_cpp(ids: [u32; 4], logic: u32, owned: &[u32]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = owned.contains(id);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_signed_id_array4_like_cpp(ids: [i32; 4], logic: u32, owned: &[u32]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .map(|id| owned.contains(&id))
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_count_array4_like_cpp(
    ids: [i32; 4],
    counts: [u32; 4],
    logic: u32,
    owned: &[PlayerConditionCountLikeCpp],
) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .map(|id| count_for(owned, id) >= counts[i])
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_currency_array_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.currency_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.currency_id.iter().enumerate() {
        if *id != 0 {
            results[i] = count_for(context.currencies, *id) >= condition.currency_count[i];
        }
    }
    player_condition_logic_like_cpp(condition.currency_logic, results)
}

fn check_auras_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.aura_spell_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.aura_spell_id.iter().enumerate() {
        if *id != 0 {
            results[i] = u32::try_from(*id)
                .ok()
                .and_then(|id| context.auras.iter().find(|aura| aura.spell_id == id))
                .map(|aura| {
                    condition.aura_stacks[i] == 0 || aura.stacks >= condition.aura_stacks[i]
                })
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(condition.aura_spell_logic, results)
}

fn check_u16_array4_like_cpp(ids: [u16; 4], logic: u32, owned: &[u16]) -> bool {
    if ids[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            results[i] = owned.contains(id);
        }
    }
    player_condition_logic_like_cpp(logic, results)
}

fn check_lfg_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.lfg_status[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, status) in condition.lfg_status.iter().enumerate() {
        if *status != 0 {
            results[i] = player_condition_compare_like_cpp(
                i32::from(condition.lfg_compare[i]),
                count_for(context.lfg_values, u32::from(*status)) as i32,
                condition.lfg_value[i] as i32,
            );
        }
    }
    player_condition_logic_like_cpp(condition.lfg_logic, results)
}

fn check_area_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.area_id[0] == 0 {
        return true;
    }
    let mut results = [true; 4];
    for (i, id) in condition.area_id.iter().enumerate() {
        if *id != 0 {
            let area = u32::from(*id);
            results[i] = context.area_id == area || context.parent_area_ids.contains(&area);
        }
    }
    player_condition_logic_like_cpp(condition.area_logic, results)
}

fn check_quest_kills_like_cpp(
    condition: &PlayerConditionEntry,
    context: &PlayerConditionContextLikeCpp<'_>,
) -> bool {
    if condition.quest_kill_id == 0
        || context.quest_kill_id != condition.quest_kill_id
        || condition.quest_kill_monster.iter().all(|id| *id == 0)
    {
        return true;
    }
    let mut results = [true; 6];
    for (i, id) in condition.quest_kill_monster.iter().enumerate() {
        if *id != 0 {
            results[i] = context
                .quest_kills
                .iter()
                .find(|kill| kill.monster_id == *id)
                .map(|kill| kill.done)
                .unwrap_or(false);
        }
    }
    player_condition_logic_like_cpp(condition.quest_kill_logic, results)
}
