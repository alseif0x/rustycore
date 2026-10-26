use super::*;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};

const SOURCE_VISUAL_ROW_ID: u32 = 0xF234_5678;

fn install_source_visual(session: &mut WorldSession, visual_row_id: u32) {
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: visual_row_id,
                difficulty_id: 0,
                spell_visual_id: 76_543,
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
            }],
        ))),
        ..Default::default()
    });
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

#[tokio::test]
async fn selected_creation_uses_source_visual_bits_and_refresh_keeps_them_without_fresh_source() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_871).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    install_source_visual(&mut session, SOURCE_VISUAL_ROW_ID);
    let player_guid = session.player_guid().expect("canonical Player GUID");

    cast_sickness_with_visual(&mut session, player_guid, 43_872, 12_345)
        .await
        .expect("selected sickness cast creates its Aura");
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

    session.set_creature_aggro_config_like_cpp(Default::default());
    cast_sickness_with_visual(&mut session, player_guid, 43_873, 54_321)
        .await
        .expect("same-key full-mask refresh does not need fresh visual facts");
    let refreshed = sickness_provenance(&session);
    assert_eq!(refreshed.cast_id, original.cast_id);
    assert_eq!(refreshed.spell_visual_id, original.spell_visual_id);
    assert_eq!(refreshed.caster_level, original.caster_level);
}
