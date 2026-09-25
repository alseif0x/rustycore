// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spell metadata: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{LegacyCreatureAggroConfigLikeCpp, ObjectGuid, spell_duration_ms_like_cpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::session) enum CreatureAiSpellTargetLikeCpp {
    SelfTarget,
    Victim,
    Enemy,
    Buff,
    Debuff,
}

impl CreatureAiSpellTargetLikeCpp {
    pub(in crate::session) fn requires_random_threat_selection_like_cpp(self) -> bool {
        matches!(self, Self::Enemy | Self::Debuff)
    }

    pub(in crate::session) fn resolve_for_single_player_threat_list_like_cpp(
        self,
        caster_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    ) -> ObjectGuid {
        match self {
            Self::SelfTarget | Self::Buff => caster_guid,
            Self::Victim | Self::Enemy | Self::Debuff => victim_guid,
        }
    }
}

pub(in crate::session) fn creature_ai_spell_target_like_cpp(
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureAiSpellTargetLikeCpp {
    const TARGET_UNIT_TARGET_ENEMY_LIKE_CPP: u32 = 6;
    const TARGET_UNIT_DEST_AREA_ENEMY_LIKE_CPP: u32 = 16;
    const TARGET_DEST_TARGET_ENEMY_LIKE_CPP: u32 = 53;

    // C++ does not inspect implicit targets when hostile max range is zero.
    let has_max_range = config
        .spell_misc_store
        .as_ref()
        .and_then(|store| {
            store.entry_for_spell_difficulty_with_fallback_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
            )
        })
        .and_then(|misc| {
            config
                .spell_range_store
                .as_ref()
                .and_then(|store| store.get(u32::from(misc.range_index)))
        })
        .is_some_and(|range| range.range_max[0] != 0.0);
    if !has_max_range {
        return CreatureAiSpellTargetLikeCpp::SelfTarget;
    }

    let positive = wow_data::represented_spell_is_positive_like_cpp(spell);
    spell.effects().iter().fold(
        CreatureAiSpellTargetLikeCpp::SelfTarget,
        |selected, effect| {
            // C++ `FillAISpellInfo` intentionally considers TargetA only.
            let target_a = effect.implicit_target_1;
            let mut candidate = match target_a {
                TARGET_UNIT_TARGET_ENEMY_LIKE_CPP | TARGET_DEST_TARGET_ENEMY_LIKE_CPP => {
                    CreatureAiSpellTargetLikeCpp::Victim
                }
                TARGET_UNIT_DEST_AREA_ENEMY_LIKE_CPP => CreatureAiSpellTargetLikeCpp::Enemy,
                _ => CreatureAiSpellTargetLikeCpp::SelfTarget,
            };
            if effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA {
                if target_a == TARGET_UNIT_TARGET_ENEMY_LIKE_CPP {
                    candidate = CreatureAiSpellTargetLikeCpp::Debuff;
                } else if positive {
                    candidate = CreatureAiSpellTargetLikeCpp::Buff;
                }
            }
            selected.max(candidate)
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum CreatureAiSpellConditionLikeCpp {
    Aggro,
    Combat,
    Die,
}

pub(in crate::session) fn creature_ai_spell_condition_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> CreatureAiSpellConditionLikeCpp {
    const SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD_LIKE_CPP: u32 = 0x0080_0000;
    if i32::try_from(spell_id).ok().is_some_and(|spell_id| {
        config.spell_store.as_ref().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                0,
                SPELL_ATTR0_ALLOW_CAST_WHILE_DEAD_LIKE_CPP,
            )
        })
    }) {
        return CreatureAiSpellConditionLikeCpp::Die;
    }
    let Some(misc) = config.spell_misc_store.as_ref().and_then(|store| {
        store.entry_for_spell_difficulty_with_fallback_like_cpp(
            spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) else {
        return CreatureAiSpellConditionLikeCpp::Combat;
    };
    if misc.is_passive_like_cpp()
        || spell_duration_ms_like_cpp(
            u32::from(misc.duration_index),
            config.spell_duration_store.as_deref(),
        ) == -1
    {
        CreatureAiSpellConditionLikeCpp::Aggro
    } else {
        CreatureAiSpellConditionLikeCpp::Combat
    }
}

pub(in crate::session) fn creature_ai_spell_difficulty_chain_like_cpp(
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Vec<u8> {
    let mut chain = Vec::new();
    let mut visited = [false; 256];
    let mut current = difficulty_id;
    loop {
        if visited[usize::from(current)] {
            break;
        }
        visited[usize::from(current)] = true;
        chain.push(current);
        if current == 0 {
            break;
        }
        current = config
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(u32::from(current)))
            .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
    }
    chain
}

pub(in crate::session) fn creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ resolves one effective `SpellTargetRestrictions` row through the
    // active difficulty fallback chain before `CheckTargetCreatureType`.
    // Creature/player type checks are not represented by M2.6, so missing
    // authority or any effective nonzero mask must keep publication closed.
    let Some(store) = config.spell_target_restrictions_store.as_ref() else {
        return true;
    };
    const REPRESENTED_HOSTILE_UNIT_TARGETS_LIKE_CPP: u32 = 0x0000_0002 | 0x0000_0080;

    store
        .resolved_for_difficulty_chain_like_cpp(
            spell_id,
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
                .into_iter()
                .map(u32::from),
        )
        .is_some_and(|restriction| {
            restriction.target_creature_type_mask_like_cpp() != 0
                // C++ seeds ExplicitTargetMask from this signed DB2 field,
                // then synthesizes any required source/destination payload.
                // This slice serializes only one hostile living Unit target;
                // fail closed for every other validation or wire requirement.
                || (restriction.targets as u32 & !REPRESENTED_HOSTILE_UNIT_TARGETS_LIKE_CPP != 0)
        })
}

pub(in crate::session) fn creature_ai_spell_cooldowns_entry_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<&wow_data::SpellCooldownsEntry> {
    let store = config.spell_cooldowns_store.as_ref()?;
    creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
        .into_iter()
        .find_map(|difficulty_id| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
                .max_by_key(|entry| entry.id)
        })
}

