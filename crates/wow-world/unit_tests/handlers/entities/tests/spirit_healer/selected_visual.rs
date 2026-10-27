use super::*;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};

const SOURCE_VISUAL_ROW_ID: u32 = 0xF234_5678;
const REFRESH_VISUAL_ROW_ID: u32 = 0xA765_4321;

fn visual_row(
    id: u32,
    difficulty_id: u8,
    spell_visual_id: u32,
    priority: i32,
    spell_id: u32,
) -> wow_data::SpellXSpellVisualEntry {
    wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id,
        spell_visual_id,
        probability: 1.0,
        flags: 0,
        priority,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id,
    }
}

fn install_source_visual(session: &mut WorldSession, visual_row_id: u32) {
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [visual_row(
                visual_row_id,
                0,
                76_543,
                0,
                SICKNESS_SPELL as u32,
            )],
        ))),
        ..Default::default()
    });
}

fn mark_sickness_no_threat(session: &mut WorldSession) {
    let spells = Arc::get_mut(
        session
            .catalogs
            .spell_catalogs
            .spell_store
            .as_mut()
            .expect("selected sickness SpellStore"),
    )
    .expect("fixture owns the selected sickness SpellStore");
    let mut attributes = spells
        .misc_attributes_for_difficulty_like_cpp(SICKNESS_SPELL, 0, None)
        .expect("selected sickness SpellMisc attributes");
    attributes[1] |= wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT;
    spells.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL, attributes);
}

async fn cast_sickness_with_visual(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    cast_id_counter: i64,
    packet_visual_id: u32,
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
                cast_id_counter,
            ),
            SpellCastVisual {
                spell_visual_id: packet_visual_id,
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

fn sickness_provenance(session: &WorldSession) -> wow_entities::AuraCastProvenanceLikeCpp {
    let application = visible_sickness_aura(session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("fresh selected sickness Aura");
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(application.slot)
        })
        .expect("canonical sickness Aura provenance")
}

async fn cast_server_triggered_sickness(
    session: &mut WorldSession,
    metadata: wow_entities::SpellCastMetadata,
) -> Result<(), &'static str> {
    let generators = session.id_generators_for_test_like_cpp();
    let spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let player_guid = session.player_guid().expect("canonical Player GUID");
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        session,
        generators.item.as_ref(),
        &spawns,
        SICKNESS_SPELL,
        player_guid,
        metadata,
    )
    .await
}

fn install_sickness_mana_cost(session: &mut WorldSession) {
    let spells = Arc::get_mut(
        session
            .catalogs
            .spell_catalogs
            .spell_store
            .as_mut()
            .expect("selected sickness SpellStore"),
    )
    .expect("fixture owns the selected sickness SpellStore");
    let mut sickness = spells
        .get(SICKNESS_SPELL)
        .expect("selected sickness SpellInfo")
        .clone();
    sickness.power_costs.push(wow_data::SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: wow_constants::PowerType::Mana as i8,
        mana_cost: 25,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });
    spells.insert(SICKNESS_SPELL, sickness);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_power_index(wow_constants::PowerType::Mana, Some(0));
            unit.set_max_power(wow_constants::PowerType::Mana, 100);
            unit.set_power(wow_constants::PowerType::Mana, 100);
        })
        .expect("canonical Player mana");
}

fn sickness_mana(session: &WorldSession) -> i32 {
    session
        .core
        .with_owned_player_like_cpp(|player| player.unit().get_power(wow_constants::PowerType::Mana))
        .expect("canonical Player mana")
}

fn install_visual_override_potential(session: &mut WorldSession) {
    let spells = Arc::get_mut(
        session
            .catalogs
            .spell_catalogs
            .spell_store
            .as_mut()
            .expect("selected sickness SpellStore"),
    )
    .expect("fixture owns the selected sickness SpellStore");
    let mut potential = spells
        .get(SICKNESS_SPELL)
        .expect("selected sickness SpellInfo")
        .clone();
    potential.spell_id = 846_202;
    potential.effect_type = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
    potential.aura_type = Some(403);
    potential.effects = vec![wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: 403,
        effect_misc_value_1: SICKNESS_SPELL,
        ..Default::default()
    }];
    spells.insert(
        846_202,
        potential,
    );
}

