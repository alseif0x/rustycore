use super::*;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_data::spell::aura_types::{
    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, SPELL_AURA_MOD_PERCENT_STAT, SPELL_AURA_NO_PVP_CREDIT,
};
use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
use wow_data::spell_db2::SpellEffectDb2Entry;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};

pub(super) fn effect_row(
    id: u32,
    difficulty_id: u8,
    effect_index: u32,
    effect: u32,
    aura: i32,
    base_points: i32,
) -> SpellEffectDb2Entry {
    let misc_value = match effect_index {
        0 => -1,
        _ => 127,
    };
    SpellEffectDb2Entry {
        id,
        difficulty_id: i32::from(difficulty_id),
        effect_index: i32::try_from(effect_index).expect("fixture effect index"),
        effect,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: i16::try_from(aura).expect("fixture aura type"),
        effect_aura_period: 0,
        effect_base_points: base_points,
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
        effect_misc_value: [misc_value, 0],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: SICKNESS_SPELL as u32,
    }
}

fn base_effect_rows() -> Vec<SpellEffectDb2Entry> {
    [
        (0, SPELL_AURA_MOD_PERCENT_STAT, -10),
        (1, SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, -20),
        (2, SPELL_AURA_NO_PVP_CREDIT, -30),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (index, aura, points))| {
        effect_row(
            id as u32 + 1,
            0,
            index,
            SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
        )
    })
    .collect()
}

pub(super) fn difficulty(id: u32, fallback: u8) -> wow_data::DifficultyEntry {
    wow_data::DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: fallback,
        toggle_difficulty_id: 0,
    }
}

async fn selected_session(
    guid_counter: i64,
    map_difficulty: u8,
    effect_rows: Vec<SpellEffectDb2Entry>,
    difficulty_rows: Vec<wow_data::DifficultyEntry>,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    selected_session_with_inputs(
        guid_counter,
        map_difficulty,
        effect_rows,
        difficulty_rows,
        None,
        None,
    )
    .await
}

pub(super) async fn selected_session_with_spell_attributes(
    guid_counter: i64,
    map_difficulty: u8,
    effect_rows: Vec<SpellEffectDb2Entry>,
    difficulty_rows: Vec<wow_data::DifficultyEntry>,
    attributes: [u32; 15],
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    selected_session_with_inputs(
        guid_counter,
        map_difficulty,
        effect_rows,
        difficulty_rows,
        Some(attributes),
        None,
    )
    .await
}

pub(super) async fn selected_session_with_raw_misc_rows(
    guid_counter: i64,
    map_difficulty: u8,
    effect_rows: Vec<SpellEffectDb2Entry>,
    difficulty_rows: Vec<wow_data::DifficultyEntry>,
    raw_misc_rows: Vec<wow_data::SpellMiscEntry>,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    selected_session_with_inputs(
        guid_counter,
        map_difficulty,
        effect_rows,
        difficulty_rows,
        None,
        Some(raw_misc_rows),
    )
    .await
}

async fn selected_session_with_inputs(
    guid_counter: i64,
    map_difficulty: u8,
    effect_rows: Vec<SpellEffectDb2Entry>,
    difficulty_rows: Vec<wow_data::DifficultyEntry>,
    attributes_override: Option<[u32; 15]>,
    raw_misc_rows: Option<Vec<wow_data::SpellMiscEntry>>,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(guid_counter).await;
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));

    let mut misc_rows = crate::session::hub_ref(&session)
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .map(|store| store.entries_like_cpp().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(raw_misc_rows) = raw_misc_rows {
        misc_rows.retain(|row| row.spell_id != SICKNESS_SPELL as u32);
        misc_rows.extend(raw_misc_rows);
    } else if let Some(attributes) = attributes_override {
        let misc = misc_rows
            .iter_mut()
            .find(|row| row.spell_id == SICKNESS_SPELL as u32 && row.difficulty_id == 0)
            .expect("selected sickness SpellMisc fixture row");
        misc.attributes = attributes.map(|attribute| attribute as i32);
    }
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
    session.set_spell_store(Arc::new(
        wow_data::SpellStore::new().hydrate_effective_core_db2_like_cpp(stores),
    ));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries(
        difficulty_rows,
    )));
    install_player_on_difficulty_map(&mut session, map_difficulty);

    assert_eq!(
        session.core.current_map_difficulty_id_like_cpp(),
        map_difficulty,
        "the selected cast reads the difficulty of the canonical map that owns Player"
    );
    let player_guid = session.player_guid().expect("canonical Player GUID");
    (session, send_rx, player_guid)
}

