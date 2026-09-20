//! Nonlethal Creature threat settlement for represented white-swing damage.

use wow_core::ObjectGuid;

#[derive(Clone, Copy, Debug)]
pub(in crate::session) struct CreatureDamageThreatPlanLikeCpp {
    suppress: bool,
    no_initial_threat: bool,
    multiplier: f32,
}

#[derive(Clone, Debug)]
pub(in crate::session) struct CreatureDamageThreatOutcomeLikeCpp {
    pub(in crate::session) attacker_guid: ObjectGuid,
    pub(in crate::session) attacker_authority: wow_loot::OwnedLootAuthority,
    pub(in crate::session) attacker_spawn_id: u64,
    pub(in crate::session) delta: f32,
}

impl Default for CreatureDamageThreatPlanLikeCpp {
    fn default() -> Self {
        Self {
            suppress: false,
            no_initial_threat: false,
            multiplier: 1.0,
        }
    }
}

/// Plan the modifiers C++ `ThreatManager::AddThreat` applies to the
/// post-clamp `damageTaken` passed by `Unit::DealDamage` (`Unit.cpp:1033-1034`,
/// `ThreatManager.cpp:353-465`, `662-690`).
#[allow(clippy::too_many_arguments)]
pub(super) fn plan_creature_damage_threat_like_cpp(
    map: &wow_map::ManagedMapInnerLikeCpp,
    attacker_guid: ObjectGuid,
    spell_id: Option<i32>,
    spell_store: Option<&wow_data::SpellStore>,
    spell_misc_store: Option<&wow_data::SpellMiscStore>,
    spell_threat_store: Option<&wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: Option<&wow_data::SpellChainStoreLikeCpp>,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> CreatureDamageThreatPlanLikeCpp {
    let suppress = spell_id.is_some_and(|spell_id| {
        spell_store.is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                1,
                wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
            ) || store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                4,
                wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
            )
        })
    });
    let no_initial_threat = spell_id.is_some_and(|spell_id| {
        spell_store.is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                2,
                wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
            )
        })
    });
    let school_mask = spell_id
        .and_then(|spell_id| u32::try_from(spell_id).ok())
        .and_then(|spell_id| {
            spell_misc_store.and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    spell_id,
                    difficulty_id,
                    difficulty_store,
                )
            })
        })
        .map_or(0x01, |entry| u32::from(entry.school_mask));
    let spell_multiplier = spell_id
        .and_then(|spell_id| u32::try_from(spell_id).ok())
        .and_then(|spell_id| {
            spell_threat_store.and_then(|store| {
                store.get_spell_threat_entry_like_cpp(spell_id, |spell_id| {
                    spell_chain_store.map_or(spell_id, |chains| {
                        chains.first_spell_in_chain_like_cpp(spell_id)
                    })
                })
            })
        })
        .map_or(1.0, |entry| entry.pct_mod);
    let aura_multiplier = map
        .with_creature_like_cpp(attacker_guid, |attacker| {
            let Some(spell_store) = spell_store else {
                return 1.0;
            };
            crate::session_rules::creature_aura_effects_like_cpp(
                &attacker.unit().subsystems().auras.applied_auras,
                spell_store,
                difficulty_id,
                difficulty_store,
            )
            .into_iter()
            .filter(|effect| {
                effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT
                    && effect.misc_value as u32 & school_mask != 0
            })
            .fold(1.0_f32, |multiplier, effect| {
                multiplier * (1.0 + effect.amount as f32 / 100.0)
            })
        })
        .unwrap_or(1.0);

    CreatureDamageThreatPlanLikeCpp {
        suppress,
        no_initial_threat,
        multiplier: spell_multiplier * aura_multiplier,
    }
}

/// Apply one already-authorized nonlethal damage threat transition and create
/// the reciprocal threat reference on the Creature attacker. The returned
/// value is the exact additive delta that the compatibility mirror must replay.
pub(super) fn apply_creature_damage_threat_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    victim_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
    damage_taken: u32,
    plan: CreatureDamageThreatPlanLikeCpp,
) -> Option<CreatureDamageThreatOutcomeLikeCpp> {
    if victim_guid == attacker_guid || plan.suppress {
        return None;
    }
    let attacker_identity = map.with_creature_like_cpp(attacker_guid, |attacker| {
        (
            attacker.is_alive(),
            attacker.loot_authority_like_cpp().clone(),
            attacker.spawn_id(),
        )
    });
    let admitted = map
        .with_creature_like_cpp(victim_guid, |victim| {
            victim.is_alive()
                && (!plan.no_initial_threat || victim.unit().subsystems().combat.has_combat())
        })
        .unwrap_or(false)
        && attacker_identity
            .as_ref()
            .is_some_and(|(alive, _, _)| *alive);
    if !admitted {
        return None;
    }

    let amount = damage_taken as f32 * plan.multiplier;
    let victim_started = map
        .get_typed_creature_mut(victim_guid)?
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(attacker_guid, false, false);
    let attacker_started = map
        .get_typed_creature_mut(attacker_guid)?
        .unit_mut()
        .subsystems_mut()
        .combat
        .set_in_combat_with(victim_guid, false, false);
    if !victim_started || !attacker_started {
        return None;
    }

    map.get_typed_creature_mut(victim_guid)?
        .enter_ai_combat(attacker_guid);

    let threat_ref = {
        let victim = map.get_typed_creature_mut(victim_guid)?;
        victim
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(attacker_guid, amount);
        victim
            .unit()
            .subsystems()
            .combat
            .threat_ref(attacker_guid)
            .copied()
    };
    if let Some(threat_ref) = threat_ref {
        map.get_typed_creature_mut(attacker_guid)?
            .unit_mut()
            .subsystems_mut()
            .combat
            .put_threatened_by_me_ref(victim_guid, threat_ref);
    }
    let (_, attacker_authority, attacker_spawn_id) = attacker_identity?;
    Some(CreatureDamageThreatOutcomeLikeCpp {
        attacker_guid,
        attacker_authority,
        attacker_spawn_id,
        delta: amount,
    })
}
