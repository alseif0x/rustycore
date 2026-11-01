//! C++ spell-click source condition queries.

use wow_constants::ConditionSourceType;
use wow_data::{
    Condition, ConditionEntriesByTypeStore, ConditionId, NpcSpellClickStoreLikeCpp,
    SPELL_CLICK_USER_FRIEND_LIKE_CPP, SPELL_CLICK_USER_PARTY_LIKE_CPP,
    SPELL_CLICK_USER_RAID_LIKE_CPP, SpellClickInfoLikeCpp, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
};
use wow_entities::WorldObject;

use crate::context::{ConditionSourceInfo, SpellClickRequirementContextLikeCpp};
use crate::evaluate::is_object_meet_to_conditions_like_cpp;

/// C++ `ConditionMgr::IsObjectMeetingSpellClickConditions`.
pub fn is_object_meeting_spell_click_conditions_like_cpp<'a>(
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
    clicker: Option<&'a WorldObject>,
    target: Option<&'a WorldObject>,
    meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if let Some(conditions) = condition_store.conditions_for_like_cpp(
        ConditionSourceType::SpellClickEvent,
        ConditionId::new(creature_id, spell_id as i32, 0),
    ) {
        let mut source_info = ConditionSourceInfo::from_targets(clicker, target, None);
        return is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            meets,
        );
    }

    true
}

/// C++ `ConditionMgr::HasConditionsForSpellClickEvent`.
pub fn has_conditions_for_spell_click_event_like_cpp(
    condition_store: &ConditionEntriesByTypeStore,
    creature_id: u32,
    spell_id: u32,
) -> bool {
    condition_store
        .conditions_for_like_cpp(
            ConditionSourceType::SpellClickEvent,
            ConditionId::new(creature_id, spell_id as i32, 0),
        )
        .is_some()
}

/// C++ `SpellClickInfo::IsFitToRequirements`.
pub fn spell_click_info_is_fit_to_requirements_like_cpp(
    info: &SpellClickInfoLikeCpp,
    context: SpellClickRequirementContextLikeCpp,
) -> bool {
    if !context.clicker_is_player {
        return true;
    }

    match info.user_type {
        SPELL_CLICK_USER_FRIEND_LIKE_CPP => context.clicker_is_friendly_to_summoner,
        SPELL_CLICK_USER_RAID_LIKE_CPP => context.clicker_is_in_raid_with_summoner,
        SPELL_CLICK_USER_PARTY_LIKE_CPP => context.clicker_is_in_party_with_summoner,
        _ => true,
    }
}

/// C++ `Player::CanSeeSpellClickOn`.
pub fn can_see_spell_click_on_like_cpp<'a>(
    spell_click_store: &NpcSpellClickStoreLikeCpp,
    condition_store: &'a ConditionEntriesByTypeStore,
    creature_entry: u32,
    creature_npc_flags: u64,
    clicker: Option<&'a WorldObject>,
    target: Option<&'a WorldObject>,
    requirement_context: SpellClickRequirementContextLikeCpp,
    mut meets: impl FnMut(&'a Condition, &mut ConditionSourceInfo<'a>) -> bool,
) -> bool {
    if (creature_npc_flags & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
        return false;
    }

    let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature_entry);
    if click_bounds.is_empty() {
        return false;
    }

    for click_info in click_bounds {
        if !spell_click_info_is_fit_to_requirements_like_cpp(click_info, requirement_context) {
            return false;
        }

        if is_object_meeting_spell_click_conditions_like_cpp(
            condition_store,
            creature_entry,
            click_info.spell_id,
            clicker,
            target,
            &mut meets,
        ) {
            return true;
        }
    }

    false
}