#[tokio::test]
async fn selected_cast_go_uses_source_visual_and_refresh_keeps_aura_provenance() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_871).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::prepare_recovered_player_for_selected_cast_like_cpp(&mut session);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    mark_sickness_no_threat(&mut session);
    super::selected_cast_threat::install_known_positive_class(&mut session);
    install_source_visual(&mut session, SOURCE_VISUAL_ROW_ID);
    let player_guid = session.player_guid().expect("canonical Player GUID");

    cast_sickness_with_visual(&mut session, player_guid, 43_872, 12_345)
        .await
        .expect("selected sickness cast creates its Aura");
    let first_packets = drain_packets(&send_rx);
    let first_go = first_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("selected sickness cast publishes SpellGo");
    let first_visual_id = decode_spell_go(first_go).3;
    assert_eq!(
        first_visual_id,
        SOURCE_VISUAL_ROW_ID,
        "SpellGo uses the selected relation ID rather than the supplied visual"
    );
    assert_ne!(first_visual_id, 12_345);
    assert_ne!(first_visual_id, 76_543);
    let original = sickness_provenance(&session);
    assert_eq!(original.spell_visual_id, SOURCE_VISUAL_ROW_ID as i32);
    session.update(0).await;
    let aura_updates = drain_packets(&send_rx)
        .into_iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(&bytes))
        .filter(|update| update.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert!(!aura_updates.is_empty(), "the selected Aura is published");
    assert!(
        aura_updates
            .iter()
            .all(|update| update.visual_id == SOURCE_VISUAL_ROW_ID as i32)
    );

    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [visual_row(
                REFRESH_VISUAL_ROW_ID,
                0,
                23_456,
                0,
                SICKNESS_SPELL as u32,
            )],
        ))),
        ..Default::default()
    });
    super::selected_cast_threat::drop_class_options_for_refresh(&mut session);
    cast_sickness_with_visual(&mut session, player_guid, 43_873, 54_321)
        .await
        .expect("known cast visual permits refresh when Aura-only facts are missing");
    let refresh_packets = drain_packets(&send_rx);
    let refresh_go = refresh_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("matching refresh publishes its newly selected SpellGo visual");
    let (refresh_cast_id, _, _, refresh_visual_id, _) = decode_spell_go(refresh_go);
    assert_ne!(refresh_cast_id, original.cast_id);
    assert_eq!(refresh_visual_id, REFRESH_VISUAL_ROW_ID);
    let refreshed = sickness_provenance(&session);
    assert_eq!(refreshed.cast_id, original.cast_id);
    assert_eq!(refreshed.spell_visual_id, original.spell_visual_id);
    assert_eq!(refreshed.caster_level, original.caster_level);
}

#[tokio::test]
async fn selected_cast_visual_uses_difficulty_fallback_and_ordered_relation_rows() {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, SPELL_AURA_MOD_PERCENT_STAT,
        SPELL_AURA_NO_PVP_CREDIT,
    };
    use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

    let effect_rows = [
        (0, SPELL_AURA_MOD_PERCENT_STAT, -10),
        (1, SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, -20),
        (2, SPELL_AURA_NO_PVP_CREDIT, -30),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, (index, aura, points))| {
        super::selected_execution_effects::effect_row(
            70 + id as u32,
            0,
            index,
            SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
        )
    })
    .collect();
    let difficulties = vec![
        super::selected_execution_effects::difficulty(0, 0),
        super::selected_execution_effects::difficulty(2, 0),
        super::selected_execution_effects::difficulty(3, 2),
    ];
    let mut attributes = [0; 15];
    attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT;
    let (mut session, send_rx, player_guid) =
        super::selected_execution_effects::selected_session_with_spell_attributes(
            43_881,
            3,
            effect_rows,
            difficulties,
            attributes,
        )
        .await;
    super::prepare_recovered_player_for_selected_cast_like_cpp(&mut session);
    super::selected_cast_threat::install_known_positive_class(&mut session);
    let selected_relation_id = 0xF876_5432;
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(
            wow_data::SpellXSpellVisualStore::from_entries([
                visual_row(0xFFFF_FFFE, 0, 88_001, 0, SICKNESS_SPELL as u32),
                visual_row(0xF123_4567, 2, 88_002, 1, SICKNESS_SPELL as u32),
                visual_row(
                    selected_relation_id,
                    2,
                    88_003,
                    0,
                    SICKNESS_SPELL as u32,
                ),
            ]),
        )),
        ..Default::default()
    });

    super::selected_execution_effects::cast_sickness(&mut session, player_guid, 43_882)
        .await
        .expect("selected sickness cast resolves fallback visual rows");
    let packets = drain_packets(&send_rx);
    let go = packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("nonzero selected relation ID publishes SpellGo");
    assert_eq!(decode_spell_go(go).3, selected_relation_id);
    assert_ne!(decode_spell_go(go).3, 88_003);
}

