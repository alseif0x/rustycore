use super::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
enum CatalogShape {
    ThreeOnly,
    ValidZeroAndThree,
    ConflictingZeroShape,
    MissingWispKey,
    DifficultyCycle,
    MissingWispTransform,
}

const CURRENT_POLARITY_TRANSFORM: i32 = 99_013;
const CURRENT_POLARITY_TRANSFORM_TEMPLATE: u32 = 12_862;

fn spell_effect_row(
    id: u32,
    spell_id: i32,
    difficulty: u8,
    effect_index: i32,
    aura_type: i32,
    base_points: i32,
    misc_value: i32,
) -> wow_data::spell_db2::SpellEffectDb2Entry {
    wow_data::spell_db2::SpellEffectDb2Entry {
        id,
        difficulty_id: i32::from(difficulty),
        effect_index,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: i16::try_from(aura_type).expect("aura type fits DB2 field"),
        effect_aura_period: 0,
        effect_base_points: base_points,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 1,
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
        effect_misc_value: [misc_value, 0],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: u32::try_from(spell_id).expect("positive spell ID"),
    }
}

fn effect_rows(
    spell_id: i32,
    difficulty: u8,
    conflict: bool,
    mismatch_zero_shape: bool,
    omit_wisp_transform: bool,
) -> Vec<wow_data::spell_db2::SpellEffectDb2Entry> {
    let expected = if spell_id == GHOST {
        [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]
    } else {
        [
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        ]
    };
    let points = if spell_id == GHOST {
        [-21, 49, 49]
    } else {
        [74, 74, -1]
    };
    let mut rows = Vec::new();
    for index in 0..3 {
        if spell_id == WISP && omit_wisp_transform && index == 2 {
            continue;
        }
        let mismatched_shape =
            mismatch_zero_shape && difficulty == 0 && spell_id == GHOST && index == 0;
        let misc_value = if spell_id == WISP && index == 2 {
            WISP_TEMPLATE as i32
        } else {
            0
        };
        rows.push(spell_effect_row(
            200_000
                + u32::try_from(spell_id).expect("positive spell ID") * 10
                + u32::from(difficulty) * 3
                + index as u32,
            spell_id,
            difficulty,
            index as i32,
            if mismatched_shape { 0 } else { expected[index] },
            if conflict && difficulty == 0 {
                points[index] + 300
            } else {
                points[index]
            },
            misc_value,
        ));
    }
    rows
}

fn difficulty_entry(id: u32, fallback: u8) -> wow_data::DifficultyEntry {
    wow_data::DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: fallback,
        toggle_difficulty_id: 0,
    }
}

fn install_catalog(fixture: &mut GhostFixture, shape: CatalogShape) {
    install_catalog_with_current_transform(fixture, shape, None, false);
}

pub(super) fn install_current_polarity_catalog(
    fixture: &mut GhostFixture,
    current_spell_id: i32,
    current_has_three_key: bool,
) {
    install_catalog_with_current_transform(
        fixture,
        CatalogShape::ThreeOnly,
        Some(current_spell_id),
        current_has_three_key,
    );
}

