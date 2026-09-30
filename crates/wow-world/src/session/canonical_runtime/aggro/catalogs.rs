//! Application-owned faction/reputation and DB2 lookups; no Actor callback.
use super::*;

pub(in crate::session) fn with_policies<R>(config: &LegacyCreatureAggroConfigLikeCpp,
    apply: impl FnOnce(&mut AggroPolicies<'_>) -> R) -> R {
    let mut hostility = |source: i32, target: AggroFactionTarget<'_>| match target {
        AggroFactionTarget::Player(candidate) => player_hostile(source, candidate, config),
        AggroFactionTarget::Unit(faction) => unit_hostile(source, faction, config),
    };
    let mut select_ai = |facts: AggroAiFacts<'_>| match select_kind(facts, config) {
        LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented => AggroAiSelection::Unrepresented,
        LegacyCreatureAiSelectionDecisionLikeCpp::Selected(kind) => AggroAiSelection::Selected(
            if !creature_ai_uses_base_move_in_line_of_sight_like_cpp(&kind) { AggroAiKind::NoBaseLos }
            else if matches!(kind, CreatureAiKindLikeCpp::TurretAI) { AggroAiKind::Turret }
            else { AggroAiKind::Base }),
    };
    let mut can_attack = |facts: AggroTurretFacts| turret_decision(facts, config);
    let mut attack_distance = |facts: AggroDistanceFacts| creature_attack_distance_like_cpp(CreatureAttackDistanceInputLikeCpp {
        aggro_rate: facts.aggro_rate, creature_combat_reach: facts.creature_combat_reach,
        expansion_max_level: max_level_for_expansion_like_cpp(facts.required_expansion),
        max_player_level_config: facts.max_player_level_config,
        player_level_for_target: facts.player_level_for_target, creature_level_for_target: facts.creature_level_for_target,
        creature_detect_range_aura_mod: facts.creature_detect_range_aura_mod,
        player_detected_range_aura_mod: facts.player_detected_range_aura_mod,
    });
    apply(&mut AggroPolicies { hostility: &mut hostility, select_ai: &mut select_ai,
        can_attack: &mut can_attack, attack_distance: &mut attack_distance })
}

