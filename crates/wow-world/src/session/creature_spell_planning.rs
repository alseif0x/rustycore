// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spell planning: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::creature_ai_spell_x_spell_visual_id_like_cpp;
use super::{LegacyCreatureAggroConfigLikeCpp, ObjectGuid, OwnedLootAuthority};
use super::{
    creature_ai_spell_difficulty_chain_like_cpp, creature_ai_spell_go_cast_flags_like_cpp,
};

/// The exact creature incarnation a cast plan was captured from.
///
/// A GUID and an engagement epoch cannot separate a replacement that reused
/// both, so carry the same three-part identity the melee synchronization path
/// proves: spawn ID, loot-storage authority and health-state revision
/// authority. Holding the two authorities keeps their allocations alive, so
/// neither identity can be recycled while the plan is in flight.
#[derive(Clone)]
pub(in crate::session) struct CreatureSpellCasterIncarnationLikeCpp {
    pub(in crate::session) spawn_id: u64,
    pub(in crate::session) authority: OwnedLootAuthority,
    pub(in crate::session) health_state_revision_authority:
        wow_entities::HealthStateRevisionAuthorityLikeCpp,
}

impl CreatureSpellCasterIncarnationLikeCpp {
    pub(in crate::session) fn capture_like_cpp(creature: &wow_entities::Creature) -> Self {
        Self {
            spawn_id: creature.spawn_id(),
            authority: creature.loot_authority_like_cpp().clone(),
            health_state_revision_authority: creature
                .unit()
                .health_state_revision_authority_like_cpp(),
        }
    }

    pub(in crate::session) fn matches_like_cpp(&self, creature: &wow_entities::Creature) -> bool {
        creature.spawn_id() == self.spawn_id
            && creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&self.authority)
            && creature
                .unit()
                .shares_health_state_revision_authority_like_cpp(
                    &self.health_state_revision_authority,
                )
    }
}

impl std::fmt::Debug for CreatureSpellCasterIncarnationLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `OwnedLootAuthority` is deliberately not `Debug`: its identity is the
        // shared allocation, not a printable value.
        f.debug_struct("CreatureSpellCasterIncarnationLikeCpp")
            .field("spawn_id", &self.spawn_id)
            .field(
                "health_state_revision_authority",
                &self.health_state_revision_authority,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub(in crate::session) struct CreatureSpellCastPlanLikeCpp {
    pub(in crate::session) caster_guid: ObjectGuid,
    pub(in crate::session) target_guid: ObjectGuid,
    pub(in crate::session) map_id: u16,
    pub(in crate::session) instance_id: u32,
    pub(in crate::session) spell_id: i32,
    pub(in crate::session) spell_x_spell_visual_id: u32,
    pub(in crate::session) cast_time_ms: u32,
    pub(in crate::session) spell_go_cast_flags: u32,
    pub(in crate::session) engagement_epoch: u64,
    pub(in crate::session) caster_incarnation: CreatureSpellCasterIncarnationLikeCpp,
}

pub(in crate::session) fn creature_ai_successful_untriggered_spell_resets_combat_timers_like_cpp(
    command: &CreatureSpellCastPlanLikeCpp,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ `Spell::IsAutoActionResetSpell` rejects triggered casts and these
    // two attribute cases. CreatureAI `DoCast`/`CastSpell` is untriggered in
    // this slice, so a successful represented cast resets the base swing when
    // neither applicable opt-out is present (Spell.cpp:3839-3840, 8056-8064).
    const SPELL_ATTR2_DO_NOT_RESET_COMBAT_TIMERS_LIKE_CPP: u32 = 0x0002_0000;
    const SPELL_ATTR6_DOESNT_RESET_SWING_TIMER_IF_INSTANT_LIKE_CPP: u32 = 0x0200_0000;

    let has_attribute = |word, attribute| {
        config.spell_store.as_ref().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                command.spell_id,
                difficulty_id,
                config.difficulty_store.as_deref(),
                word,
                attribute,
            )
        })
    };
    !has_attribute(2, SPELL_ATTR2_DO_NOT_RESET_COMBAT_TIMERS_LIKE_CPP)
        && (command.cast_time_ms != 0
            || !has_attribute(6, SPELL_ATTR6_DOESNT_RESET_SWING_TIMER_IF_INSTANT_LIKE_CPP))
}