fn install_catalog_with_current_transform(
    fixture: &mut GhostFixture,
    shape: CatalogShape,
    current_transform_spell: Option<i32>,
    current_has_three_key: bool,
) {
    let payload_difficulties: &[u8] = match shape {
        CatalogShape::ThreeOnly
        | CatalogShape::MissingWispKey
        | CatalogShape::MissingWispTransform => &[3],
        CatalogShape::ValidZeroAndThree | CatalogShape::ConflictingZeroShape => &[0, 3],
        CatalogShape::DifficultyCycle => &[0],
    };
    let mut misc_rows = Vec::new();
    let mut effect_rows_all = Vec::new();
    for spell_id in [GHOST, WISP] {
        for difficulty in payload_difficulties {
            let mut attributes = selected_aura_attributes(spell_id);
            if current_transform_spell.is_some() && spell_id == WISP {
                attributes[0] &= !0x0400_0000;
                attributes[4] |= 0x0000_1000;
            }
            misc_rows.push(wow_data::SpellMiscEntry {
                id: 300_000
                    + u32::try_from(spell_id).expect("positive spell ID") * 10
                    + u32::from(*difficulty),
                attributes: attributes.map(|attribute| attribute as i32),
                difficulty_id: *difficulty,
                duration_index: 21,
                spell_id: u32::try_from(spell_id).expect("positive spell ID"),
                ..Default::default()
            });
            effect_rows_all.extend(effect_rows(
                spell_id,
                *difficulty,
                matches!(shape, CatalogShape::ConflictingZeroShape),
                matches!(shape, CatalogShape::ConflictingZeroShape),
                spell_id == WISP && matches!(shape, CatalogShape::MissingWispTransform),
            ));
        }
    }

    if let Some(spell_id) = current_transform_spell {
        let current_difficulties: &[u8] = if current_has_three_key { &[0, 3] } else { &[0] };
        for difficulty in current_difficulties {
            let mut attributes = [0; 15];
            if *difficulty == 0 {
                attributes[4] = 0x0000_1000;
            } else {
                attributes[0] = 0x0400_0000;
            }
            misc_rows.push(wow_data::SpellMiscEntry {
                id: 400_000
                    + u32::try_from(spell_id).expect("positive current transform ID") * 10
                    + u32::from(*difficulty),
                attributes: attributes.map(|attribute| attribute as i32),
                difficulty_id: *difficulty,
                duration_index: 21,
                spell_id: u32::try_from(spell_id).expect("positive current transform ID"),
                ..Default::default()
            });
            effect_rows_all.push(spell_effect_row(
                400_000
                    + u32::try_from(spell_id).expect("positive current transform ID") * 10
                    + u32::from(*difficulty),
                spell_id,
                *difficulty,
                0,
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                0,
                CURRENT_POLARITY_TRANSFORM_TEMPLATE as i32,
            ));
        }
    }

    let keys = match shape {
        CatalogShape::ThreeOnly | CatalogShape::MissingWispTransform => {
            vec![(GHOST as u32, 3), (WISP as u32, 3)]
        }
        CatalogShape::ValidZeroAndThree | CatalogShape::ConflictingZeroShape => {
            vec![
                (GHOST as u32, 0),
                (GHOST as u32, 3),
                (WISP as u32, 0),
                (WISP as u32, 3),
            ]
        }
        CatalogShape::MissingWispKey => vec![(GHOST as u32, 3)],
        CatalogShape::DifficultyCycle => vec![(GHOST as u32, 0), (WISP as u32, 0)],
    };
    let mut keys = keys;
    if let Some(spell_id) = current_transform_spell {
        keys.push((spell_id as u32, 0));
        if current_has_three_key {
            keys.push((spell_id as u32, 3));
        }
    }
    let misc_store = wow_data::spell_db2::SpellMiscStore::from_entries(misc_rows.clone());
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        misc_store,
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effect_rows_all),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let spells = crate::selected_spell_fixtures::canonical_spell_store(keys)
        .hydrate_effective_core_db2_like_cpp(stores);
    fixture.session.set_spell_store(Arc::new(spells));
    fixture
        .session
        .set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    let difficulties = match shape {
        CatalogShape::DifficultyCycle => wow_data::DifficultyStore::from_entries([
            difficulty_entry(3, 4),
            difficulty_entry(4, 3),
        ]),
        _ => wow_data::DifficultyStore::from_entries([difficulty_entry(3, 0)]),
    };
    fixture.session.set_difficulty_store(Arc::new(difficulties));
}