#[derive(Debug, Clone, Copy)]
pub(in crate::session) struct CreatureSpellCooldownProfileLikeCpp {
    pub(in crate::session) spell_id: u32,
    pub(in crate::session) category_id: u32,
    pub(in crate::session) recovery_time_ms: u64,
    pub(in crate::session) category_recovery_time_ms: u64,
    pub(in crate::session) passive: bool,
}

pub(in crate::session) fn creature_ai_spell_cooldown_profile_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Option<CreatureSpellCooldownProfileLikeCpp> {
    let signed_spell_id = i32::try_from(spell_id).ok()?;
    let spell_store = config.spell_store.as_deref()?;
    let metadata = spell_store.hit_metadata_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
    )?;
    let cooldowns = creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config);
    Some(CreatureSpellCooldownProfileLikeCpp {
        spell_id,
        category_id: metadata.category_id,
        recovery_time_ms: cooldowns
            .and_then(|entry| u64::try_from(entry.recovery_time).ok())
            .unwrap_or(0),
        category_recovery_time_ms: cooldowns
            .and_then(|entry| u64::try_from(entry.category_recovery_time).ok())
            .unwrap_or(0),
        passive: spell_store.is_passive_like_cpp(signed_spell_id),
    })
}

pub(in crate::session) fn creature_ai_spell_has_represented_cooldown_semantics_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    const SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP: u32 = 0x0200_0000;
    const SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP: i32 = 0x04;

    let Ok(signed_spell_id) = i32::try_from(spell_id) else {
        return false;
    };
    let Some(spell_store) = config.spell_store.as_deref() else {
        return false;
    };
    let Some(metadata) = spell_store.hit_metadata_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
    ) else {
        return false;
    };
    if spell_store.is_passive_like_cpp(signed_spell_id) {
        return true;
    }
    // C++ consumes charges instead of starting the normal spell/category
    // cooldown. M2.6 does not yet own creature charge recovery.
    if metadata.charge_category_id != 0 {
        return false;
    }
    if spell_store.has_attribute_for_difficulty_like_cpp(
        signed_spell_id,
        difficulty_id,
        config.difficulty_store.as_deref(),
        0,
        SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP,
    ) {
        return false;
    }
    if metadata.category_id == 0 {
        return true;
    }
    config
        .spell_category_store
        .as_deref()
        .and_then(|store| store.get(metadata.category_id))
        .is_some_and(|category| {
            category.flags & SPELL_CATEGORY_FLAG_COOLDOWN_STARTS_ON_EVENT_LIKE_CPP == 0
        })
}

