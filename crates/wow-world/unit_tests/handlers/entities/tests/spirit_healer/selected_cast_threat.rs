use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_data::spell::attributes::{SPELL_ATTR1_NO_THREAT, SPELL_ATTR4_NO_HELPFUL_THREAT};
use wow_data::spell::aura_types::{
    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, SPELL_AURA_MOD_PERCENT_STAT, SPELL_AURA_NO_PVP_CREDIT,
};
use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

use super::selected_execution_effects::{
    cast_sickness, complete_difficulty_chain, difficulty, effect_row,
    selected_session_with_raw_misc_rows, selected_session_with_spell_attributes,
};

const SPELL_ATTR0_AURA_IS_DEBUFF: u32 = 0x0400_0000;

fn positive_sickness_effect_rows() -> Vec<wow_data::spell_db2::SpellEffectDb2Entry> {
    [
        (0, SPELL_AURA_MOD_PERCENT_STAT, 20),
        (1, SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, 30),
        (2, SPELL_AURA_NO_PVP_CREDIT, 40),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (index, aura, points))| {
        let mut row = effect_row(
            50 + id as u32,
            0,
            index,
            SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
        );
        row.implicit_target = [1, 0];
        row
    })
    .collect()
}

pub(super) fn install_known_positive_class(session: &mut WorldSession) {
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([wow_data::SpellClassOptionsEntry {
            id: 1,
            spell_id: SICKNESS_SPELL,
            modal_next_spell: 0,
            spell_class_set: 4,
            spell_class_mask: [0; 4],
        }]),
    ));
}

fn install_initial_threat_entry(session: &mut WorldSession) {
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            SICKNESS_SPELL as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 7,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        )]),
    }));
}

fn add_canonical_creature_threatening_player(
    session: &mut WorldSession,
    counter: i64,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("selected canonical Player");
    let map_key = session
        .core
        .current_canonical_player_map_key_like_cpp()
        .expect("Player's canonical map key");
    let map_id = u16::try_from(map_key.map_id).expect("test map id fits ObjectGuid");
    let guid = ObjectGuid::create_world_object(
        HighGuid::Creature,
        0,
        1,
        map_id,
        map_key.instance_id,
        9001,
        counter,
    );
    let position = session.player_position_like_cpp().expect("Player position");
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9001);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_key.map_id, map_key.instance_id)
        .expect("creature map key");
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_ai_identity_runtime(1, 35, 0, 0);

    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let mut manager = manager.lock().expect("canonical MapManager lock");
    let map = manager
        .find_map_mut(map_key.map_id, map_key.instance_id)
        .expect("Player's canonical Map")
        .map_mut();
    map.insert_map_object_record(
        wow_entities::MapObjectRecord::new_creature(creature)
            .expect("valid canonical creature record"),
    )
    .expect("canonical creature admission");
    let threat_ref = {
        let creature = map
            .get_typed_creature_mut(guid)
            .expect("admitted canonical Creature");
        creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_threat(player_guid, 25.0);
        *creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(player_guid)
            .expect("Creature threat reference")
    };
    map.get_typed_player_mut(player_guid)
        .expect("canonical Player in the same Map")
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(guid, threat_ref);
    guid
}