pub(super) fn move_player_to_difficulty(fixture: &mut GhostFixture, difficulty: u8) {
    let handle = fixture
        .session
        .core
        .player_handle_like_cpp
        .expect("canonical Player owner");
    let position = fixture
        .session
        .player_position_like_cpp()
        .expect("Player position");
    let manager = fixture
        .session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let target = if difficulty == 0 {
        wow_map::MapKey::new(571, 0)
    } else {
        wow_map::MapKey::new(571, 30_000 + u32::from(difficulty))
    };
    let mut manager = manager.lock().expect("canonical MapManager");
    manager
        .detach_player_like_cpp(handle)
        .expect("detach Player from its current map");
    if difficulty != 0 {
        manager.create_map_entry(
            target.map_id,
            target.instance_id,
            difficulty,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
    }
    manager
        .attach_player_like_cpp(handle, target, position)
        .expect("attach Player to the requested difficulty map");
}

fn application(fixture: &GhostFixture, spell_id: i32) -> wow_entities::AuraApplicationLikeCpp {
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
        .values()
        .find(|aura| aura.spell_id == spell_id)
        .cloned()
        .expect("selected aura application")
}

fn reset_loaded_effect_state(fixture: &GhostFixture) {
    let mut manager = fixture.canonical.lock().expect("canonical manager");
    let player = manager
        .find_map_mut(571, 0)
        .expect("Player map")
        .map_mut()
        .get_typed_player_mut(fixture.player)
        .expect("canonical Player");
    player.remove_player_flag(wow_world_core::session::PLAYER_FLAGS_GHOST_LIKE_CPP);
    player
        .unit_mut()
        .set_server_side_ghost_visibility_like_cpp(1);
    player
        .unit_mut()
        .set_server_side_ghost_visibility_detect_like_cpp(1);
    player.unit_mut().set_display_id(1_000, true);
}

fn ghost_visibility(fixture: &GhostFixture) -> (u32, u32) {
    let manager = fixture.canonical.lock().expect("canonical manager");
    let unit = manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit();
    (
        unit.server_side_ghost_visibility(),
        unit.server_side_ghost_visibility_detect(),
    )
}

fn aura_update_pending(fixture: &GhostFixture, slot: u8) -> bool {
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
        .visible_auras_to_update
        .contains(&slot)
}

fn set_current_transform(fixture: &mut GhostFixture, spell_id: i32) {
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .apply_transform_aura_like_cpp(spell_id, true, None);
        })
        .expect("canonical Player owner");
}

fn current_transform(fixture: &GhostFixture) -> i32 {
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
        .transform_spell_like_cpp()
}

async fn wisp_at_original_difficulty_three() -> (GhostFixture, wow_entities::AuraApplicationLikeCpp)
{
    let mut fixture = fixture();
    install_catalog(&mut fixture, CatalogShape::ValidZeroAndThree);
    move_player_to_difficulty(&mut fixture, 3);
    cast_selected(&mut fixture, WISP, None).await;
    move_player_to_difficulty(&mut fixture, 0);
    assert_eq!(fixture.session.core.current_map_difficulty_id_like_cpp(), 0);
    let aura = application(&fixture, WISP);
    assert_eq!(aura.difficulty_id, 3);
    assert_eq!(application(&fixture, WISP).difficulty_id, 3);
    outgoing(&fixture);
    (fixture, aura)
}