fn has_reputation_state(
    candidate: &AggroCandidate,
    faction_id: u32,
) -> bool {
    candidate
        .player_reputation_state_flags
        .iter()
        .any(|(candidate_faction_id, _)| *candidate_faction_id == faction_id)
}
fn is_at_war(
    candidate: &AggroCandidate,
    faction_id: u32,
) -> bool {
    candidate
        .player_reputation_state_flags
        .iter()
        .find_map(|(candidate_faction_id, flags)| {
            (*candidate_faction_id == faction_id).then_some(*flags)
        })
        .is_some_and(|flags| flags & wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP != 0)
}
fn reputation_standing(
    candidate: &AggroCandidate,
    faction_id: u32,
) -> i32 {
    candidate
        .player_reputation_standings
        .iter()
        .find_map(|(candidate_faction_id, standing)| {
            (*candidate_faction_id == faction_id).then_some(*standing)
        })
        .unwrap_or(0)
}
fn forced_reputation_rank(
    candidate: &AggroCandidate,
    faction_id: u32,
) -> Option<wow_data::reputation::ReputationRankLikeCpp> {
    candidate
        .player_forced_reputation_ranks
        .iter()
        .find_map(|(candidate_faction_id, rank)| {
            (*candidate_faction_id == faction_id).then_some(*rank)
        })
}
fn player_hostile(
    source_faction: i32,
    candidate: &AggroCandidate,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<bool> {
    let faction_template_store = config.faction_template_store.as_ref()?;
    let creature_faction_template_id =
        source_faction.max(0) as u32;
    if creature_faction_template_id == 0 || candidate.player_faction_template_id == 0 {
        return Some(false);
    }

    let creature_faction_template = faction_template_store.get(creature_faction_template_id)?;
    let player_faction_template =
        faction_template_store.get(candidate.player_faction_template_id)?;

    if creature_faction_template.is_contested_guard_faction_like_cpp()
        && candidate.player_is_contested_pvp
    {
        return Some(true);
    }

    let creature_faction_id = u32::from(creature_faction_template.faction);
    if creature_faction_id != 0 {
        if let Some(forced_rank) =
            forced_reputation_rank(
                candidate,
                creature_faction_id,
            )
        {
            return Some(forced_rank <= wow_data::reputation::ReputationRankLikeCpp::Hostile);
        }
        if candidate
            .player_forced_reputation_faction_ids
            .contains(&creature_faction_id)
        {
            return None;
        }

        let player_flags2 = UnitFlags2::from_bits_truncate(candidate.player_unit_flags2);
        if !player_flags2.contains(UnitFlags2::IGNORE_REPUTATION)
            && let Some(faction_store) = config.faction_store.as_ref()
            && let Some(faction_entry) = faction_store.get(creature_faction_id)
            && faction_entry.can_have_reputation_like_cpp()
            && has_reputation_state(
                candidate,
                creature_faction_id,
            )
        {
            if !is_at_war(candidate, creature_faction_id) {
                return Some(false);
            }

            let rank = wow_data::reputation::reputation_rank_from_standing_like_cpp(
                reputation_standing(
                    candidate,
                    creature_faction_id,
                ),
            );
            // C++ `GetFactionReactionTo` caps an at-war player reaction to at
            // most neutral; `Creature::_IsTargetAcceptable` still requires an
            // actually hostile reaction to start aggro.
            return Some(rank <= wow_data::reputation::ReputationRankLikeCpp::Hostile);
        }
    }

    if creature_faction_template.is_hostile_to_like_cpp(player_faction_template) {
        return Some(true);
    }
    if creature_faction_template.is_friendly_to_like_cpp(player_faction_template) {
        return Some(false);
    }
    if player_faction_template.is_friendly_to_like_cpp(creature_faction_template) {
        return Some(false);
    }
    if creature_faction_template.is_hostile_by_default_like_cpp() {
        return Some(true);
    }
    Some(false)
}
fn unit_hostile(
    source_faction: i32,
    target_faction: Option<u32>,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<bool> {
    let faction_templates = config.faction_template_store.as_ref()?;
    let creature_faction = faction_templates
        .get(u32::try_from(source_faction).ok()?)?;
    let target_faction = faction_templates.get(target_faction?)?;

    if creature_faction.is_hostile_to_like_cpp(target_faction) {
        return Some(true);
    }
    if creature_faction.is_friendly_to_like_cpp(target_faction)
        || target_faction.is_friendly_to_like_cpp(creature_faction)
    {
        return Some(false);
    }
    Some(creature_faction.is_hostile_by_default_like_cpp())
}

pub(in crate::session) fn select_kind(facts: AggroAiFacts<'_>, config: &LegacyCreatureAggroConfigLikeCpp)
    -> LegacyCreatureAiSelectionDecisionLikeCpp {
    // Pet override precedes the script registry gate, as before.
    if !facts.is_pet && !facts.script_name.is_empty() {
        return LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented;
    }
    let flags_extra = CreatureFlagsExtra::from_bits_truncate(facts.flags_extra);
    let input = CreatureAiSelectionInputLikeCpp {
        ai_name: facts.ai_name.to_owned(), script_name: facts.script_name.to_owned(),
        script_can_create_creature_ai: false, is_pet: facts.is_pet,
        is_vehicle: facts.is_vehicle, is_totem: facts.is_totem,
        is_trigger: flags_extra.contains(CreatureFlagsExtra::TRIGGER), first_spell_id: facts.first_spell_id,
        is_critter: facts.creature_type == CreatureType::Critter as u32,
        is_guardian: facts.is_guardian, is_guard: flags_extra.contains(CreatureFlagsExtra::GUARD),
        is_civilian: facts.is_civilian,
        is_neutral_to_all: config.creature_faction_template_is_neutral_to_all_like_cpp(facts.faction_template.max(0) as u32),
        has_spellclick_npc_flag: NPCFlags1::from_bits_truncate(facts.npc_flags).contains(NPCFlags1::SPELL_CLICK),
        is_controllable_guardian: facts.is_controllable_guardian,
        controllable_guardian_owner_is_player: facts.owner_is_player,
    };
    LegacyCreatureAiSelectionDecisionLikeCpp::Selected(select_creature_ai_like_cpp(&input))
}

pub(in crate::session) fn turret_decision(facts: AggroTurretFacts, config: &LegacyCreatureAggroConfigLikeCpp) -> AggroAttackDecision {
    if !matches!(facts.kind, AggroAiKind::Turret) { return AggroAttackDecision::Allowed; }
    let Some(misc_store) = config.spell_misc_store.as_ref() else { return AggroAttackDecision::Unrepresented; };
    let Some(range_store) = config.spell_range_store.as_ref() else { return AggroAttackDecision::Unrepresented; };
    // `TurretAI` caches GetMin/MaxRange from the active map difficulty in
    // its constructor (CombatAI.cpp:196-199).
    let Some(misc) = misc_store.entry_for_spell_difficulty_with_fallback_like_cpp(
        facts.first_spell_id, facts.difficulty, config.difficulty_store.as_deref())
        else { return AggroAttackDecision::Unrepresented; };
    let Some(range) = range_store.get(u32::from(misc.range_index)) else { return AggroAttackDecision::Unrepresented; };
    // `Unit::IsWithinCombatRange` compares squared center distance with
    // `(requested range + both combat reaches)^2` using strict `<`.
    let distance_sq = facts.position.distance_sq(&facts.target_position);
    let reach_sum = facts.combat_reach + facts.target_combat_reach.max(0.0);
    let combat_distance = range.range_max[0] + reach_sum;
    let minimum_range = range.range_min[0];
    let input = CreatureAiCanAttackInputLikeCpp {
        target_within_turret_combat_range: distance_sq < combat_distance * combat_distance,
        target_within_turret_min_range: minimum_range != 0.0
            && distance_sq < (minimum_range + reach_sum) * (minimum_range + reach_sum),
        ..CreatureAiCanAttackInputLikeCpp::default()
    };
    if creature_ai_can_attack_like_cpp(&CreatureAiKindLikeCpp::TurretAI, &input) {
        AggroAttackDecision::Allowed
    } else { AggroAttackDecision::Rejected }
}
