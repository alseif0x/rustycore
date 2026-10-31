use super::*;

fn completion_cooldown_store(
    spell_id: i32,
    category_rows: impl IntoIterator<Item = wow_data::spell_db2::SpellCategoriesEntry>,
    cooldown_rows: impl IntoIterator<Item = wow_data::spell_db2::SpellCooldownsEntry>,
) -> Arc<wow_data::SpellStore> {
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries(category_rows),
        wow_data::spell_db2::SpellMiscStore::from_entries([wow_data::spell_db2::SpellMiscEntry {
            id: 54_104,
            spell_id: spell_id as u32,
            difficulty_id: 0,
            ..Default::default()
        }]),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries([]),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries(cooldown_rows),
        wow_data::SpellCastingRequirementsStore::from_entries([]),
        wow_data::SpellPowerStore::from_entries([]),
        wow_data::SpellPowerDifficultyStore::from_entries([]),
    );
    Arc::new(
        crate::selected_spell_fixtures::canonical_spell_store([(spell_id as u32, 0)])
            .hydrate_effective_core_db2_like_cpp(stores),
    )
}

fn completion_session(
    spell_store: Option<Arc<wow_data::SpellStore>>,
    player_counter: i64,
) -> (
    crate::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
) {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, player_counter);
    let canonical = shared_canonical_map_manager();
    install_canonical_player(&mut session, &canonical, player_guid);
    if let Some(spell_store) = spell_store {
        session.set_spell_store(spell_store);
    }
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    (session, send_rx, player_guid)
}

async fn execute_client_completion(
    session: &mut crate::session::WorldSession,
    spell_id: i32,
    player_guid: ObjectGuid,
    actual_spell_difficulty_id: Option<u8>,
) {
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8_690);
    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            cast_id,
            SpellCastVisual::default(),
            SpellTargetData::default(),
            SpellCastMetadata {
                from_client: true,
                client_cast_id: Some(cast_id),
                actual_spell_difficulty_id_like_cpp: actual_spell_difficulty_id,
                ..Default::default()
            },
        )
        .await
        .expect("client cast completion executes");
}

fn completion_history(session: &crate::session::WorldSession) -> wow_entities::SpellHistory {
    session
        .core
        .with_owned_player_like_cpp(|player| player.unit().subsystems().spells.history.clone())
        .expect("canonical Player spell history")
}

#[tokio::test]
async fn client_completion_records_hearthstone_category_from_selected_db2_rows_like_cpp() {
    let spell_id = 8_690;
    let store = completion_cooldown_store(
        spell_id,
        [wow_data::spell_db2::SpellCategoriesEntry {
            id: 72_843,
            difficulty_id: 0,
            category: 1_176,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: spell_id as u32,
        }],
        [wow_data::spell_db2::SpellCooldownsEntry {
            id: 54_105,
            difficulty_id: 0,
            category_recovery_time: 1_800_000,
            recovery_time: 1_800_000,
            start_recovery_time: 0,
            spell_id: spell_id as u32,
        }],
    );
    let (mut session, send_rx, player_guid) = completion_session(Some(store), 108_690);
    let before_secs = wow_world_core::session::connection_identity::unix_now();

    execute_client_completion(&mut session, spell_id, player_guid, Some(0)).await;

    let after_secs = wow_world_core::session::connection_identity::unix_now();
    let history = completion_history(&session);
    let cooldown = history
        .cooldowns
        .get(&(spell_id as u32))
        .expect("completed client cast records its cooldown");
    assert_eq!(cooldown.category_id, 1_176);
    assert!(cooldown.category_end_ms > 0);
    assert_eq!(history.category_cooldowns.get(&1_176), Some(&8_690));
    assert!(cooldown.cooldown_end_ms >= ((before_secs + 1_800) as u64) * 1_000);
    assert!(cooldown.cooldown_end_ms <= ((after_secs + 1_800) as u64) * 1_000);
    assert!(cooldown.category_end_ms >= ((before_secs + 1_800) as u64) * 1_000);
    assert!(cooldown.category_end_ms <= ((after_secs + 1_800) as u64) * 1_000);
    assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::CooldownEvent));
}

