//! Compatibility spell planning paths delegate to the Map-owned motor.
use super::*;
use super::canonical_runtime::spell::{catalogs, projection};
pub(in crate::session) use wow_map::map_manager::{
    SpellCastPlan as CreatureSpellCastPlanLikeCpp,
    SpellCasterIncarnation as CreatureSpellCasterIncarnationLikeCpp,
    SpellTopologyError as CreatureAiSpellRepresentationRejectionLikeCpp,
};

pub(in crate::session) fn creature_ai_spell_plan_like_cpp(
    creature: &crate::map_manager::WorldCreature, caster_guid: ObjectGuid,
    target_guid: ObjectGuid, map_id: u16, instance_id: u32, spell_id: u32,
    spell: &wow_data::SpellInfo, difficulty_id: u8, config: &LegacyCreatureAggroConfigLikeCpp,
) -> Result<CreatureSpellCastPlanLikeCpp, ()> {
    catalogs::with_policies(config, |policies| creature.spell_cast_plan(caster_guid, target_guid,
        map_id, instance_id, spell_id, &projection::spell_info(spell), difficulty_id, policies))
}

pub(in crate::session) fn creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(
    spell: &wow_data::SpellInfo,
) -> bool { wow_map::map_manager::has_nonzero_power_cost(&projection::spell_info(spell)) }

pub(in crate::session) fn creature_ai_zero_power_rows_have_unrepresented_implicit_cost_like_cpp(
    spell: &wow_data::SpellInfo, difficulty_id: u8, config: &LegacyCreatureAggroConfigLikeCpp,
    creature: &crate::map_manager::WorldCreature,
) -> bool {
    catalogs::with_policies(config, |policies| creature.spell_has_implicit_cost(
        &projection::spell_info(spell), difficulty_id, policies))
}

pub(in crate::session) fn creature_ai_spell_single_unit_topology_like_cpp(
    spell: &wow_data::SpellInfo, target_guid: ObjectGuid, recipient_guid: ObjectGuid,
    requires_projectile_payload: bool,
) -> Result<(), CreatureAiSpellRepresentationRejectionLikeCpp> {
    wow_map::map_manager::single_unit_topology(&projection::spell_info(spell),
        target_guid, recipient_guid, requires_projectile_payload)
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
    wow_map::map_manager::resets_combat_timers(command.cast_time_ms, has_attribute)
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
