// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::live_player;
use crate::session::WorldSession;
use std::sync::Arc;
use std::time::Instant;
use wow_constants::DeathState;
use wow_core::ObjectGuid;
use wow_entities::{
    AppliedAuraRef, AuraApplicationLikeCpp, OwnedAuraRef, PlayerDeathOperationContext,
    PlayerDeathOperationPhase,
};
use wow_world_entities::PlayerMovementHostLikeCpp;

const ORDINARY_AURA: u32 = 94_011;
const PASSIVE_AURA: u32 = 94_012;
const DEATH_PERSISTENT_AURA: u32 = 94_013;
const DUMMY_AURA_TYPE_LIKE_CPP: i32 = 4;

fn install_aura_catalog(session: &mut WorldSession, spell_attributes: &[(u32, [i32; 15])]) {
    let misc_entries = spell_attributes
        .iter()
        .enumerate()
        .map(
            |(index, (spell_id, attributes))| wow_data::spell_db2::SpellMiscEntry {
                id: u32::try_from(index + 1).expect("fixture SpellMisc id fits u32"),
                attributes: *attributes,
                difficulty_id: 0,
                spell_icon_file_data_id: 0,
                active_icon_file_data_id: 0,
                spell_id: *spell_id,
                ..Default::default()
            },
        )
        .collect::<Vec<_>>();
    let class_options_entries = spell_attributes
        .iter()
        .enumerate()
        .map(|(index, (spell_id, _))| wow_data::SpellClassOptionsEntry {
            id: u32::try_from(index + 1).expect("fixture SpellClassOptions id fits u32"),
            spell_id: i32::try_from(*spell_id).expect("fixture spell id fits i32"),
            modal_next_spell: 0,
            spell_class_set: 0,
            spell_class_mask: [0; 4],
        })
        .collect::<Vec<_>>();
    let effect_entries = spell_attributes
        .iter()
        .enumerate()
        .map(
            |(index, (spell_id, _))| wow_data::spell_db2::SpellEffectDb2Entry {
                id: u32::try_from(index + 1).expect("fixture SpellEffect id fits u32"),
                difficulty_id: 0,
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_amplitude: 0.0,
                effect_attributes: 0,
                effect_aura: i16::try_from(DUMMY_AURA_TYPE_LIKE_CPP)
                    .expect("DUMMY aura type fits i16"),
                effect_aura_period: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                effect_chain_amplitude: 0.0,
                effect_chain_targets: 0,
                effect_die_sides: 0,
                effect_item_type: 0,
                effect_mechanic: 0,
                effect_points_per_resource: 0.0,
                effect_pos_facing: 0.0,
                effect_real_points_per_level: 0.0,
                effect_trigger_spell: 0,
                bonus_coefficient_from_ap: 0.0,
                pvp_multiplier: 0.0,
                coefficient: 0.0,
                variance: 0.0,
                resource_coefficient: 0.0,
                group_size_base_points_coefficient: 0.0,
                effect_misc_value: [0; 2],
                effect_radius_index: [0; 2],
                effect_spell_class_mask: [0; 4],
                implicit_target: [0; 2],
                spell_id: *spell_id,
            },
        )
        .collect::<Vec<_>>();
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries(misc_entries),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effect_entries),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let spells = crate::selected_spell_fixtures::canonical_spell_store(
        spell_attributes.iter().map(|(spell_id, _)| (*spell_id, 0)),
    )
    .hydrate_effective_core_db2_like_cpp(stores);
    session.set_spell_store(Arc::new(spells));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries(class_options_entries),
    ));
}

fn aura_application(spell_id: u32, caster_guid: ObjectGuid, slot: u8) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id: i32::try_from(spell_id).expect("fixture spell id fits i32"),
        difficulty_id: 0,
        caster_guid,
        slot,
        duration_total: -1,
        duration_remaining: -1,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

fn add_runtime_aura(session: &mut WorldSession, spell_id: u32, caster_guid: ObjectGuid, slot: u8) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .insert_runtime_application_like_cpp(aura_application(spell_id, caster_guid, slot));
        })
        .expect("canonical Player");
}

