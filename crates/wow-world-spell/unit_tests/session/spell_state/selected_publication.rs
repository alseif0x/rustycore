//! Per-Player selected publication prepares immutable frames before clearing.

use super::*;
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_data::character_progression::ChrRacesEntry;
use wow_entities::Player;
use wow_packet::WorldPacket;

const MAP: MapKey = MapKey::new(571, 0);
const SICKNESS: i32 = 26_007;

fn races(sickness: i32) -> ChrRacesStore {
    ChrRacesStore::from_entries([ChrRacesEntry {
        id: 4,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: "Selected publication race".into(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: sickness,
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
    }])
}

fn add_player(manager: &mut MapManager, guid: ObjectGuid) -> PlayerHandle {
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_race(4);
    player.unit_mut().set_level(80);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, MAP, Position::ZERO)
        .unwrap();
    handle
}

fn fixture() -> (MapManager, PlayerHandle) {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(MAP.map_id, MAP.instance_id);
    let handle = add_player(&mut manager, ObjectGuid::create_player(1, 84_200_001));
    (manager, handle)
}

fn install(manager: &mut MapManager, handle: PlayerHandle, slot: u8, spell_id: i32) -> ObjectGuid {
    let cast_id = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Cast,
        3,
        1,
        571,
        0,
        spell_id as u32,
        100 + i64::from(slot),
    );
    manager
        .with_player_mut_like_cpp(handle, |player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.insert_runtime_application_like_cpp(AuraApplication {
                spell_id,
                difficulty_id: 0,
                caster_guid: handle.guid(),
                slot,
                duration_total: 600_000,
                duration_remaining: 12_345,
                stack_count: 1,
                aura_flags: 1,
                effect_mask: 7,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            });
            auras.set_aura_cast_provenance_like_cpp(
                slot,
                AuraCastProvenanceLikeCpp {
                    cast_id,
                    spell_visual_id: 42,
                },
            );
            auras.visible_auras_to_update.insert(slot);
        })
        .unwrap();
    cast_id
}

fn pending(manager: &MapManager, handle: PlayerHandle) -> Vec<u8> {
    let mut slots = manager
        .with_player_like_cpp(handle, |player| {
            player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .iter()
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap();
    slots.sort_unstable();
    slots
}

#[test]
fn current_race_sickness_and_ghost_wisp_are_prepared_in_slot_order_once() {
    let (mut manager, handle) = fixture();
    let ghost = install(&mut manager, handle, 7, 8326);
    let sickness = install(&mut manager, handle, 2, SICKNESS);
    let wisp = install(&mut manager, handle, 5, 20584);
    install(&mut manager, handle, 1, 15007);
    install(&mut manager, handle, 9, 118);

    let updates =
        collect_player_pending_selected_aura_updates(&mut manager, handle, Some(&races(SICKNESS)))
            .unwrap();
    assert_eq!(updates.len(), 3);
    for (update, (slot, spell, cast)) in
        updates
            .iter()
            .zip([(2, SICKNESS, sickness), (5, 20584, wisp), (7, 8326, ghost)])
    {
        let mut packet = WorldPacket::from_bytes(update.packet_bytes());
        assert_eq!(
            packet.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::AuraUpdate as u16
        );
        assert!(!packet.read_bit().unwrap());
        assert_eq!(packet.read_bits(9).unwrap(), 1);
        assert_eq!(packet.read_uint8().unwrap(), slot);
        assert!(packet.read_bit().unwrap());
        assert_eq!(packet.read_packed_guid().unwrap(), cast);
        assert_eq!(packet.read_int32().unwrap(), spell);
        assert_eq!(packet.read_int32().unwrap(), 42);
    }
    assert_eq!(pending(&manager, handle), [1, 9]);
    assert!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, Some(&races(SICKNESS)))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn empty_pending_needs_no_race_lookup_but_classification_requires_known_metadata() {
    let (mut manager, handle) = fixture();
    let missing = ChrRacesStore::from_entries([]);
    assert!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, None)
            .unwrap()
            .is_empty()
    );
    assert!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, Some(&missing))
            .unwrap()
            .is_empty()
    );
    install(&mut manager, handle, 0, 8326);
    let invalid = races(-1);
    for catalog in [None, Some(&missing), Some(&invalid)] {
        assert!(
            collect_player_pending_selected_aura_updates(&mut manager, handle, catalog).is_err()
        );
        assert_eq!(pending(&manager, handle), [0]);
    }
    assert_eq!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, Some(&races(0)))
            .unwrap()
            .len(),
        1
    );
    install(&mut manager, handle, 4, 15007);
    assert!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, Some(&races(0)))
            .unwrap()
            .is_empty()
    );
    assert_eq!(pending(&manager, handle), [4]);
}

#[test]
fn incomplete_pending_snapshot_never_consumes_the_other_selected_slot() {
    for malformed_slot in [false, true] {
        let (mut manager, handle) = fixture();
        install(&mut manager, handle, 0, 8326);
        if malformed_slot {
            install(&mut manager, handle, 3, 20584);
        }
        manager
            .with_player_mut_like_cpp(handle, |player| {
                let auras = &mut player.unit_mut().subsystems_mut().auras;
                auras.visible_auras_to_update.insert(3);
                if malformed_slot {
                    auras.runtime_application_mut_like_cpp(3).unwrap().slot = 4;
                }
            })
            .unwrap();
        assert!(
            collect_player_pending_selected_aura_updates(
                &mut manager,
                handle,
                Some(&races(SICKNESS))
            )
            .is_err()
        );
        assert_eq!(pending(&manager, handle), [0, 3]);
    }
}

#[test]
fn stale_detached_or_invalid_canonical_sources_preserve_pending() {
    for mode in 0..6 {
        let (mut manager, handle) = fixture();
        install(&mut manager, handle, 0, 8326);
        let current = match mode {
            0 => {
                let replacement = add_player(&mut manager, handle.guid());
                install(&mut manager, replacement, 0, 8326);
                replacement
            }
            1 => {
                manager.detach_player_like_cpp(handle).unwrap();
                handle
            }
            _ => {
                manager
                    .with_player_mut_like_cpp(handle, |player| match mode {
                        2 => player
                            .unit_mut()
                            .world_mut()
                            .object_mut()
                            .remove_from_world(),
                        3 | 4 => {
                            let world = player.unit_mut().world_mut();
                            world.object_mut().remove_from_world();
                            world.reset_map().unwrap();
                            if mode == 3 {
                                world.set_map(1, 0).unwrap();
                            }
                            world.object_mut().add_to_world();
                        }
                        _ => player
                            .unit_mut()
                            .world_mut()
                            .object_mut()
                            .create(ObjectGuid::create_player(1, 84_200_002)),
                    })
                    .unwrap();
                handle
            }
        };
        assert!(
            collect_player_pending_selected_aura_updates(
                &mut manager,
                handle,
                Some(&races(SICKNESS))
            )
            .is_err()
        );
        assert_eq!(pending(&manager, current), [0]);
    }
}

#[test]
fn existing_global_collection_remains_ghost_wisp_only() {
    let (mut manager, handle) = fixture();
    install(&mut manager, handle, 0, SICKNESS);
    install(&mut manager, handle, 1, 8326);
    let plan = manager.begin_tick_like_cpp(100).into_started().unwrap();
    assert_eq!(
        collect_pending_selected_aura_updates(&mut manager, &plan).len(),
        1
    );
    assert_eq!(pending(&manager, handle), [0]);
}