fn install_player_on_difficulty_map(session: &mut WorldSession, difficulty: u8) {
    let player_guid = session.player_guid().expect("canonical Player GUID");
    let handle = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player owner");
    let position = session.player_position_like_cpp().expect("Player position");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let target = wow_map::MapKey::new(571, 10_000 + u32::from(difficulty));
    let mut manager = manager.lock().expect("canonical MapManager");
    manager
        .detach_player_like_cpp(handle)
        .expect("detach Player from its initial map");
    manager.create_map_entry(
        target.map_id,
        target.instance_id,
        difficulty,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager
        .attach_player_like_cpp(handle, target, position)
        .expect("attach Player to the configured difficulty map");
    let configured_map = manager
        .find_map(target.map_id, target.instance_id)
        .expect("configured difficulty map")
        .map();
    configured_map
        .get_typed_player(player_guid)
        .expect("Player is owned by the configured map");
    assert_eq!(configured_map.spawn_mode(), difficulty);
}

pub(super) async fn cast_sickness(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    guid_counter: i64,
) -> Result<(), &'static str> {
    session
        .execute_spell_with_visual_and_target_data(
            SICKNESS_SPELL,
            player_guid,
            ObjectGuid::create_world_object(
                HighGuid::Cast,
                3,
                1,
                571,
                0,
                SICKNESS_SPELL as u32,
                guid_counter,
            ),
            SpellCastVisual {
                spell_visual_id: 245_039,
                script_visual_id: 0,
            },
            SpellTargetData {
                flags: 0x0000_0002,
                unit: player_guid,
                ..Default::default()
            },
        )
        .await
}

fn sickness_effect_snapshot(session: &WorldSession) -> (u32, Vec<(u32, i32)>) {
    let application = visible_sickness_aura(session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("selected sickness application");
    let effects = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .threat_snapshot_like_cpp(application.slot)
                .expect("selected sickness effect snapshot")
                .effects()
                .iter()
                .map(|(bit, _, amount, _)| (*bit, *amount))
                .collect::<Vec<_>>()
        })
        .expect("canonical sickness Player");
    (application.effect_mask, effects)
}

fn sickness_effect_misc_value(session: &WorldSession, effect_bit: u32) -> i32 {
    let application = visible_sickness_aura(session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("selected sickness application");
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .threat_snapshot_like_cpp(application.slot)
                .expect("selected sickness effect snapshot")
                .effects()
                .iter()
                .find(|(bit, _, _, _)| *bit == effect_bit)
                .map(|(_, _, _, misc_value)| *misc_value)
                .expect("selected sickness effect bit")
        })
        .expect("canonical sickness Player")
}

fn assert_sickness_aura_update(packets: &[Vec<u8>], expected_effect_mask: u32) {
    let updates = packets
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|update| update.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert_eq!(updates.len(), 1, "selected cast publishes one AuraUpdate");
    assert_eq!(updates[0].active_flags, expected_effect_mask);
    assert_eq!(updates[0].aura_flags, 0x0015);
}

pub(super) fn complete_difficulty_chain() -> Vec<wow_data::DifficultyEntry> {
    vec![difficulty(0, 0), difficulty(3, 0), difficulty(4, 3)]
}

