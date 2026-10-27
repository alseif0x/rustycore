use super::*;
use super::selected_load::{difficulty, indexed_effect, selected_misc};
use std::sync::Arc;

fn install_nonzero_only_sickness_data(session: &mut WorldSession) {
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    let misc_rows = vec![selected_misc(
        3,
        SICKNESS_SPELL_ID as u32,
        3,
        427,
        attributes,
    )];
    let effect_rows = [
        (1_u32, 0_i32, 80_i16, -1_i32),
        (2_u32, 1_i32, 79_i16, 127_i32),
        (3_u32, 2_i32, 159_i16, 127_i32),
    ]
    .into_iter()
    .map(|(id, index, aura_type, misc_value)| {
        indexed_effect(
            id,
            SICKNESS_SPELL_ID as u32,
            3,
            index,
            aura_type,
            misc_value,
        )
    })
    .collect::<Vec<_>>();
    let exact_spell_keys = misc_rows
        .iter()
        .map(|row| (row.spell_id, row.difficulty_id))
        .chain(
            effect_rows
                .iter()
                .map(|row| (row.spell_id, row.difficulty_id as u8)),
        )
        .collect::<Vec<_>>();
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries(misc_rows.clone()),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effect_rows),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let spells = crate::selected_spell_fixtures::canonical_spell_store(exact_spell_keys)
        .hydrate_effective_core_db2_like_cpp(stores);
    assert!(spells.get(SICKNESS_SPELL_ID).is_none());
    assert!(!spells.contains_spell_info_exact_like_cpp(SICKNESS_SPELL_ID as u32, 0));
    assert!(spells.contains_spell_info_exact_like_cpp(SICKNESS_SPELL_ID as u32, 3));
    assert_eq!(
        spells.misc_attributes_for_exact_difficulty(SICKNESS_SPELL_ID, 0),
        None
    );
    assert_eq!(
        spells.misc_attributes_for_exact_difficulty(SICKNESS_SPELL_ID, 3),
        Some(attributes)
    );
    assert!(spells
        .effect_rows_for_exact_difficulty(SICKNESS_SPELL_ID, 0)
        .is_none());
    assert_eq!(
        spells
            .effect_rows_for_exact_difficulty(SICKNESS_SPELL_ID, 3)
            .map(|rows| rows.len()),
        Some(3)
    );

    session.set_spell_store(Arc::new(spells));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(
        misc_rows,
    )));
    session.set_spell_duration_store(Arc::new(
        wow_data::SpellDurationStore::from_entries([wow_data::SpellDurationEntry {
            id: 427,
            duration: -600_000,
            duration_per_level: 0,
            max_duration: 600_000,
        }]),
    ));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
        difficulty(3, 0),
    ])));
    session.set_spell_aura_options_store(Arc::new(
        wow_data::SpellAuraOptionsStore::from_entries([wow_data::SpellAuraOptionsEntry {
            id: SICKNESS_SPELL_ID as u32,
            difficulty_id: 3,
            cumulative_aura: 0,
            proc_category_recovery: 0,
            proc_chance: 0,
            proc_charges: 5,
            spell_procs_per_minute_id: 0,
            proc_type_mask: [0; 2],
            spell_id: SICKNESS_SPELL_ID as u32,
        }]),
    ));
}

fn rows_from_save(
    saved: &wow_persistence::PlayerLifecycleAuraSpellSave,
) -> (
    Vec<crate::session::CharacterAuraRowLikeCpp>,
    Vec<crate::session::CharacterAuraEffectRowLikeCpp>,
) {
    let mut aura_rows = Vec::new();
    let mut effect_rows = Vec::new();
    for family in &saved.families {
        let caster_guid = ObjectGuid::from_raw_bytes(&family.caster_guid_binary);
        let item_guid = ObjectGuid::from_raw_bytes(&family.item_guid_binary);
        for aura in &family.auras {
            let effect_mask = aura
                .effects
                .iter()
                .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index));
            aura_rows.push(crate::session::CharacterAuraRowLikeCpp {
                caster_guid,
                item_guid,
                spell_id: family.spell_id,
                effect_mask,
                recalculate_mask: aura.recalculate_mask,
                difficulty: aura.difficulty,
                stack_count: aura.stack_count,
                max_duration_ms: aura.max_duration,
                remain_time_ms: aura.remaining_duration,
                remain_charges: aura.charges,
                cast_item_id: aura.cast_item_id,
                cast_item_level: aura.cast_item_level,
            });
            effect_rows.extend(aura.effects.iter().map(|effect| {
                crate::session::CharacterAuraEffectRowLikeCpp {
                    caster_guid,
                    item_guid,
                    spell_id: family.spell_id,
                    effect_mask,
                    effect_index: effect.index,
                    amount: effect.amount,
                    base_amount: effect.base_amount,
                }
            }));
        }
    }
    (aura_rows, effect_rows)
}