fn canonical_creature_threat(
    session: &WorldSession,
    creature_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> Option<f32> {
    let map_key = session
        .core
        .current_canonical_player_map_key_like_cpp()
        .expect("Player's canonical map key");
    session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .lock()
        .expect("canonical MapManager lock")
        .find_map(map_key.map_id, map_key.instance_id)
        .expect("Player's canonical Map")
        .map()
        .get_typed_creature(creature_guid)
        .expect("canonical Creature")
        .unit()
        .subsystems()
        .combat
        .threat_value(player_guid)
}

pub(super) fn drop_class_options_for_refresh(session: &mut WorldSession) {
    let catalogs = std::mem::take(&mut session.catalogs.spell_catalogs);
    let spell_store = catalogs.spell_store.clone();
    let serverside_spell_store = catalogs.serverside_spell_store.clone();
    let spell_misc_store = catalogs.spell_misc_store.clone();
    let spell_duration_store = catalogs.spell_duration_store.clone();
    let spell_aura_options_store = catalogs.spell_aura_options_store.clone();
    let spell_levels_store = catalogs.spell_levels_store.clone();
    let spell_proc_store = catalogs.spell_proc_store.clone();
    drop(catalogs);

    let catalogs = &mut session.catalogs.spell_catalogs;
    catalogs.spell_store = spell_store;
    catalogs.serverside_spell_store = serverside_spell_store;
    catalogs.spell_misc_store = spell_misc_store;
    catalogs.spell_duration_store = spell_duration_store;
    catalogs.spell_aura_options_store = spell_aura_options_store;
    catalogs.spell_levels_store = spell_levels_store;
    catalogs.spell_proc_store = spell_proc_store;
}

#[tokio::test]
async fn selected_debuff_attributes_override_positive_effect_amounts_for_initial_threat() {
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    let (mut session, _send_rx, player_guid) = selected_session_with_spell_attributes(
        50_101,
        4,
        positive_sickness_effect_rows(),
        complete_difficulty_chain(),
        attributes,
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let creature_guid = add_canonical_creature_threatening_player(&mut session, 50_102);
    install_initial_threat_entry(&mut session);

    cast_sickness(&mut session, player_guid, 50_103)
        .await
        .expect("selected sickness cast completes");
    session.update(0).await;

    assert_eq!(
        canonical_creature_threat(&session, creature_guid, player_guid),
        Some(25.0),
        "SpellAttr0 AURA_IS_DEBUFF makes the positive-amount spell harmful as a whole"
    );
}

#[tokio::test]
async fn selected_positive_polarity_forwards_initial_threat_to_assisting_creature() {
    let (mut session, _send_rx, player_guid) = selected_session_with_spell_attributes(
        50_201,
        4,
        positive_sickness_effect_rows(),
        complete_difficulty_chain(),
        [0; 15],
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    install_known_positive_class(&mut session);
    let creature_guid = add_canonical_creature_threatening_player(&mut session, 50_202);
    install_initial_threat_entry(&mut session);

    cast_sickness(&mut session, player_guid, 50_203)
        .await
        .expect("selected sickness cast completes");
    session.update(0).await;

    assert_eq!(
        canonical_creature_threat(&session, creature_guid, player_guid),
        Some(39.0),
        "known positive Warrior-family effects forward 7 * 2 initial threat"
    );
}

#[tokio::test]
async fn selected_helpful_threat_attribute_follows_zero_to_three_fallback() {
    let mut attributes = [0; 15];
    attributes[4] = SPELL_ATTR4_NO_HELPFUL_THREAT;
    let misc = wow_data::SpellMiscEntry {
        id: SICKNESS_SPELL as u32 * 10 + 3,
        spell_id: SICKNESS_SPELL as u32,
        difficulty_id: 3,
        duration_index: 427,
        attributes: attributes.map(|attribute| attribute as i32),
        ..Default::default()
    };
    let (mut session, _send_rx, player_guid) = selected_session_with_raw_misc_rows(
        50_301,
        0,
        positive_sickness_effect_rows(),
        vec![difficulty(0, 3), difficulty(3, 0)],
        vec![misc],
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    install_known_positive_class(&mut session);
    let creature_guid = add_canonical_creature_threatening_player(&mut session, 50_302);
    install_initial_threat_entry(&mut session);
    let spell_store = session.catalogs.spell_store().expect("selected SpellStore");
    assert_eq!(
        spell_store.misc_attributes_for_exact_difficulty(SICKNESS_SPELL, 0),
        None,
        "no sickness SpellMisc attributes exist at map difficulty zero"
    );
    assert_eq!(
        spell_store.misc_attributes_for_exact_difficulty(SICKNESS_SPELL, 3),
        Some(attributes),
        "NO_HELPFUL_THREAT exists only on the difficulty-three row"
    );
    assert_eq!(
        spell_store.misc_attributes_for_difficulty_like_cpp(
            SICKNESS_SPELL,
            0,
            session.catalogs.difficulty_store().map(AsRef::as_ref),
        ),
        None,
        "the legacy getter stops at difficulty zero instead of following 0-to-3"
    );

    cast_sickness(&mut session, player_guid, 50_303)
        .await
        .expect("selected sickness cast completes");
    session.update(0).await;

    assert_eq!(
        canonical_creature_threat(&session, creature_guid, player_guid),
        Some(25.0),
        "selected 0-to-3 attributes prevent forwarding the positive cast bonus"
    );
}

#[tokio::test]
async fn source_no_threat_refresh_ignores_unavailable_creation_metadata() {
    let mut attributes = [0; 15];
    attributes[1] = SPELL_ATTR1_NO_THREAT;
    let effects = positive_sickness_effect_rows();
    let (mut session, send_rx, player_guid) = selected_session_with_spell_attributes(
        50_401,
        4,
        effects,
        complete_difficulty_chain(),
        attributes,
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    install_known_positive_class(&mut session);
    cast_sickness(&mut session, player_guid, 50_402)
        .await
        .expect("first selected sickness cast creates its Aura from known metadata");
    session.update(0).await;
    let before = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("first selected cast created a full-mask sickness application");
    assert_ne!(
        before.aura_flags & 0x0100,
        0,
        "the real positive-polarity cast created a positive application"
    );
    let before_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(before.slot)
        })
        .expect("canonical Player provenance");
    let _ = drain_packets(&send_rx);

    assert!(session.catalogs.spell_class_options_store().is_some());
    drop_class_options_for_refresh(&mut session);

    assert!(
        session.catalogs.spell_class_options_store().is_none(),
        "taking and restoring the existing public catalog Arcs leaves class options unknown"
    );
    cast_sickness(&mut session, player_guid, 50_403)
        .await
        .expect("matching refresh is allowed when creation-only metadata is missing");
    session.update(0).await;

    let after = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("refreshed full-mask sickness application");
    let after_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(after.slot)
        })
        .expect("canonical Player provenance after refresh");
    assert_eq!(after.slot, before.slot);
    assert_eq!(after.effect_mask, 0b111);
    assert_eq!(after.aura_flags, before.aura_flags);
    assert_eq!(after.caster_guid, before.caster_guid);
    assert_eq!(after_provenance.cast_id, before_provenance.cast_id);
    assert_eq!(
        after_provenance.spell_visual_id,
        before_provenance.spell_visual_id
    );
    assert_eq!(
        after_provenance.caster_level,
        before_provenance.caster_level
    );
}

#[tokio::test]
async fn selected_cast_requires_initial_threat_polarity_before_spell_go() {
    let (mut session, send_rx, player_guid) = selected_session_with_spell_attributes(
        50_501,
        4,
        positive_sickness_effect_rows(),
        complete_difficulty_chain(),
        [0; 15],
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    drop_class_options_for_refresh(&mut session);
    assert!(session.catalogs.spell_class_options_store().is_none());
    install_initial_threat_entry(&mut session);

    assert_eq!(
        cast_sickness(&mut session, player_guid, 50_502).await,
        Err("Unable to resolve selected cast polarity metadata")
    );
    assert!(drain_packets(&send_rx).is_empty());
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
}

#[tokio::test]
async fn selected_cast_without_a_spell_info_key_fails_before_spell_go() {
    let (mut session, send_rx, player_guid) = selected_session_with_spell_attributes(
        50_601,
        4,
        positive_sickness_effect_rows(),
        vec![difficulty(0, 0), difficulty(3, 0)],
        [0; 15],
    )
    .await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    assert_eq!(session.core.current_map_difficulty_id_like_cpp(), 4);
    assert!(
        session
            .catalogs
            .difficulty_store()
            .is_some_and(|store| store.get(4).is_none()),
        "map difficulty four is missing from the known DifficultyStore"
    );
    assert!(
        session
            .catalogs
            .spell_store()
            .and_then(|store| store.effect_rows_for_exact_difficulty(SICKNESS_SPELL, 0))
            .is_some_and(|rows| !rows.is_empty()),
        "positive base effect rows exist at difficulty zero"
    );

    assert_eq!(
        cast_sickness(&mut session, player_guid, 50_602).await,
        Err("Spell not found")
    );
    assert!(drain_packets(&send_rx).is_empty());
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
}