#[tokio::test]
async fn selected_sickness_cast_uses_effect_rows_from_the_canonical_map_difficulty() {
    let mut effects = base_effect_rows();
    effects.extend([
        effect_row(
            31,
            3,
            0,
            SPELL_EFFECT_APPLY_AURA,
            SPELL_AURA_MOD_PERCENT_STAT,
            -40,
        ),
        effect_row(
            33,
            3,
            2,
            SPELL_EFFECT_APPLY_AURA,
            SPELL_AURA_NO_PVP_CREDIT,
            -60,
        ),
        effect_row(
            42,
            4,
            1,
            SPELL_EFFECT_APPLY_AURA,
            SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            -50,
        ),
    ]);

    for (case, map_difficulty, expected) in [
        (0_i64, 4_u8, vec![(1_u32, -40), (2, -50), (4, -60)]),
        (1, 3, vec![(1_u32, -40), (2, -20), (4, -60)]),
        (2, 0, vec![(1_u32, -10), (2, -20), (4, -30)]),
    ] {
        let guid_counter = 44_100 + case * 10;
        let (mut session, send_rx, player_guid) = selected_session(
            guid_counter,
            map_difficulty,
            effects.clone(),
            complete_difficulty_chain(),
        )
        .await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        cast_sickness(&mut session, player_guid, guid_counter + 1)
            .await
            .expect("selected sickness cast executes");

        assert_eq!(
            visible_sickness_aura(&session, SICKNESS_SPELL)[0].difficulty_id,
            map_difficulty,
            "the created AuraApplication records the raw map difficulty"
        );
        let snapshot = sickness_effect_snapshot(&session);
        assert_eq!(snapshot.0, 0b111);
        assert_eq!(snapshot.1, expected);
        let mut packets = drain_packets(&send_rx);
        assert!(
            packets
                .iter()
                .any(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16),
            "resolved selected cast publishes SpellGo"
        );

        session.update(0).await;
        packets.extend(drain_packets(&send_rx));
        assert_sickness_aura_update(&packets, 0b111);
    }
}

#[tokio::test]
async fn selected_sickness_refresh_uses_original_aura_difficulty_properties() {
    let mut effects = base_effect_rows();
    let mut refreshed_stat = effect_row(
        31,
        3,
        0,
        SPELL_EFFECT_APPLY_AURA,
        SPELL_AURA_MOD_PERCENT_STAT,
        -40,
    );
    refreshed_stat.effect_die_sides = 1;
    refreshed_stat.effect_misc_value[0] = 2;
    effects.extend([
        refreshed_stat,
        effect_row(
            33,
            3,
            2,
            SPELL_EFFECT_APPLY_AURA,
            SPELL_AURA_NO_PVP_CREDIT,
            -60,
        ),
    ]);

    let (mut session, send_rx, player_guid) =
        selected_session(44_171, 0, effects, complete_difficulty_chain()).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_sickness(&mut session, player_guid, 44_172)
        .await
        .expect("initial selected sickness cast executes");
    assert_eq!(sickness_effect_misc_value(&session, 0b001), -1);
    drain_packets(&send_rx);
    session.update(0).await;
    drain_packets(&send_rx);

    install_player_on_difficulty_map(&mut session, 3);
    cast_sickness(&mut session, player_guid, 44_173)
        .await
        .expect("selected sickness refresh executes");

    let application = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("refreshed selected sickness application");
    assert_eq!(
        application.difficulty_id, 0,
        "refresh keeps the original Aura difficulty"
    );
    assert_eq!(
        sickness_effect_snapshot(&session).1,
        vec![(1, -40), (2, -20), (4, -60)],
        "refresh applies the new base points to the original effect properties"
    );
    assert_eq!(
        sickness_effect_misc_value(&session, 0b001),
        -1,
        "refresh retains the original SpellInfo misc value"
    );

    let mut packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16),
        "selected refresh publishes SpellGo"
    );
    session.update(0).await;
    packets.extend(drain_packets(&send_rx));
    assert_sickness_aura_update(&packets, 0b111);
}

#[tokio::test]
async fn selected_sickness_cast_falls_back_from_difficulty_zero_to_three() {
    let effects = [
        (0, SPELL_AURA_MOD_PERCENT_STAT, -40),
        (1, SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, -50),
        (2, SPELL_AURA_NO_PVP_CREDIT, -60),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (index, aura, points))| {
        effect_row(
            id as u32 + 31,
            3,
            index,
            SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
        )
    })
    .collect();
    let (mut session, send_rx, player_guid) =
        selected_session(44_151, 0, effects, vec![difficulty(0, 3), difficulty(3, 0)]).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_sickness(&mut session, player_guid, 44_152)
        .await
        .expect("difficulty fallback sickness cast executes");

    assert_eq!(
        visible_sickness_aura(&session, SICKNESS_SPELL)[0].difficulty_id,
        0,
        "fallback effects retain the raw map difficulty on the application"
    );
    let snapshot = sickness_effect_snapshot(&session);
    assert_eq!(snapshot.0, 0b111);
    assert_eq!(snapshot.1, vec![(1, -40), (2, -50), (4, -60)]);
    let mut packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16),
        "the difficulty fallback publishes SpellGo"
    );
    session.update(0).await;
    packets.extend(drain_packets(&send_rx));
    assert_sickness_aura_update(&packets, 0b111);
}

