use super::*;

fn keyed_rows(
    player: ObjectGuid,
    caster: ObjectGuid,
    item: ObjectGuid,
    effect_mask: u32,
    remaining_ms: i32,
) -> (
    crate::session::CharacterAuraRowLikeCpp,
    Vec<crate::session::CharacterAuraEffectRowLikeCpp>,
) {
    let (mut aura, mut effects) = sickness_rows(player, remaining_ms);
    aura.caster_guid = caster;
    aura.item_guid = item;
    aura.effect_mask = effect_mask;
    aura.recalculate_mask = effect_mask;
    for effect in &mut effects {
        effect.caster_guid = caster;
        effect.item_guid = item;
        effect.effect_mask = effect_mask;
    }
    (aura, effects)
}

fn authority(session: &WorldSession) -> Option<bool> {
    let (state, hub) = crate::session::split_spell_state_ref(session);
    state.resolved_player_aura_authority_complete_like_cpp(hub)
}

fn loaded_sickness(
    session: &WorldSession,
) -> Vec<(
    wow_entities::AuraApplicationLikeCpp,
    wow_entities::AuraPersistenceMetadata,
    wow_entities::AuraCastProvenanceLikeCpp,
)> {
    crate::session::hub_ref(session)
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            auras
                .runtime_applications_like_cpp()
                .values()
                .filter(|aura| aura.spell_id == SICKNESS_SPELL_ID)
                .map(|application| {
                    (
                        application.clone(),
                        auras
                            .aura_persistence_metadata(application.slot)
                            .expect("loaded selected aura metadata")
                            .clone(),
                        auras.aura_cast_provenance_like_cpp(application.slot),
                    )
                })
                .collect()
        })
        .expect("canonical Player owner")
}

#[test]
fn selected_sickness_load_joins_original_mask_and_uses_only_active_effects() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    session
        .set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries([])));
    let (mut aura, effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b1001, 90_000);
    aura.remain_charges = 3;

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], effects, 0),
        1
    );
    let loaded = loaded_sickness(&session);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0.effect_mask, 0b001);
    assert_eq!(loaded[0].0.represented_effect_amounts[0].amount, -75);
    assert_eq!(loaded[0].1.recalculate_mask, 0b001);
    assert_eq!(loaded[0].1.base_amounts, [(0, -76)]);
    assert_eq!(
        loaded[0].1.charges, 0,
        "an absent AuraOptions row clears charges"
    );
    assert_eq!(authority(&session), Some(true));
}

#[test]
fn selected_sickness_zero_active_mask_is_known_absence_after_cast_id_allocation() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (aura, effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b1000, 90_000);
    let before =
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(&session, SICKNESS_SPELL_ID)
            .expect("canonical Player cast sequence");

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], effects, 0),
        0
    );
    let after =
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(&session, SICKNESS_SPELL_ID)
            .expect("canonical Player cast sequence remains available");
    assert_eq!(after.counter(), before.counter() + 2);
    assert!(loaded_sickness(&session).is_empty());
    assert_eq!(authority(&session), Some(true));
    assert!(
        capture_player_save_request(&session)
            .unwrap()
            .resurrection_sickness
            .expect("catalog-complete normalized absence is authoritative")
            .families
            .is_empty()
    );
}

#[test]
fn selected_sickness_foreign_finite_item_replaces_and_saves_full_key() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let foreign = ObjectGuid::create_player(1, 99);
    let item = ObjectGuid::create_item(1, 88);
    let (first, mut first_effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    let (mut second, mut second_effects) = keyed_rows(player, foreign, item, 0b111, 70_000);
    second.cast_item_id = 1_234;
    second.cast_item_level = 44;
    for effect in &mut second_effects {
        effect.amount = -75;
    }
    first_effects.extend(second_effects);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([first, second], first_effects, 0,),
        2,
        "both rows pass TryCreate and install, even though the second replaces the first"
    );
    let loaded = loaded_sickness(&session);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0.caster_guid, foreign);
    assert_eq!(loaded[0].1.item_guid, item);
    assert_eq!(
        (loaded[0].1.cast_item_id, loaded[0].1.cast_item_level),
        (1_234, 44)
    );
    assert_eq!(
        loaded[0].0.aura_flags & 0x1,
        0,
        "foreign caster provenance stays visible"
    );

    let saved = capture_player_save_request(&session)
        .unwrap()
        .resurrection_sickness
        .expect("loaded foreign finite item key remains capturable");
    assert_eq!(saved.families.len(), 1);
    assert_eq!(saved.families[0].caster_guid_binary, foreign.to_raw_bytes());
    assert_eq!(saved.families[0].item_guid_binary, item.to_raw_bytes());
    assert_eq!(saved.families[0].auras[0].cast_item_id, 1_234);
}