pub(in crate::session) fn creature_ai_spell_is_combat_forbidden_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    i32::try_from(spell_id).ok().is_none_or(|spell_id| {
        config.spell_store.as_ref().is_none_or(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                0,
                wow_data::spell::attributes::SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL,
            )
        })
    })
}

pub(in crate::session) fn creature_ai_effective_spell_info_like_cpp(
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> wow_data::SpellInfo {
    let mut effective = spell.clone();
    if let Some(effects) = config.spell_store.as_ref().and_then(|store| {
        store.effects_for_difficulty_like_cpp(
            spell.spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) {
        effective.effects = effects.to_vec();
    }
    effective
}

pub(in crate::session) fn creature_ai_has_temporally_unrepresented_noninstant_spell_like_cpp(
    spells: &[u32],
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // M2.6 has no cast-completion/cancellation state. If one template slot
    // could start a non-instant Aggro/Combat cast, merely dropping that slot
    // would leave Rust free to emit another slot while C++ still owns
    // UNIT_STATE_CASTING. Suppress this creature's whole spell surface until
    // M3.1 can represent that temporal state.
    spells
        .iter()
        .copied()
        .filter(|spell_id| *spell_id != 0)
        .any(|spell_id| {
            let Some(spell_store) = config.spell_store.as_ref() else {
                return false;
            };
            let Ok(spell_id_i32) = i32::try_from(spell_id) else {
                return false;
            };
            let Some(spell) = spell_store.get(spell_id_i32) else {
                return false;
            };
            let spell = creature_ai_effective_spell_info_like_cpp(spell, difficulty_id, config);
            spell.cast_time_ms != 0
                && matches!(
                    creature_ai_spell_condition_like_cpp(spell_id, difficulty_id, config),
                    CreatureAiSpellConditionLikeCpp::Aggro
                        | CreatureAiSpellConditionLikeCpp::Combat
                )
        })
}

pub(in crate::session) fn creature_ai_spell_initial_cooldown_like_cpp(
    spell_id: u32,
    _spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u64 {
    // C++ `AISpellInfoType` starts at AI_DEFAULT_COOLDOWN=5000 and
    // `FillAISpellInfo` raises it with raw `RecoveryTime`, deliberately not
    // `GetRecoveryTime()`/CategoryRecoveryTime.
    let recovery_time_ms =
        creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config)
            .and_then(|entry| u64::try_from(entry.recovery_time).ok())
            .unwrap_or(0);
    recovery_time_ms.max(5_000)
}

pub(in crate::session) fn creature_ai_spell_repeat_cooldown_like_cpp(
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u64 {
    // C++ `CombatAI::UpdateAI` re-schedules in [cooldown, cooldown*2].
    creature_ai_spell_initial_cooldown_like_cpp(spell_id, spell, difficulty_id, config)
}

pub(in crate::session) fn creature_ai_spell_x_spell_visual_id_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Result<u32, ()> {
    // C++ falls back only when the current difficulty has no visual rows at
    // all. It then orders the selected vector by caster-player condition and
    // evaluates both caster conditions. Viewer conditions likewise make the
    // emitted visual row viewer-specific. This slice can prove only a single
    // unconditional row; a conditional or ambiguous selected vector must not
    // silently fall through to another difficulty or pick a HashMap order.
    let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
        return Err(());
    };
    for difficulty_id in creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config) {
        let rows = store
            .entries_like_cpp()
            .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        if rows.len() != 1
            || rows[0].viewer_player_condition_id != 0
            || rows[0].viewer_unit_condition_id != 0
            || rows[0].caster_player_condition_id != 0
            || rows[0].caster_unit_condition_id != 0
        {
            return Err(());
        }
        return Ok(rows[0].id);
    }
    Ok(0)
}

pub(in crate::session) fn creature_ai_spell_go_cast_flags_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u32 {
    const CAST_FLAG_UNKNOWN_9_LIKE_CPP: u32 = 0x0000_0100;
    const CAST_FLAG_NO_GCD_LIKE_CPP: u32 = 0x0004_0000;
    let start_recovery_time =
        creature_ai_spell_cooldowns_entry_like_cpp(spell_id, difficulty_id, config)
            .map_or(0, |entry| entry.start_recovery_time);
    CAST_FLAG_UNKNOWN_9_LIKE_CPP
        | (start_recovery_time == 0)
            .then_some(CAST_FLAG_NO_GCD_LIKE_CPP)
            .unwrap_or(0)
}