#[tokio::test]
async fn three_only_catalog_selects_ghost_loaded_effects_and_removal() {
    let mut fixture = fixture();
    install_catalog(&mut fixture, CatalogShape::ThreeOnly);
    let spell_store = fixture.session.catalogs.spell_store().expect("SpellStore");
    for spell_id in [GHOST, WISP] {
        assert!(spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 3));
        assert!(!spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
        assert!(spell_store.get(spell_id).is_none());
    }

    let base_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let base_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    move_player_to_difficulty(&mut fixture, 3);
    cast_selected(&mut fixture, GHOST, None).await;
    move_player_to_difficulty(&mut fixture, 0);
    assert_eq!(fixture.session.core.current_map_difficulty_id_like_cpp(), 0);
    let ghost = application(&fixture, GHOST);
    assert_eq!(ghost.difficulty_id, 3);

    reset_loaded_effect_state(&fixture);
    fixture
        .session
        .player_aura_application_cx_like_cpp()
        .apply_loaded_ghost_wisp_aura(&ghost)
        .expect("restore Ghost effects from its original difficulty");
    assert!(is_ghost_flag_set(&fixture));
    assert_eq!(ghost_visibility(&fixture), (2, 2));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run * 1.5).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim * 1.5).abs() < 0.0001);

    fixture
        .session
        .remove_aura(ghost.slot)
        .expect("remove Ghost using its original difficulty");
    assert!(!is_ghost_flag_set(&fixture));
    assert_eq!(ghost_visibility(&fixture), (1, 1));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim).abs() < 0.0001);
}

#[tokio::test]
async fn ghost_wisp_consumers_ignore_conflicting_zero_metadata_on_the_current_map() {
    let mut fixture = fixture();
    install_catalog(&mut fixture, CatalogShape::ValidZeroAndThree);
    let spell_store = fixture.session.catalogs.spell_store().expect("SpellStore");
    for spell_id in [GHOST, WISP] {
        assert!(spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
        assert!(spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 3));
        assert!(spell_store.get(spell_id).is_some());
    }

    let base_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let base_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    move_player_to_difficulty(&mut fixture, 3);
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    move_player_to_difficulty(&mut fixture, 0);
    assert_eq!(fixture.session.core.current_map_difficulty_id_like_cpp(), 0);
    let wisp = application(&fixture, WISP);
    let ghost = application(&fixture, GHOST);
    assert_eq!((wisp.difficulty_id, ghost.difficulty_id), (3, 3));
    install_catalog(&mut fixture, CatalogShape::ConflictingZeroShape);

    reset_loaded_effect_state(&fixture);
    fixture
        .session
        .player_aura_application_cx_like_cpp()
        .apply_loaded_ghost_wisp_aura(&wisp)
        .expect("loaded Wisp selects the key from its stored difficulty");
    fixture
        .session
        .player_aura_application_cx_like_cpp()
        .apply_loaded_ghost_wisp_aura(&ghost)
        .expect("loaded Ghost selects the key from its stored difficulty");
    assert_eq!(player_display(&fixture), 1_825);
    assert!(is_ghost_flag_set(&fixture));
    assert_eq!(ghost_visibility(&fixture), (2, 2));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run * 1.75).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim * 1.75).abs() < 0.0001);

    fixture
        .session
        .remove_aura(wisp.slot)
        .expect("removal selects Wisp metadata from the stored difficulty");
    assert_eq!(player_display(&fixture), 1_000);
    fixture
        .session
        .remove_aura(ghost.slot)
        .expect("removal selects Ghost metadata from the stored difficulty");
    assert!(!is_ghost_flag_set(&fixture));
    assert_eq!(ghost_visibility(&fixture), (1, 1));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim).abs() < 0.0001);
}

#[test]
fn three_only_known_empty_families_need_no_zero_difficulty_payload() {
    let mut fixture = fixture();
    install_catalog(&mut fixture, CatalogShape::ThreeOnly);
    super::persistence::set_aura_authority(&mut fixture, true);
    let spell_store = fixture.session.catalogs.spell_store().expect("SpellStore");
    for spell_id in [GHOST, WISP] {
        assert!(spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 3));
        assert!(!spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
        assert!(spell_store.get(spell_id).is_none());
    }

    let families = super::persistence::captured_families(&mut fixture)
        .expect("known selected families remain authoritative when empty");
    assert_eq!(families.len(), 2);
    assert!(families.iter().all(|family| family.auras.is_empty()));
}

