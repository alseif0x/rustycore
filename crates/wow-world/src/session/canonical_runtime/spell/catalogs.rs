//! Application catalogs remain lazy at the original operation-specific gates.
use super::*;

pub(in crate::session) fn with_policies<R>(config: &LegacyCreatureAggroConfigLikeCpp,
    apply: impl FnOnce(&mut SpellPolicies<'_>) -> R) -> R {
    let mut select_ai = |facts: wow_map::map_manager::AggroAiFacts<'_>| match super::super::aggro::catalogs::select_kind(facts, config) {
        LegacyCreatureAiSelectionDecisionLikeCpp::ScriptRegistryUnrepresented => SpellAiKind::Unrepresented,
        LegacyCreatureAiSelectionDecisionLikeCpp::Selected(CreatureAiKindLikeCpp::CombatAI) => SpellAiKind::Combat,
        LegacyCreatureAiSelectionDecisionLikeCpp::Selected(CreatureAiKindLikeCpp::TurretAI) => SpellAiKind::Turret,
        LegacyCreatureAiSelectionDecisionLikeCpp::Selected(_) => SpellAiKind::Other,
    };
    let mut info = |id, difficulty, effective| {
        let spell = config.spell_store.as_ref()?.get(i32::try_from(id).ok()?)?;
        Some(if effective {
            projection::spell_info(&creature_ai_effective_spell_info_like_cpp(spell, difficulty, config))
        } else { projection::spell_info(spell) })
    };
    let mut condition = |id, difficulty| match creature_ai_spell_condition_like_cpp(id, difficulty, config) {
        CreatureAiSpellConditionLikeCpp::Aggro => SpellCondition::Aggro,
        CreatureAiSpellConditionLikeCpp::Combat => SpellCondition::Combat,
        CreatureAiSpellConditionLikeCpp::Die => SpellCondition::Die,
    };
    let mut target = |id, facts: &SpellInfoFacts, difficulty| target_facts(id, facts, difficulty, config);
    let mut disable = |id, map_id: u16, area_id| {
        let Some(disables) = config.disable_mgr.as_deref() else { return SpellDisable::Unrepresented; };
        let kind = config.map_store.as_deref().and_then(|store| store.get(u32::from(map_id)))
            .map(|entry| entry.instance_type);
        match disables.creature_spell_disable_decision_like_cpp(id, u32::from(map_id),
            (area_id != 0).then_some(area_id), kind.map(|kind| kind == wow_data::map::MAP_ARENA),
            kind.map(|kind| kind == wow_data::map::MAP_BATTLEGROUND)) {
            CreatureSpellDisableDecisionLikeCpp::Enabled => SpellDisable::Enabled,
            CreatureSpellDisableDecisionLikeCpp::Disabled => SpellDisable::Disabled,
            CreatureSpellDisableDecisionLikeCpp::ContextUnrepresented => SpellDisable::Unrepresented,
        }
    };
    let mut check = |check, id, difficulty| match check {
        SpellPreparationCheck::RuntimeHooks => config.spell_has_no_unrepresented_runtime_hooks_like_cpp(id),
        SpellPreparationCheck::CastingRequirements => config.spell_has_no_unrepresented_casting_requirements_like_cpp(id),
        SpellPreparationCheck::ShapeshiftRequirements => config.spell_has_no_unrepresented_shapeshift_requirements_like_cpp(id),
        SpellPreparationCheck::AuraRestrictions => config.spell_has_no_unrepresented_aura_restrictions_like_cpp(id, difficulty),
        SpellPreparationCheck::CooldownSemantics => creature_ai_spell_has_represented_cooldown_semantics_like_cpp(id, difficulty, config),
        SpellPreparationCheck::CombatForbidden => creature_ai_spell_is_combat_forbidden_like_cpp(id, difficulty, config),
        SpellPreparationCheck::TargetRestrictions => creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(id, difficulty, config),
        SpellPreparationCheck::Projectile => creature_ai_spell_requires_projectile_payload_like_cpp(id, difficulty, config),
    };
    let mut attributes = |id, difficulty| config.spell_store.as_ref()?.misc_attributes_for_difficulty_like_cpp(
        id, difficulty, config.difficulty_store.as_deref());
    let mut has_attribute = |id, difficulty, word, mask| config.spell_store.as_ref().is_some_and(|store|
        store.has_attribute_for_difficulty_like_cpp(id, difficulty, config.difficulty_store.as_deref(), word, mask));
    let mut minimum = |id, difficulty| initial_delay(id, difficulty, config);
    let mut visual = |id, difficulty| creature_ai_spell_x_spell_visual_id_like_cpp(id, difficulty, config);
    let mut go_flags = |id, difficulty| creature_ai_spell_go_cast_flags_like_cpp(id, difficulty, config);
    let mut cooldown = |id, difficulty| creature_ai_spell_cooldown_profile_like_cpp(id, difficulty, config)
        .map(|row| SpellCooldown { spell_id: row.spell_id, category_id: row.category_id,
            recovery_time_ms: row.recovery_time_ms, category_recovery_time_ms: row.category_recovery_time_ms,
            passive: row.passive });
    let mut range = |id, difficulty| creature_ai_effective_spell_range_like_cpp(id, difficulty, config)
        .map(|row| SpellRange { minimum: row.range_min[0], maximum: row.range_max[0], flags: row.flags });
    let mut hit_metadata = |id, difficulty| config.spell_store.as_ref()?.hit_metadata_for_difficulty_like_cpp(
        id, difficulty, config.difficulty_store.as_deref()).map(projection::hit_metadata);
    let mut faction_authority = || config.faction_template_store.is_some();
    let mut factions = |caster, victim| {
        let store = config.faction_template_store.as_deref()?;
        let (caster, victim) = (store.get(caster)?, store.get(victim)?);
        Some(SpellFactionFacts { creature_faction_id: u32::from(caster.faction),
            contested_guard: caster.is_contested_guard_faction_like_cpp(),
            caster_hostile: caster.is_hostile_to_like_cpp(victim), victim_hostile: victim.is_hostile_to_like_cpp(caster),
            caster_friendly: caster.is_friendly_to_like_cpp(victim), victim_friendly: victim.is_friendly_to_like_cpp(caster) })
    };
    let mut can_have_reputation = |id| config.faction_store.as_deref()?.get(id)
        .map(|row| row.can_have_reputation_like_cpp());
    apply(&mut SpellPolicies { select_ai: &mut select_ai, info: &mut info, condition: &mut condition,
        target: &mut target, disable: &mut disable, check: &mut check, attributes: &mut attributes,
        has_attribute: &mut has_attribute, minimum: &mut minimum, visual: &mut visual, go_flags: &mut go_flags,
        cooldown: &mut cooldown, range: &mut range, hit_metadata: &mut hit_metadata, faction_authority: &mut faction_authority,
        factions: &mut factions, can_have_reputation: &mut can_have_reputation })
}