#[tokio::test]
async fn client_completion_missing_difficulty_is_fail_closed_and_keeps_cooldown_event_like_cpp() {
    let spell_id = 8_690;
    let store = completion_cooldown_store(
        spell_id,
        [wow_data::spell_db2::SpellCategoriesEntry {
            id: 72_843,
            difficulty_id: 0,
            category: 1_176,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: spell_id as u32,
        }],
        [wow_data::spell_db2::SpellCooldownsEntry {
            id: 54_105,
            difficulty_id: 0,
            category_recovery_time: 1_800_000,
            recovery_time: 1_800_000,
            start_recovery_time: 0,
            spell_id: spell_id as u32,
        }],
    );
    let (mut session, send_rx, player_guid) = completion_session(Some(store), 108_691);

    execute_client_completion(&mut session, spell_id, player_guid, None).await;

    let history = completion_history(&session);
    assert!(!history.cooldowns.contains_key(&(spell_id as u32)));
    assert!(history.category_cooldowns.is_empty());
    assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::CooldownEvent));
}

#[tokio::test]
async fn client_completion_loaded_empty_category_catalog_records_category_zero_like_cpp() {
    let spell_id = 18_690;
    let store = completion_cooldown_store(
        spell_id,
        [],
        [wow_data::spell_db2::SpellCooldownsEntry {
            id: 54_106,
            difficulty_id: 0,
            category_recovery_time: 0,
            recovery_time: 5_000,
            start_recovery_time: 0,
            spell_id: spell_id as u32,
        }],
    );
    let (mut session, send_rx, player_guid) = completion_session(Some(store), 108_692);
    let before_secs = wow_world_core::session::connection_identity::unix_now();

    execute_client_completion(&mut session, spell_id, player_guid, Some(0)).await;

    let after_secs = wow_world_core::session::connection_identity::unix_now();
    let history = completion_history(&session);
    let cooldown = history
        .cooldowns
        .get(&(spell_id as u32))
        .expect("a loaded empty category catalog is a native category-zero answer");
    assert_eq!(cooldown.category_id, 0);
    assert!(cooldown.category_end_ms > 0);
    assert!(history.category_cooldowns.is_empty());
    assert!(cooldown.cooldown_end_ms >= ((before_secs + 5) as u64) * 1_000);
    assert!(cooldown.cooldown_end_ms <= ((after_secs + 5) as u64) * 1_000);
    assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::CooldownEvent));
}

#[test]
fn cast_cooldown_record_rejects_missing_key_store_or_unhydrated_catalogs_like_cpp() {
    let spell_id = 8_690;
    let store = completion_cooldown_store(
        spell_id,
        [wow_data::spell_db2::SpellCategoriesEntry {
            id: 72_843,
            difficulty_id: 0,
            category: 1_176,
            defense_type: 0,
            dispel_type: 0,
            mechanic: 0,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category: 0,
            spell_id: spell_id as u32,
        }],
        [wow_data::spell_db2::SpellCooldownsEntry {
            id: 54_105,
            difficulty_id: 0,
            category_recovery_time: 1_800_000,
            recovery_time: 1_800_000,
            start_recovery_time: 0,
            spell_id: spell_id as u32,
        }],
    );
    let unhydrated = Arc::new(crate::selected_spell_fixtures::canonical_spell_store([(
        spell_id as u32,
        0,
    )]));
    for (index, recorded_spell_id, difficulty, candidate_store) in [
        (0, spell_id, Some(1), Some(Arc::clone(&store))),
        (1, spell_id, Some(0), None),
        (2, spell_id, Some(0), Some(unhydrated)),
        (3, 45_454, Some(0), Some(Arc::clone(&store))),
    ] {
        let (mut session, _send_rx, _player_guid) =
            completion_session(candidate_store, 109_000 + index);
        let (state, mut hub) = crate::session::split_spell_state_mut(&mut session);
        state.record_cast_character_spell_cooldown_like_cpp(
            &mut hub,
            recorded_spell_id,
            difficulty,
            1_800_000,
        );

        let history = completion_history(&session);
        assert!(!history.cooldowns.contains_key(&(recorded_spell_id as u32)));
        assert!(history.category_cooldowns.is_empty());
    }
}