#[tokio::test]
async fn native_aura_loader_restores_ghost_and_wisp_from_original_difficulty() {
    let mut source = fixture();
    install_catalog(&mut source, CatalogShape::ThreeOnly);
    super::persistence::set_aura_authority(&mut source, true);
    move_player_to_difficulty(&mut source, 3);
    cast_selected(&mut source, WISP, None).await;
    cast_selected(&mut source, GHOST, None).await;

    let source_spell_store = source.session.catalogs.spell_store().expect("SpellStore");
    for spell_id in [GHOST, WISP] {
        assert!(source_spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 3));
        assert!(!source_spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
        assert!(source_spell_store.get(spell_id).is_none());
    }

    let families =
        super::persistence::captured_families(&mut source).expect("complete source aura authority");
    assert_eq!(families.len(), 2);
    assert!(
        families
            .iter()
            .all(|family| { family.auras.len() == 1 && family.auras[0].difficulty == 3 })
    );
    let (aura_rows, effect_rows) = super::persistence::loader_rows(&families);

    let mut restored = fixture();
    install_catalog(&mut restored, CatalogShape::ThreeOnly);
    super::persistence::set_aura_authority(&mut restored, true);
    let spell_store = restored.session.catalogs.spell_store().expect("SpellStore");
    for spell_id in [GHOST, WISP] {
        assert!(spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 3));
        assert!(!spell_store.contains_spell_info_exact_like_cpp(spell_id as u32, 0));
        assert!(spell_store.get(spell_id).is_none());
    }
    let base_run = player_speed(&restored, UnitMoveTypeLikeCpp::Run);
    let base_swim = player_speed(&restored, UnitMoveTypeLikeCpp::Swim);
    assert_eq!(
        restored.session.core.current_map_difficulty_id_like_cpp(),
        0
    );
    assert_eq!(
        restored
            .session
            .load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0,),
        2,
        "native persistence loading reaches both selected effect consumers"
    );

    let authority_complete = crate::session::hub_ref(&restored.session)
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .persisted_player_aura_authority_complete_like_cpp()
        });
    assert_eq!(authority_complete, Some(true));
    assert_eq!(application(&restored, GHOST).difficulty_id, 3);
    assert_eq!(application(&restored, WISP).difficulty_id, 3);
    assert!(is_ghost_flag_set(&restored));
    assert_eq!(ghost_visibility(&restored), (2, 2));
    assert_eq!(player_display(&restored), 1_825);
    assert!((player_speed(&restored, UnitMoveTypeLikeCpp::Run) - base_run * 1.75).abs() < 0.0001);
    assert!((player_speed(&restored, UnitMoveTypeLikeCpp::Swim) - base_swim * 1.75).abs() < 0.0001);
}