fn add_owned_typed_aura(
    session: &mut WorldSession,
    spell_id: u32,
    caster_guid: ObjectGuid,
    slot: u8,
) -> AppliedAuraRef {
    let applied = AppliedAuraRef::new(spell_id, caster_guid, slot, 1);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.insert_runtime_application_like_cpp(aura_application(
                spell_id,
                caster_guid,
                slot,
            ));
            auras.add_owned(OwnedAuraRef::new(spell_id, caster_guid, None));
            auras.register_applied_aura_type_like_cpp(applied, DUMMY_AURA_TYPE_LIKE_CPP);
        })
        .expect("canonical Player");
    applied
}

#[tokio::test]
async fn native_death_auras_remove_ordinary_references_and_keep_protected_auras() {
    let (mut session, _send, _realm, guid) = live_player(94_011);
    let ordinary = [0; 15];
    let mut passive = [0; 15];
    passive[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    let mut death_persistent = [0; 15];
    death_persistent[3] = 0x0010_0000;
    install_aura_catalog(
        &mut session,
        &[
            (ORDINARY_AURA, ordinary),
            (PASSIVE_AURA, passive),
            (DEATH_PERSISTENT_AURA, death_persistent),
        ],
    );
    let ordinary_ref = add_owned_typed_aura(&mut session, ORDINARY_AURA, guid, 11);
    let passive_ref = add_owned_typed_aura(&mut session, PASSIVE_AURA, guid, 12);
    let death_persistent_ref = add_owned_typed_aura(&mut session, DEATH_PERSISTENT_AURA, guid, 13);

    let item_guid_generator = session.id_generators_for_test_like_cpp().item;
    let outcome = PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
        &mut session,
        1_000,
        PlayerDeathOperationContext::new(guid, false),
        &Default::default(),
        item_guid_generator.as_ref(),
    )
    .await
    .expect("ordinary removable Aura supports native death cleanup");
    assert!(outcome.killed);

    let remaining = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let mut runtime_spell_ids = auras
                .runtime_applications_like_cpp()
                .values()
                .map(|aura| aura.spell_id as u32)
                .collect::<Vec<_>>();
            runtime_spell_ids.sort_unstable();
            (
                runtime_spell_ids,
                auras.applied_auras.clone(),
                auras.owned_auras.clone(),
                auras.applied_aura_types.clone(),
            )
        })
        .expect("canonical Player");

    assert_eq!(remaining.0, vec![PASSIVE_AURA, DEATH_PERSISTENT_AURA]);
    assert_eq!(remaining.1, vec![passive_ref, death_persistent_ref]);
    assert_eq!(
        remaining.2,
        vec![
            OwnedAuraRef::new(PASSIVE_AURA, guid, None),
            OwnedAuraRef::new(DEATH_PERSISTENT_AURA, guid, None),
        ]
    );
    assert_eq!(
        remaining.3,
        std::collections::HashMap::from([(
            DUMMY_AURA_TYPE_LIKE_CPP,
            vec![passive_ref, death_persistent_ref],
        )])
    );
    assert!(!remaining.1.contains(&ordinary_ref));
}

#[tokio::test]
async fn native_death_auras_reject_duplicate_spell_without_order_before_claim() {
    let (mut session, _send, _realm, guid) = live_player(94_012);
    install_aura_catalog(&mut session, &[(ORDINARY_AURA, [0; 15])]);
    add_runtime_aura(&mut session, ORDINARY_AURA, guid, 21);
    add_runtime_aura(&mut session, ORDINARY_AURA, guid, 22);

    let item_guid_generator = session.id_generators_for_test_like_cpp().item;
    let outcome = PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
        &mut session,
        1_000,
        PlayerDeathOperationContext::new(guid, false),
        &Default::default(),
        item_guid_generator.as_ref(),
    )
    .await;

    assert!(outcome.is_err());
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.unit().data().health,
            player.unit().death_state(),
            player
                .native_death_operation_snapshot()
                .map(|snapshot| snapshot.phase()),
        )),
        Some((
            1_000,
            DeathState::Alive,
            Some(PlayerDeathOperationPhase::Ready),
        ))
    );
}