pub(in crate::session) fn target_facts(id: u32, facts: &SpellInfoFacts, difficulty: u8,
    config: &LegacyCreatureAggroConfigLikeCpp) -> SpellTarget {
    let has_max_range = config.spell_misc_store.as_ref().and_then(|store|
        store.entry_for_spell_difficulty_with_fallback_like_cpp(id, difficulty, config.difficulty_store.as_deref()))
        .and_then(|misc| config.spell_range_store.as_ref().and_then(|store| store.get(u32::from(misc.range_index))))
        .is_some_and(|range| range.range_max[0] != 0.0);
    if !has_max_range { return SpellTarget::SelfTarget; }
    wow_map::map_manager::classify_target(facts)
}

pub(in crate::session) fn initial_delay(spell_id: u32, difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp) -> u64 {
    // C++ `AISpellInfoType` starts at AI_DEFAULT_COOLDOWN=5000 and
    // `FillAISpellInfo` raises it with raw `RecoveryTime`, deliberately not
    // `GetRecoveryTime()`/CategoryRecoveryTime.
    let recovery_time_ms =
        creature_spell_metadata::creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config)
            .and_then(|entry| u64::try_from(entry.recovery_time).ok())
            .unwrap_or(0);
    recovery_time_ms.max(5_000)
}