#[tokio::test]
async fn wisp_polarity_uses_its_cast_difficulty_for_current_transform() {
    let mut source = fixture();
    install_current_polarity_catalog(&mut source, CURRENT_POLARITY_TRANSFORM, true);
    super::persistence::set_aura_authority(&mut source, true);
    move_player_to_difficulty(&mut source, 3);
    cast_selected(&mut source, WISP, None).await;
    let source_spell_store = source.session.catalogs.spell_store().expect("SpellStore");
    assert!(source_spell_store.contains_spell_info_exact_like_cpp(WISP as u32, 3));
    assert!(!source_spell_store.contains_spell_info_exact_like_cpp(WISP as u32, 0));
    assert!(source_spell_store.get(WISP).is_none());
    let families = super::persistence::captured_families(&mut source)
        .expect("complete three-only source aura authority");
    let wisp_family = families
        .iter()
        .find(|family| family.spell_id == WISP as u32)
        .expect("Wisp persistence family");
    assert_eq!(wisp_family.auras.len(), 1);
    assert_eq!(wisp_family.auras[0].difficulty, 3);
    let (aura_rows, effect_rows) = super::persistence::loader_rows(&families);

    let mut restored = fixture();
    install_current_polarity_catalog(&mut restored, CURRENT_POLARITY_TRANSFORM, true);
    super::persistence::set_aura_authority(&mut restored, true);
    install_wisp_templates(
        &mut restored.session,
        &[
            (WISP_TEMPLATE, 1_825),
            (CURRENT_POLARITY_TRANSFORM_TEMPLATE, 1_826),
        ],
    );
    restored
        .session
        .apply_aura(CURRENT_POLARITY_TRANSFORM, restored.player, 0, 0x0100)
        .expect("apply canonical current transform Aura");
    assert_eq!(
        application(&restored, CURRENT_POLARITY_TRANSFORM).difficulty_id,
        0
    );
    assert_eq!(current_transform(&restored), CURRENT_POLARITY_TRANSFORM);
    restored
        .session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_display_id(1_826, true))
        .expect("canonical Player owner");
    let spell_store = restored.session.catalogs.spell_store().expect("SpellStore");
    assert!(spell_store.contains_spell_info_exact_like_cpp(CURRENT_POLARITY_TRANSFORM as u32, 0));
    assert!(spell_store.contains_spell_info_exact_like_cpp(CURRENT_POLARITY_TRANSFORM as u32, 3));
    assert!(spell_store.contains_spell_info_exact_like_cpp(WISP as u32, 3));
    assert!(!spell_store.contains_spell_info_exact_like_cpp(WISP as u32, 0));
    assert!(spell_store.get(WISP).is_none());
    assert_eq!(
        restored.session.core.current_map_difficulty_id_like_cpp(),
        0
    );

    assert_eq!(
        restored.session.load_represented_character_auras_like_cpp(
            aura_rows.clone(),
            effect_rows.clone(),
            0
        ),
        1
    );
    assert_eq!(application(&restored, WISP).difficulty_id, 3);
    assert_eq!(player_display(&restored), 1_826);
    assert_eq!(
        current_transform(&restored),
        CURRENT_POLARITY_TRANSFORM,
        "difficulty-3 current polarity is negative even though its difficulty-0 key is positive"
    );

    let wisp_slot = application(&restored, WISP).slot;
    {
        let mut manager = restored.canonical.lock().expect("canonical manager");
        manager
            .find_map_mut(571, 0)
            .expect("Player map")
            .map_mut()
            .get_typed_player_mut(restored.player)
            .expect("canonical Player")
            .unit_mut()
            .subsystems_mut()
            .auras
            .runtime_application_mut_like_cpp(wisp_slot)
            .expect("Wisp AuraApplication")
            .aura_flags = 0x0010;
    }
    let wisp_with_conflicting_aura_flags = application(&restored, WISP);
    restored
        .session
        .player_aura_application_cx_like_cpp()
        .apply_loaded_ghost_wisp_aura(&wisp_with_conflicting_aura_flags)
        .expect("selected metadata, not AuraApplication flags, classifies Wisp");
    assert_eq!(current_transform(&restored), CURRENT_POLARITY_TRANSFORM);
    assert_eq!(player_display(&restored), 1_826);

    let mut malformed = fixture();
    install_current_polarity_catalog(&mut malformed, CURRENT_POLARITY_TRANSFORM, false);
    super::persistence::set_aura_authority(&mut malformed, true);
    assert_eq!(
        malformed
            .session
            .load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0),
        1
    );
    set_current_transform(&mut malformed, CURRENT_POLARITY_TRANSFORM);
    malformed
        .session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_display_id(1_826, true))
        .expect("canonical Player owner");
    malformed
        .session
        .set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
            difficulty_entry(3, 4),
            difficulty_entry(4, 3),
        ])));
    let malformed_wisp = application(&malformed, WISP);
    assert!(
        malformed
            .session
            .player_aura_application_cx_like_cpp()
            .apply_loaded_ghost_wisp_aura(&malformed_wisp)
            .is_err(),
        "current Info lookup must reject a malformed fallback graph before transform mutation"
    );
    assert_eq!(current_transform(&malformed), CURRENT_POLARITY_TRANSFORM);
    assert_eq!(player_display(&malformed), 1_826);
}