pub(in crate::session) fn creature_ai_spell_plan_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    spell_id: u32,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Result<CreatureSpellCastPlanLikeCpp, ()> {
    Ok(CreatureSpellCastPlanLikeCpp {
        caster_guid,
        target_guid,
        map_id,
        instance_id,
        spell_id: spell.spell_id,
        spell_x_spell_visual_id: creature_ai_spell_x_spell_visual_id_like_cpp(
            spell_id,
            difficulty_id,
            config,
        )?,
        cast_time_ms: spell.cast_time_ms,
        spell_go_cast_flags: creature_ai_spell_go_cast_flags_like_cpp(
            spell_id,
            difficulty_id,
            config,
        ),
        engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
        caster_incarnation: CreatureSpellCasterIncarnationLikeCpp::capture_like_cpp(
            &creature.creature,
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum CreatureAiSpellRepresentationRejectionLikeCpp {
    NonInstant,
    ProjectileOrAmmo,
    EffectOrTarget,
}

pub(in crate::session) fn creature_ai_spell_requires_projectile_payload_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    // C++ `Spell::SendSpellStart` / `SendSpellGo` add
    // `CAST_FLAG_PROJECTILE` and `SpellCastData::AmmoDisplayID` for any of
    // these three attributes (Spell.cpp:4678-4679, 4750-4751, 4779-4780).
    // The represented packet currently serializes neither optional ammo field,
    // so accepting one of these spells would produce a structurally different
    // START+GO pair.
    const SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP: u32 = 0x0000_0002;
    const SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP: u32 = 0x0000_0004;
    const SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP: u32 = 0x0008_0000;

    let Some(spell_id_i32) = i32::try_from(spell_id).ok() else {
        return true;
    };
    let db2_requires_projectile = config.spell_store.as_ref().is_some_and(|store| {
        store.has_attribute_for_difficulty_like_cpp(
            spell_id_i32,
            difficulty_id,
            config.difficulty_store.as_deref(),
            0,
            SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP,
        ) || store.has_attribute_for_difficulty_like_cpp(
            spell_id_i32,
            difficulty_id,
            config.difficulty_store.as_deref(),
            10,
            SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP,
        )
    });
    let custom_requires_ammo = config
        .spell_custom_attribute_store
        .as_ref()
        .is_some_and(|store| {
            creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config)
                .into_iter()
                .any(|difficulty_id| {
                    store.attributes_for_spell_difficulty_like_cpp(
                        spell_id,
                        u32::from(difficulty_id),
                    ) & SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP
                        != 0
                })
        });
    db2_requires_projectile || custom_requires_ammo
}

pub(in crate::session) fn creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(
    spell: &wow_data::SpellInfo,
) -> bool {
    // `SpellInfo::PowerCosts` retains zero-valued SpellPower rows. Their mere
    // presence does not make `Spell::m_powerCost` nonzero; in particular live
    // 15691 has a type-3 row whose flat, per-level, periodic, percentage,
    // max-percentage, periodic-percentage and optional values are all zero.
    // Any nonzero cost input still fails closed until Creature power
    // calculation/check/deduction is represented.
    spell.power_costs.iter().any(|cost| {
        cost.mana_cost != 0
            || cost.mana_cost_per_level != 0
            || cost.mana_per_second != 0
            || cost.power_cost_pct != 0.0
            || cost.power_cost_max_pct != 0.0
            || cost.power_pct_per_second != 0.0
            || cost.required_aura_spell_id != 0
            || cost.optional_cost != 0
    })
}

pub(in crate::session) fn creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
    creature: &crate::map_manager::WorldCreature,
) -> bool {
    const SPELL_ATTR1_USE_ALL_MANA_LIKE_CPP: u32 = 0x0000_0002;
    const SPELL_ATTR4_WEAPON_SPEED_COST_SCALING_LIKE_CPP: u32 = 0x0000_0400;
    const SPELL_AURA_MOD_ADDITIONAL_POWER_COST_LIKE_CPP: i32 = 63;

    if spell.power_costs.is_empty()
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
    {
        return false;
    }
    let Some(attributes) = config.spell_store.as_ref().and_then(|store| {
        store.misc_attributes_for_difficulty_like_cpp(
            spell.spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )
    }) else {
        // A zero-valued DB2 row is safe only when the attributes that can turn
        // it into a real cost are themselves represented and absent.
        return true;
    };
    if attributes[1] & SPELL_ATTR1_USE_ALL_MANA_LIKE_CPP != 0
        || attributes[4] & SPELL_ATTR4_WEAPON_SPEED_COST_SCALING_LIKE_CPP != 0
    {
        return true;
    }

    let auras = &creature.creature.unit().subsystems().auras;
    auras.has_aura_type_like_cpp(SPELL_AURA_MOD_ADDITIONAL_POWER_COST_LIKE_CPP)
        || auras
            .has_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL)
        || auras.has_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT,
        )
}

pub(in crate::session) fn creature_ai_spell_single_unit_topology_like_cpp(
    spell: &wow_data::SpellInfo,
    target_guid: ObjectGuid,
    recipient_guid: ObjectGuid,
    requires_projectile_payload: bool,
) -> Result<(), CreatureAiSpellRepresentationRejectionLikeCpp> {
    // Cast-time completion belongs to M3.1. M2.6 must fail closed rather than
    // emitting START and an immediate, premature GO for a non-instant spell.
    if spell.cast_time_ms != 0 {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::NonInstant);
    }
    if requires_projectile_payload {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::ProjectileOrAmmo);
    }
    if spell.requires_spell_focus != 0
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
    {
        // Focus discovery and Creature power-cost calculation/deduction are
        // not part of M2.6. Emitting GO when C++ CheckCast/CheckPower would
        // fail would be a false successful cast, so keep this slice closed.
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }
    if target_guid != recipient_guid {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }

    // M2.6 owns AI selection, cooldowns and cast wire. Damage calculation is
    // deliberately left to M3.2: raw EffectBasePoints is not CalcValue and
    // omits dice, scaling, bonuses, mitigation, hit, absorb and resist. Until
    // that pipeline exists, only a topology whose START/GO target lists can be
    // proven from hydrated metadata is emitted, with no fabricated health
    // mutation. TargetA=6 is C++ TARGET_UNIT_TARGET_ENEMY; TargetB must be
    // empty, and chained/radius/triggered effects would add unsupported target
    // or follow-up topology.
    let mut represented_effects = 0usize;
    for effect in spell.effects().iter().filter(|effect| effect.effect != 0) {
        if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(effect.effect) {
            continue;
        }
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
            || effect.implicit_target_1 != 6
            || effect.implicit_target_2 != 0
            || effect.chain_targets != 0
            || effect.effect_radius_index_1 != 0
            || effect.effect_trigger_spell != 0
        {
            return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
        }
        represented_effects += 1;
    }
    if represented_effects == 0 {
        return Err(CreatureAiSpellRepresentationRejectionLikeCpp::EffectOrTarget);
    }
    Ok(())
}