#[tokio::test]
async fn selected_sickness_explicit_inert_row_does_not_inherit_the_active_base_effect() {
    let mut effects = base_effect_rows();
    effects.push(effect_row(31, 3, 0, 0, SPELL_AURA_MOD_PERCENT_STAT, 999));
    let (mut session, send_rx, player_guid) =
        selected_session(44_201, 4, effects, complete_difficulty_chain()).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    assert_eq!(
        session
            .catalogs
            .spell_store()
            .and_then(|store| store.get(SICKNESS_SPELL))
            .expect("base sickness SpellInfo")
            .effect_type,
        SPELL_EFFECT_APPLY_AURA,
        "the fallback base has an active primary effect"
    );

    assert_eq!(
        cast_sickness(&mut session, player_guid, 44_202).await,
        Err("Race-selected sickness effect catalog differs from the supported Aura"),
        "an explicit inert row is rejected instead of inheriting the active base effect"
    );
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    let mut packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16),
        "the selected row is sent in SpellGo before the sickness shape is rejected"
    );
    session.update(0).await;
    packets.extend(drain_packets(&send_rx));
    assert!(
        packets
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .flat_map(|bytes| decode_sickness_auras(bytes))
            .all(|update| update.spell_id != SICKNESS_SPELL),
        "the rejected explicit inert row produces no sickness AuraUpdate"
    );
}

#[tokio::test]
async fn selected_sickness_effect_resolution_failures_precede_spellgo_and_aura_mutation() {
    let base = base_effect_rows();
    let cycle = vec![difficulty(0, 0), difficulty(3, 4), difficulty(4, 3)];
    let mut invalid = base.clone();
    invalid.push(effect_row(
        44,
        4,
        32,
        SPELL_EFFECT_APPLY_AURA,
        SPELL_AURA_NO_PVP_CREDIT,
        -1,
    ));

    for (case, rows, difficulties, missing_difficulty_catalog) in [
        (0_i64, base, complete_difficulty_chain(), true),
        (1, base_effect_rows(), cycle, false),
        (2, invalid, complete_difficulty_chain(), false),
    ] {
        let guid_counter = 44_300 + case * 10;
        let (mut session, send_rx, player_guid) =
            selected_session(guid_counter, 4, rows, difficulties).await;
        session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
            spell_x_spell_visual_store: Some(Arc::new(
                wow_data::SpellXSpellVisualStore::from_entries([wow_data::SpellXSpellVisualEntry {
                    id: 44_399,
                    difficulty_id: 4,
                    spell_visual_id: 245_039,
                    probability: 1.0,
                    flags: 0,
                    priority: 0,
                    spell_icon_file_id: 0,
                    active_icon_file_id: 0,
                    viewer_unit_condition_id: 0,
                    viewer_player_condition_id: 0,
                    caster_unit_condition_id: 0,
                    caster_player_condition_id: 0,
                    spell_id: SICKNESS_SPELL as u32,
                }]),
            )),
            ..Default::default()
        });
        if missing_difficulty_catalog {
            session.catalogs.difficulty_store = None;
        }

        let result = session.execute_spell(SICKNESS_SPELL, player_guid).await;
        assert_eq!(result, Err("Unable to resolve selected spell effects"));
        assert!(
            drain_packets(&send_rx).is_empty(),
            "malformed or unknown selected effects fail before SpellGo or AuraUpdate"
        );
        assert!(
            visible_sickness_aura(&session, SICKNESS_SPELL).is_empty(),
            "failed selected resolution leaves canonical aura state unchanged"
        );
    }
}