#[tokio::test]
async fn ghost_wisp_removal_rejects_a_missing_original_key_before_mutation() {
    let (mut fixture, aura) = wisp_at_original_difficulty_three().await;
    install_catalog(&mut fixture, CatalogShape::MissingWispKey);
    let spell_store = fixture.session.catalogs.spell_store().expect("SpellStore");
    assert!(!spell_store.contains_spell_info_any_difficulty_like_cpp(WISP as u32));
    assert!(spell_store.get(WISP).is_none());
    let before_display = player_display(&fixture);
    let before_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let before_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    let before_pending = aura_update_pending(&fixture, aura.slot);

    assert!(
        fixture
            .session
            .player_aura_application_cx_like_cpp()
            .apply_loaded_ghost_wisp_aura(&aura)
            .is_err()
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(fixture.session.remove_aura(aura.slot).is_err());
    assert_eq!(application(&fixture, WISP).slot, aura.slot);
    assert_eq!(player_display(&fixture), before_display);
    assert_eq!(player_speed(&fixture, UnitMoveTypeLikeCpp::Run), before_run);
    assert_eq!(
        player_speed(&fixture, UnitMoveTypeLikeCpp::Swim),
        before_swim
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(outgoing(&fixture).is_empty());
}

#[tokio::test]
async fn ghost_wisp_removal_rejects_a_malformed_original_difficulty_graph_before_mutation() {
    let (mut fixture, aura) = wisp_at_original_difficulty_three().await;
    install_catalog(&mut fixture, CatalogShape::DifficultyCycle);
    let before_display = player_display(&fixture);
    let before_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let before_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    let before_pending = aura_update_pending(&fixture, aura.slot);

    assert!(
        fixture
            .session
            .player_aura_application_cx_like_cpp()
            .apply_loaded_ghost_wisp_aura(&aura)
            .is_err()
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(fixture.session.remove_aura(aura.slot).is_err());
    assert_eq!(application(&fixture, WISP).slot, aura.slot);
    assert_eq!(player_display(&fixture), before_display);
    assert_eq!(player_speed(&fixture, UnitMoveTypeLikeCpp::Run), before_run);
    assert_eq!(
        player_speed(&fixture, UnitMoveTypeLikeCpp::Swim),
        before_swim
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(outgoing(&fixture).is_empty());
}

#[tokio::test]
async fn ghost_wisp_removal_rejects_a_missing_active_effect_before_mutation() {
    let (mut fixture, aura) = wisp_at_original_difficulty_three().await;
    install_catalog(&mut fixture, CatalogShape::MissingWispTransform);
    let spell_store = fixture.session.catalogs.spell_store().expect("SpellStore");
    assert!(spell_store.contains_spell_info_exact_like_cpp(WISP as u32, 3));
    assert!(spell_store.get(WISP).is_none());
    let before_display = player_display(&fixture);
    let before_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let before_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    let before_pending = aura_update_pending(&fixture, aura.slot);

    assert!(
        fixture
            .session
            .player_aura_application_cx_like_cpp()
            .apply_loaded_ghost_wisp_aura(&aura)
            .is_err()
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(fixture.session.remove_aura(aura.slot).is_err());
    assert_eq!(application(&fixture, WISP).slot, aura.slot);
    assert_eq!(player_display(&fixture), before_display);
    assert_eq!(player_speed(&fixture, UnitMoveTypeLikeCpp::Run), before_run);
    assert_eq!(
        player_speed(&fixture, UnitMoveTypeLikeCpp::Swim),
        before_swim
    );
    assert_eq!(aura_update_pending(&fixture, aura.slot), before_pending);
    assert!(outgoing(&fixture).is_empty());
}