#[tokio::test]
async fn unavailable_visual_rejects_before_power_and_server_cast_id_is_allocated_once() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_891).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::prepare_recovered_player_for_selected_cast_like_cpp(&mut session);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    install_sickness_mana_cost(&mut session);
    session.set_creature_aggro_config_like_cpp(Default::default());
    let player_guid = session.player_guid().expect("canonical Player GUID");

    assert_eq!(sickness_mana(&session), 100);
    assert_eq!(
        session
            .execute_spell_with_visual_and_target_data_with_metadata(
                SICKNESS_SPELL,
                player_guid,
                ObjectGuid::create_world_object(
                    HighGuid::Cast,
                    3,
                    1,
                    571,
                    0,
                    SICKNESS_SPELL as u32,
                    43_892,
                ),
                SpellCastVisual {
                    spell_visual_id: 44_444,
                    script_visual_id: 0,
                },
                SpellTargetData {
                    flags: 0x0000_0002,
                    unit: player_guid,
                    ..Default::default()
                },
                wow_entities::SpellCastMetadata {
                    from_client: true,
                    ..Default::default()
                },
            )
            .await,
        Err("Missing SpellXSpellVisual store for selected aura creation")
    );
    assert_eq!(sickness_mana(&session), 100);

    let before = wow_world_spell::next_represented_spell_cast_guid_like_cpp(
        &session,
        SICKNESS_SPELL,
    )
    .expect("first Map CastID allocation");
    assert_eq!(
        cast_server_triggered_sickness(
            &mut session,
            wow_entities::SpellCastMetadata {
                trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
                ..Default::default()
            },
        )
        .await,
        Err("Missing SpellXSpellVisual store for selected aura creation")
    );
    let after = wow_world_spell::next_represented_spell_cast_guid_like_cpp(
        &session,
        SICKNESS_SPELL,
    )
    .expect("next Map CastID allocation");
    assert_eq!(after.counter() - before.counter(), 2);
    assert_eq!(sickness_mana(&session), 100);
    assert!(drain_packets(&send_rx).is_empty());
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
}

#[tokio::test]
async fn visual_override_potential_rejects_before_power_and_publication() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_901).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::prepare_recovered_player_for_selected_cast_like_cpp(&mut session);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    install_source_visual(&mut session, SOURCE_VISUAL_ROW_ID);
    install_sickness_mana_cost(&mut session);
    install_visual_override_potential(&mut session);
    let player_guid = session.player_guid().expect("canonical Player GUID");

    assert_eq!(
        session
            .execute_spell_with_visual_and_target_data_with_metadata(
                SICKNESS_SPELL,
                player_guid,
                ObjectGuid::create_world_object(
                    HighGuid::Cast,
                    3,
                    1,
                    571,
                    0,
                    SICKNESS_SPELL as u32,
                    43_902,
                ),
                SpellCastVisual {
                    spell_visual_id: 44_444,
                    script_visual_id: 0,
                },
                SpellTargetData {
                    flags: 0x0000_0002,
                    unit: player_guid,
                    ..Default::default()
                },
                wow_entities::SpellCastMetadata {
                    from_client: true,
                    ..Default::default()
                },
            )
            .await,
        Err("Selected aura visual override source/order is unavailable")
    );
    assert_eq!(sickness_mana(&session), 100);
    assert!(drain_packets(&send_rx).is_empty());
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
}