#[test]
fn nonzero_only_sickness_save_and_reload_preserves_original_key_and_payload() {
    let (mut session, player_guid, _) = canonical_aura_session([], SICKNESS_SPELL_ID);
    install_nonzero_only_sickness_data(&mut session);
    let item_guid = ObjectGuid::create_item(1, 88);
    let (mut aura, mut effects) = sickness_rows(player_guid, 234_567);
    aura.item_guid = item_guid;
    aura.difficulty = 3;
    aura.recalculate_mask = 0;
    aura.remain_charges = 2;
    aura.cast_item_id = 6_948;
    aura.cast_item_level = -17;
    aura.remain_time_ms = 123_456;
    for (effect, (amount, base_amount)) in effects.iter_mut().zip([(-71, -76), (-62, -67), (-3, -8)]) {
        effect.item_guid = item_guid;
        effect.amount = amount;
        effect.base_amount = base_amount;
    }

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], effects, 0),
        1,
        "typed aura/effect rows load from the exact nonzero SpellInfo key"
    );
    let saved = capture_player_save_request(&session)
        .expect("the complete Player save request is captured")
        .resurrection_sickness
        .expect("the selected sickness family is captured");
    assert_eq!(saved.spell_id, SICKNESS_SPELL_ID as u32);
    assert_eq!(saved.families.len(), 1);
    let family = &saved.families[0];
    assert_eq!(family.spell_id, SICKNESS_SPELL_ID as u32);
    assert_eq!(family.caster_guid_binary, player_guid.to_raw_bytes());
    assert_eq!(family.item_guid_binary, item_guid.to_raw_bytes());
    assert_eq!(family.auras.len(), 1);
    let saved_aura = &family.auras[0];
    assert_eq!(saved_aura.difficulty, 3);
    assert_eq!(
        (
            saved_aura.max_duration,
            saved_aura.remaining_duration,
            saved_aura.charges,
            saved_aura.cast_item_id,
            saved_aura.cast_item_level,
        ),
        (600_000_i32, 123_456_i32, 2_u8, 6_948_u32, -17_i32)
    );
    assert_eq!(
        saved_aura.effects,
        [
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 0,
                amount: -71,
                base_amount: -76,
            },
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 1,
                amount: -62,
                base_amount: -67,
            },
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 2,
                amount: -3,
                base_amount: -8,
            },
        ]
    );

    let (aura_rows, effect_rows) = rows_from_save(&saved);
    let (mut reloaded, _, _) = canonical_aura_session([], SICKNESS_SPELL_ID);
    install_nonzero_only_sickness_data(&mut reloaded);
    assert_eq!(
        reloaded.load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0),
        1
    );
    let reloaded_save = capture_player_save_request(&reloaded)
        .expect("the reloaded Player save request is captured")
        .resurrection_sickness
        .expect("the reloaded selected sickness family is captured");
    assert_eq!(reloaded_save, saved);
}

#[test]
fn nonzero_only_catalog_captures_a_complete_empty_sickness_family() {
    let (mut session, _, _) = canonical_aura_session([], SICKNESS_SPELL_ID);
    install_nonzero_only_sickness_data(&mut session);

    let saved = capture_player_save_request(&session)
        .expect("the complete Player save request is captured")
        .resurrection_sickness
        .expect("absence of selected applications is a complete empty family");
    assert_eq!(saved.spell_id, SICKNESS_SPELL_ID as u32);
    assert!(saved.families.is_empty());
}

#[test]
fn missing_original_sickness_spell_info_keeps_the_save_group_unavailable() {
    let (mut session, player_guid, _) = canonical_aura_session([], SICKNESS_SPELL_ID);
    install_nonzero_only_sickness_data(&mut session);
    let (mut aura, effects) = sickness_rows(player_guid, 90_000);
    aura.difficulty = 3;
    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], effects, 0),
        1
    );
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));

    assert!(
        capture_player_save_request(&session)
            .expect("other Player save fields remain capturable")
            .resurrection_sickness
            .is_none(),
        "an active aura with unavailable original SpellInfo cannot become an empty replacement"
    );
}
