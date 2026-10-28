use super::*;
use std::sync::Arc;

const GHOST: i32 = wow_world_entities::SPELL_GHOST_LIKE_CPP;
const WISP: i32 = wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP;

fn selected_update_slots(packets: &[Vec<u8>]) -> Vec<(u8, i32)> {
    packets
        .iter()
        .filter_map(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok()? != ServerOpcodes::AuraUpdate as u16 {
                return None;
            }
            let _update_all = packet.read_bit().ok()?;
            let count = packet.read_bits(9).ok()?;
            assert_eq!(count, 1, "selected release updates are individual slots");
            let slot = packet.read_uint8().ok()?;
            if !packet.read_bit().ok()? {
                return None;
            }
            packet.reset_bits();
            packet.read_packed_guid().ok()?;
            let spell_id = packet.read_int32().ok()?;
            Some((slot, spell_id))
        })
        .collect()
}

#[tokio::test]
async fn automatic_release_keeps_ghost_and_wisp_pending_until_the_next_recovery() {
    let (mut session, send_rx, realm_rx, player_guid, _port) =
        dead_player_session_with_graveyard_like_cpp(9_208);
    let _input_peer = keep_driver_input_open(&mut session);
    session.core.state = crate::session::SessionState::LoggedIn;
    let race_id = session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let mut spells = wow_entities::PlayerSpellRuntimeState::default();
            spells.add_known_spell_like_cpp(wow_world_entities::SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP);
            player.replace_spell_runtime_like_cpp(spells);
            player.race_like_cpp()
        })
        .expect("canonical Player owner");
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: u32::from(race_id),
                res_sickness_spell_id: 0,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "Corpse selected recovery test race".into(),
                flags: 0,
                male_display_id: 0,
                female_display_id: 0,
                high_res_male_display_id: 0,
                high_res_female_display_id: 0,
                splash_sound_id: 0,
                create_screen_file_data_id: 0,
                select_screen_file_data_id: 0,
                low_res_screen_file_data_id: 0,
                altered_form_start_visual_kit_id: [0; 3],
                altered_form_finish_visual_kit_id: [0; 3],
                heritage_armor_achievement_id: 0,
                starting_level: 1,
                ui_display_order: 0,
                playable_race_bit: 0,
                female_skeleton_file_data_id: 0,
                male_skeleton_file_data_id: 0,
                helmet_anim_scaling_race_id: 0,
                transmogrify_disabled_slot_mask: 0,
                faction_id: 0,
                cinematic_sequence_id: 0,
                base_language: 0,
                creature_type: 0,
                alliance: 0,
                race_related: 0,
                unaltered_visual_race_id: 0,
                default_class_id: 0,
                neutral_race_id: 0,
            },
        ]),
    ));
    let (graveyards, conditions) =
        graveyard_store_with_links(0, [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP], []);
    session.set_graveyard_store(graveyards);
    session.set_condition_store(conditions);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
        })
        .expect("canonical Player owner");

    session.update(0).await;
    assert_eq!(death_timer_from_driver(&session), 360_000);
    let _ = send_rx.try_iter().collect::<Vec<_>>();
    let _ = realm_rx.try_iter().collect::<Vec<_>>();
    set_death_timer_for_driver(&mut session, 1);

    session.update(1).await;

    let slots = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let mut slots = auras
                .runtime_applications_like_cpp()
                .iter()
                .filter_map(|(&slot, aura)| {
                    matches!(aura.spell_id, GHOST | WISP).then_some((slot, aura.spell_id))
                })
                .collect::<Vec<_>>();
            slots.sort_unstable_by_key(|(slot, _)| *slot);
            for (slot, _) in &slots {
                assert!(auras.visible_auras_to_update.contains(slot));
            }
            slots
        })
        .expect("canonical Player owner");
    assert_eq!(
        slots.iter().map(|(_, spell)| *spell).collect::<Vec<_>>(),
        [WISP, GHOST],
        "BuildPlayerRepop applies Wisp before Ghost after the selected Unit flush"
    );
    assert!(
        selected_update_slots(&send_rx.try_iter().collect::<Vec<_>>()).is_empty(),
        "release leaves selected applications pending for the next Recovery"
    );

    session.update(1).await;

    assert_eq!(
        selected_update_slots(&send_rx.try_iter().collect::<Vec<_>>()),
        slots,
        "the next selected Recovery publishes pending Wisp and Ghost in slot order"
    );
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            for (slot, _) in slots {
                assert!(!auras.visible_auras_to_update.contains(&slot));
            }
        })
        .expect("canonical Player owner");
}
