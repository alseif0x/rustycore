use super::*;

fn set_aura_authority(fixture: &mut GhostFixture, complete: bool) {
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(complete);
        })
        .expect("canonical Player");
}

fn install_difficulties(fixture: &mut GhostFixture, ids: impl IntoIterator<Item = u32>) {
    fixture
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids(ids)));
}

fn captured_families(
    fixture: &mut GhostFixture,
) -> Option<Vec<wow_persistence::PlayerLifecycleAuraFamilySave>> {
    fixture
        .session
        .prepare_player_save_like_cpp(1_700_000_000)
        .expect("canonical full-save owner")
        .request
        .ghost_wisp
}

fn family(
    families: &[wow_persistence::PlayerLifecycleAuraFamilySave],
    spell_id: i32,
) -> &wow_persistence::PlayerLifecycleAuraFamilySave {
    families
        .iter()
        .find(|family| family.spell_id == spell_id as u32)
        .expect("selected Ghost/Wisp family")
}

fn loader_rows(
    families: &[wow_persistence::PlayerLifecycleAuraFamilySave],
    caster: ObjectGuid,
) -> (
    Vec<wow_world_spell::CharacterAuraRowLikeCpp>,
    Vec<wow_world_spell::CharacterAuraEffectRowLikeCpp>,
) {
    let mut aura_rows = Vec::new();
    let mut effect_rows = Vec::new();
    for family in families {
        for aura in &family.auras {
            let effect_mask = aura
                .effects
                .iter()
                .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index));
            aura_rows.push(wow_world_spell::CharacterAuraRowLikeCpp {
                caster_guid: caster,
                item_guid: ObjectGuid::EMPTY,
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
                wow_world_spell::CharacterAuraEffectRowLikeCpp {
                    caster_guid: caster,
                    item_guid: ObjectGuid::EMPTY,
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

#[tokio::test]
async fn ghost_wisp_persistence_captures_casts_and_roundtrips_loaded_metadata() {
    let mut source = fixture();
    install_difficulties(&mut source, [0, 1]);
    set_aura_authority(&mut source, true);
    cast_selected(&mut source, WISP, None).await;
    cast_selected(&mut source, GHOST, None).await;

    let captured = captured_families(&mut source).expect("complete aura authority and catalogs");
    assert_eq!(
        captured.iter().map(|family| family.spell_id).collect::<Vec<_>>(),
        [GHOST as u32, WISP as u32]
    );
    assert_eq!(family(&captured, GHOST).caster_guid_binary, source.player.to_raw_bytes());
    assert_eq!(family(&captured, WISP).caster_guid_binary, source.player.to_raw_bytes());

    let ghost = &family(&captured, GHOST).auras[0];
    assert_eq!(
        (
            ghost.difficulty,
            ghost.stack_count,
            ghost.max_duration,
            ghost.remaining_duration,
            ghost.charges,
            ghost.recalculate_mask,
            ghost.cast_item_id,
            ghost.cast_item_level,
        ),
        (0, 1, -1, -1, 0, 0b111, 0, -1)
    );
    assert_eq!(
        ghost
            .effects
            .iter()
            .map(|effect| (effect.index, effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(0, -20, -21), (1, 50, 49), (2, 50, 49)]
    );
    let wisp = &family(&captured, WISP).auras[0];
    assert_eq!(
        (
            wisp.difficulty,
            wisp.stack_count,
            wisp.max_duration,
            wisp.remaining_duration,
            wisp.charges,
            wisp.recalculate_mask,
            wisp.cast_item_id,
            wisp.cast_item_level,
        ),
        (0, 1, -1, -1, 0, 0b011, 0, -1)
    );
    assert_eq!(
        wisp.effects
            .iter()
            .map(|effect| (effect.index, effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(0, 75, 74), (1, 75, 74), (2, 0, -1)]
    );

    let mut persisted = captured.clone();
    for aura in persisted.iter_mut().flat_map(|family| &mut family.auras) {
        aura.difficulty = 1;
        aura.stack_count = 2;
        aura.charges = 3;
        aura.cast_item_id = 4_321;
        aura.cast_item_level = 77;
    }
    let mut restored = fixture();
    install_difficulties(&mut restored, [0, 1]);
    // The login aura loader receives authority only after both persisted
    // aura queries have completed successfully.
    set_aura_authority(&mut restored, true);
    let (aura_rows, effect_rows) = loader_rows(&persisted, restored.player);
    assert_eq!(
        restored.session.load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0),
        2
    );
    assert_eq!(
        crate::session::hub_ref(&restored.session)
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .persisted_player_aura_authority_complete_like_cpp()
            }),
        Some(true),
        "the loader itself must retain complete authority for supported Ghost/Wisp recalculation"
    );

    let recaptured = captured_families(&mut restored).expect("restored aura authority");
    let restored_ghost = &family(&recaptured, GHOST).auras[0];
    assert_eq!(
        (
            restored_ghost.difficulty,
            restored_ghost.stack_count,
            restored_ghost.charges,
            restored_ghost.recalculate_mask,
            restored_ghost.cast_item_id,
            restored_ghost.cast_item_level,
            restored_ghost.max_duration,
            restored_ghost.remaining_duration,
        ),
        (1, 2, 3, 0b111, 4_321, 77, -1, -1)
    );
    assert_eq!(
        restored_ghost
            .effects
            .iter()
            .map(|effect| (effect.index, effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(0, -40, -21), (1, 100, 49), (2, 100, 49)]
    );
    let restored_wisp = &family(&recaptured, WISP).auras[0];
    assert_eq!(
        (
            restored_wisp.difficulty,
            restored_wisp.stack_count,
            restored_wisp.charges,
            restored_wisp.recalculate_mask,
            restored_wisp.cast_item_id,
            restored_wisp.cast_item_level,
        ),
        (1, 2, 3, 0b011, 4_321, 77)
    );
    assert_eq!(
        restored_wisp
            .effects
            .iter()
            .map(|effect| (effect.index, effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(0, 150, 74), (1, 150, 74), (2, 0, -1)]
    );

    let slots = {
        let manager = restored.canonical.lock().expect("canonical manager");
        manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(restored.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .iter()
            .filter_map(|(&slot, aura)| matches!(aura.spell_id, GHOST | WISP).then_some(slot))
            .collect::<Vec<_>>()
    };
    for slot in slots {
        restored.session.remove_aura(slot).expect("remove selected aura");
    }
    let cleared = captured_families(&mut restored).expect("complete authority after removal");
    assert!(family(&cleared, GHOST).auras.is_empty());
    assert!(family(&cleared, WISP).auras.is_empty());
}

#[test]
fn ghost_wisp_persistence_preserves_unknown_and_emits_known_empty_families() {
    let mut complete_empty = fixture();
    install_difficulties(&mut complete_empty, [0]);
    set_aura_authority(&mut complete_empty, true);
    let empty = captured_families(&mut complete_empty).expect("complete empty snapshot");
    assert_eq!(empty.iter().map(|family| family.spell_id).collect::<Vec<_>>(), [8326, 20584]);
    assert!(empty.iter().all(|family| family.auras.is_empty()));

    let mut no_difficulty = fixture();
    set_aura_authority(&mut no_difficulty, true);
    assert!(captured_families(&mut no_difficulty).is_none());

    let mut missing_spell = fixture();
    install_difficulties(&mut missing_spell, [0]);
    set_aura_authority(&mut missing_spell, true);
    missing_spell
        .session
        .set_spell_store(Arc::new(wow_data::SpellStore::new()));
    assert!(captured_families(&mut missing_spell).is_none());

    let mut incomplete = fixture();
    install_difficulties(&mut incomplete, [0]);
    set_aura_authority(&mut incomplete, false);
    assert!(captured_families(&mut incomplete).is_none());
}

#[tokio::test]
async fn ghost_wisp_persistence_rejects_duplicate_keys_and_malformed_metadata() {
    let mut duplicate = fixture();
    install_difficulties(&mut duplicate, [0, 1]);
    set_aura_authority(&mut duplicate, true);
    cast_selected(&mut duplicate, GHOST, None).await;
    let (mut second_application, second_snapshot) = {
        let manager = duplicate.canonical.lock().expect("canonical manager");
        let auras = &manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(duplicate.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras;
        let application = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == GHOST)
            .expect("fresh Ghost application")
            .clone();
        let snapshot = auras
            .threat_snapshot_like_cpp(application.slot)
            .expect("fresh Ghost effect snapshot")
            .clone();
        (application, snapshot)
    };
    second_application.slot = 7;
    second_application.difficulty_id = 1;
    duplicate
        .session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.insert_runtime_application_like_cpp(second_application);
            auras.insert_threat_snapshot_like_cpp(7, second_snapshot);
        })
        .expect("canonical Player");
    assert!(captured_families(&mut duplicate).is_none());

    let mut malformed = fixture();
    install_difficulties(&mut malformed, [0]);
    set_aura_authority(&mut malformed, true);
    cast_selected(&mut malformed, GHOST, None).await;
    let ghost_slot = {
        let manager = malformed.canonical.lock().expect("canonical manager");
        manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(malformed.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .iter()
            .find_map(|(&slot, aura)| (aura.spell_id == GHOST).then_some(slot))
            .expect("fresh Ghost application")
    };
    malformed
        .session
        .with_owned_player_mut_like_cpp(|player| {
            assert!(player.unit_mut().subsystems_mut().auras.set_aura_persistence_metadata(
                ghost_slot,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: ObjectGuid::EMPTY,
                    cast_item_id: 0,
                    cast_item_level: -1,
                    recalculate_mask: 0b111,
                    charges: 0,
                    base_amounts: vec![(32, -21)],
                },
            ));
        })
        .expect("canonical Player");
    assert!(captured_families(&mut malformed).is_none());
}

#[tokio::test]
async fn ghost_wisp_persistence_ignores_foreign_caster_and_item_keys() {
    let mut fixture = fixture();
    install_difficulties(&mut fixture, [0]);
    set_aura_authority(&mut fixture, true);
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;

    let (wisp_slot, mut foreign_application, foreign_snapshot) = {
        let manager = fixture.canonical.lock().expect("canonical manager");
        let auras = &manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras;
        let wisp_slot = auras
            .runtime_applications_like_cpp()
            .iter()
            .find_map(|(&slot, aura)| (aura.spell_id == WISP).then_some(slot))
            .expect("self Wisp application");
        let ghost = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == GHOST)
            .expect("self Ghost application");
        (
            wisp_slot,
            ghost.clone(),
            auras
                .threat_snapshot_like_cpp(ghost.slot)
                .expect("Ghost effect snapshot")
                .clone(),
        )
    };
    let item_guid = ObjectGuid::create_item(1, 96_119);
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            assert!(auras.set_aura_persistence_metadata(
                wisp_slot,
                wow_entities::AuraPersistenceMetadata {
                    item_guid,
                    cast_item_id: 96_119,
                    cast_item_level: 70,
                    recalculate_mask: 0b011,
                    charges: 0,
                    base_amounts: vec![(0, 74), (1, 74), (2, -1)],
                },
            ));
            foreign_application.slot = 7;
            foreign_application.caster_guid = ObjectGuid::create_player(1, 96_118);
            auras.insert_runtime_application_like_cpp(foreign_application);
            auras.insert_threat_snapshot_like_cpp(7, foreign_snapshot);
        })
        .expect("canonical Player");

    let families = captured_families(&mut fixture).expect("complete aura authority");
    assert_eq!(family(&families, GHOST).auras.len(), 1);
    assert_eq!(family(&families, GHOST).auras[0].effects.len(), 3);
    assert!(family(&families, WISP).auras.is_empty());
}

#[tokio::test]
async fn ghost_wisp_persistence_emits_empty_family_for_known_expired_aura() {
    let mut fixture = fixture();
    install_difficulties(&mut fixture, [0]);
    set_aura_authority(&mut fixture, true);
    cast_selected(&mut fixture, GHOST, None).await;
    let slot = {
        let manager = fixture.canonical.lock().expect("canonical manager");
        manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .iter()
            .find_map(|(&slot, aura)| (aura.spell_id == GHOST).then_some(slot))
            .expect("fresh Ghost application")
    };
    fixture
        .session
        .with_owned_player_mut_like_cpp(|player| {
            let aura = player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(slot)
                .expect("Ghost application");
            aura.duration_total = 1_000;
            aura.duration_remaining = 1;
            aura.applied_at = Instant::now() - Duration::from_secs(1);
        })
        .expect("canonical Player");

    let families = captured_families(&mut fixture).expect("known expired application");
    assert!(family(&families, GHOST).auras.is_empty());
    assert!(family(&families, WISP).auras.is_empty());
}
