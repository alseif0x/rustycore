use super::*;
use std::sync::Arc;

const WISP_SPELL: i32 = wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP;
const WISP_VISUAL_ROW_ID: u32 = 0xF765_4321;

async fn cast_wisp(session: &mut WorldSession) -> Result<(), &'static str> {
    let player_guid = session.player_guid().expect("canonical Player GUID");
    let generators = session.id_generators_for_test_like_cpp();
    let spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        session,
        generators.item.as_ref(),
        &spawns,
        WISP_SPELL,
        player_guid,
        wow_entities::SpellCastMetadata {
            trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
            ..Default::default()
        },
    )
    .await
}

fn install_wisp_visual_rows(
    session: &mut WorldSession,
    rows: impl IntoIterator<Item = wow_data::SpellXSpellVisualEntry>,
) {
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(
            wow_data::SpellXSpellVisualStore::from_entries(rows),
        )),
        ..Default::default()
    });
}

fn wisp_provenance(session: &WorldSession) -> wow_entities::AuraCastProvenanceLikeCpp {
    let application = visible_sickness_aura(session, WISP_SPELL)
        .into_iter()
        .next()
        .expect("selected Wisp Aura");
    session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(application.slot)
        })
        .expect("canonical Wisp Aura provenance")
}

#[tokio::test]
async fn wisp_cast_visual_uses_known_empty_and_source_relation_rows() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_911).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    install_wisp_visual_rows(&mut session, []);
    cast_wisp(&mut session)
        .await
        .expect("known-empty Wisp visual store creates the selected Aura");
    let empty_packets = drain_packets(&send_rx);
    assert!(
        empty_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::SpellGo as u16),
        "zero visual and no other client-log condition suppresses SpellGo"
    );
    assert_eq!(
        visible_sickness_aura(&session, WISP_SPELL).len(),
        1,
        "the known-empty cast applies Wisp before provenance is inspected"
    );
    let original = wisp_provenance(&session);
    assert_eq!(original.spell_visual_id, 0);

    install_wisp_visual_rows(
        &mut session,
        [wow_data::SpellXSpellVisualEntry {
            id: WISP_VISUAL_ROW_ID,
            difficulty_id: 0,
            spell_visual_id: 98_765,
            probability: 1.0,
            flags: 0,
            priority: 0,
            spell_icon_file_id: 0,
            active_icon_file_id: 0,
            viewer_unit_condition_id: 0,
            viewer_player_condition_id: 0,
            caster_unit_condition_id: 0,
            caster_player_condition_id: 0,
            spell_id: WISP_SPELL as u32,
        }],
    );
    cast_wisp(&mut session)
        .await
        .expect("Wisp cast resolves its selected relation row");
    let source_packets = drain_packets(&send_rx);
    let source_go = source_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("nonzero Wisp relation ID publishes SpellGo");
    let (cast_id, _, spell_id, visual_id, _) = decode_spell_go(source_go);
    assert_eq!(spell_id, WISP_SPELL);
    assert_eq!(visual_id, WISP_VISUAL_ROW_ID);
    assert_ne!(visual_id, 98_765);
    let refreshed = wisp_provenance(&session);
    assert_eq!(refreshed.cast_id, original.cast_id);
    assert_eq!(refreshed.spell_visual_id, original.spell_visual_id);
    assert_ne!(cast_id, original.cast_id);
}