#[test]
fn selected_sickness_finite_base_loads_stored_permanent_item_then_capture_skips_it() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let item = ObjectGuid::create_item(1, 89);
    let (mut aura, effects) = keyed_rows(player, player, item, 0b111, 55_000);
    aura.max_duration_ms = -1;
    aura.cast_item_id = 7_654;
    aura.cast_item_level = 50;

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], effects, 0),
        1,
        "CanBeSaved uses the finite SpellDuration base before SetLoadedState"
    );
    let loaded = loaded_sickness(&session);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0.duration_total, 0);
    assert_eq!(loaded[0].0.duration_remaining, 55_000);
    assert_eq!(loaded[0].1.item_guid, item);
    let saved = capture_player_save_request(&session)
        .unwrap()
        .resurrection_sickness
        .expect("selected snapshot remains complete after source item skip");
    assert!(saved.families.is_empty());
}

#[test]
fn selected_sickness_load_allocates_each_cast_and_last_same_caster_row_wins() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let item_a = ObjectGuid::create_item(1, 90);
    let item_b = ObjectGuid::create_item(1, 91);
    let (first, mut effects) = keyed_rows(player, player, item_a, 0b111, 80_000);
    let (mut second, second_effects) = keyed_rows(player, player, item_b, 0b111, 70_000);
    second.cast_item_id = 2_222;
    effects.extend(second_effects);
    let before =
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(&session, SICKNESS_SPELL_ID)
            .expect("canonical Player cast sequence");

    assert_eq!(
        session.load_represented_character_auras_like_cpp([first, second], effects, 0),
        2,
        "both TryCreate rows load even though the second removes the first"
    );
    let after =
        wow_world_spell::next_represented_spell_cast_guid_like_cpp(&session, SICKNESS_SPELL_ID)
            .expect("canonical Player cast sequence remains available");
    let loaded = loaded_sickness(&session);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].1.item_guid, item_b);
    assert_eq!(loaded[0].1.cast_item_id, 2_222);
    assert_eq!(loaded[0].2.cast_id.counter(), before.counter() + 2);
    assert_eq!(after.counter(), before.counter() + 3);
}

#[test]
fn selected_sickness_rejected_foreign_candidate_retires_prior_spell_application() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(SICKNESS_SPELL_ID, sickness_catalog_info());
    let mut attributes = [0; 15];
    attributes[5] = 0x20;
    spell_store.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, attributes);
    session.set_spell_store(Arc::new(spell_store));
    let foreign = ObjectGuid::create_player(1, 100);
    let (first, mut effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    let (second, second_effects) = keyed_rows(player, foreign, ObjectGuid::EMPTY, 0b111, 70_000);
    effects.extend(second_effects);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([first, second], effects, 0),
        1
    );
    assert!(loaded_sickness(&session).is_empty());
    assert_eq!(authority(&session), Some(true));
    assert!(
        capture_player_save_request(&session)
            .unwrap()
            .resurrection_sickness
            .expect("known CanBeSaved exclusion authorizes absence")
            .families
            .is_empty()
    );
}

#[test]
fn selected_sickness_unknown_duration_or_zero_stored_max_keeps_save_unknown() {
    let (mut missing_duration, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    missing_duration
        .set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([])));
    let (aura, effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        missing_duration.load_represented_character_auras_like_cpp([aura], effects, 0),
        0
    );
    assert!(loaded_sickness(&missing_duration).is_empty());
    assert_eq!(authority(&missing_duration), Some(false));
    assert!(
        capture_player_save_request(&missing_duration)
            .unwrap()
            .resurrection_sickness
            .is_none()
    );

    let (mut zero_max, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (mut aura, effects) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    aura.max_duration_ms = 0;
    assert_eq!(
        zero_max.load_represented_character_auras_like_cpp([aura], effects, 0),
        0
    );
    assert!(loaded_sickness(&zero_max).is_empty());
    assert_eq!(authority(&zero_max), Some(false));
    assert!(
        capture_player_save_request(&zero_max)
            .unwrap()
            .resurrection_sickness
            .is_none()
    );
}
