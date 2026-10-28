//! Per-Player selected publication prepares immutable frames before clearing.

use super::*;
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_data::character_progression::{ChrRacesEntry, ChrRacesStore};
use wow_data::{DifficultyStore, SpellAuraOptionsStore, SpellStore};
use wow_entities::{
    AuraCastProvenanceLikeCpp, AuraPersistenceMetadata, AuraThreatSnapshotLikeCpp, Player,
};
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

fn spell_catalog() -> SpellStore {
    let mut spells = SpellStore::new();
    for spell_id in [8326, 20_584, SICKNESS] {
        spells.insert_spell_misc_attributes_like_cpp(spell_id, [0; 15]);
    }
    spells
}

fn catalogs<'a>(
    races: Option<&'a ChrRacesStore>,
    spells: Option<&'a SpellStore>,
    aura_options: Option<&'a SpellAuraOptionsStore>,
    difficulties: Option<&'a DifficultyStore>,
) -> SelectedAuraPublicationCatalogs<'a> {
    SelectedAuraPublicationCatalogs {
        races,
        spells,
        aura_options,
        difficulties,
    }
}

fn selected_aura_types(spell_id: i32) -> Option<[i32; 3]> {
    use wow_data::spell::aura_types as aura;
    match spell_id {
        8326 => Some([
            aura::SPELL_AURA_GHOST,
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]),
        20_584 => Some([
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            aura::SPELL_AURA_TRANSFORM,
        ]),
        SICKNESS => Some([
            aura::SPELL_AURA_MOD_PERCENT_STAT,
            aura::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            aura::SPELL_AURA_NO_PVP_CREDIT,
        ]),
        _ => None,
    }
}

fn install(manager: &mut MapManager, handle: PlayerHandle, slot: u8, spell_id: i32) -> ObjectGuid {
    install_with_mask(manager, handle, slot, spell_id, 0b111)
}

fn install_with_mask(
    manager: &mut MapManager,
    handle: PlayerHandle,
    slot: u8,
    spell_id: i32,
    effect_mask: u32,
) -> ObjectGuid {
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
            let application = AuraApplication {
                spell_id,
                difficulty_id: 0,
                caster_guid: handle.guid(),
                slot,
                duration_total: 600_000,
                duration_remaining: 12_345,
                stack_count: 1,
                aura_flags: 1,
                effect_mask,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            };
            if let Some(effect_types) = selected_aura_types(spell_id) {
                let snapshot = AuraThreatSnapshotLikeCpp::new(
                    [0; 2],
                    effect_types
                        .into_iter()
                        .enumerate()
                        .filter(|(index, _)| effect_mask & (1_u32 << *index) != 0)
                        .map(|(index, aura_type)| {
                            (1_u32 << index, aura_type, 101 + index as i32, 0)
                        })
                        .collect(),
                );
                player.apply_player_threat_aura_like_cpp(
                    spell_id as u32,
                    handle.guid(),
                    slot,
                    snapshot.clone(),
                );
                player.install_player_threat_aura_like_cpp(slot, snapshot, application);
            } else {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .insert_runtime_application_like_cpp(application);
            }
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.set_aura_cast_provenance_like_cpp(
                slot,
                AuraCastProvenanceLikeCpp {
                    cast_id,
                    spell_visual_id: 42,
                    caster_level: Some(71),
                },
            );
            if selected_aura_types(spell_id).is_some() {
                assert!(auras.set_aura_persistence_metadata(
                    slot,
                    AuraPersistenceMetadata {
                        item_guid: ObjectGuid::EMPTY,
                        cast_item_id: 0,
                        cast_item_level: 63,
                        recalculate_mask: 0b111,
                        charges: 3,
                        base_amounts: vec![(0, 0), (1, 0), (2, 0)],
                    },
                ));
            }
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

fn decode_selected_delta(bytes: &[u8]) -> (u16, u32, u16, u8, u32, bool, Vec<f32>) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AuraUpdate as u16
    );
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(9).unwrap(), 1);
    let _slot = packet.read_uint8().unwrap();
    assert!(packet.read_bit().unwrap());
    let _cast_id = packet.read_packed_guid().unwrap();
    let _spell = packet.read_int32().unwrap();
    let _visual = packet.read_int32().unwrap();
    let flags = packet.read_uint16().unwrap();
    let active_flags = packet.read_uint32().unwrap();
    let cast_level = packet.read_uint16().unwrap();
    let applications = packet.read_uint8().unwrap();
    let _content_tuning = packet.read_int32().unwrap();
    let has_caster = packet.read_bit().unwrap();
    let has_duration = packet.read_bit().unwrap();
    let has_remaining = packet.read_bit().unwrap();
    let _has_time_mod = packet.read_bit().unwrap();
    let point_count = packet.read_bits(6).unwrap();
    let _estimated_points = packet.read_bits(6).unwrap();
    let _has_content_tuning = packet.read_bit().unwrap();
    packet.reset_bits();
    if has_caster {
        let _caster = packet.read_packed_guid().unwrap();
    }
    if has_duration {
        let _duration = packet.read_int32().unwrap();
    }
    if has_remaining {
        let _remaining = packet.read_int32().unwrap();
    }
    let points = (0..point_count)
        .map(|_| packet.read_float().unwrap())
        .collect();
    let _target = packet.read_packed_guid().unwrap();
    assert_eq!(packet.remaining(), 0);
    (
        flags,
        active_flags,
        cast_level,
        applications,
        point_count,
        has_duration,
        points,
    )
}

#[test]
fn current_race_sickness_and_ghost_wisp_are_prepared_in_slot_order_once() {
    let (mut manager, handle) = fixture();
    let race_store = races(SICKNESS);
    let spells = spell_catalog();
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_ids([0]);
    let ghost = install(&mut manager, handle, 7, 8326);
    let sickness = install(&mut manager, handle, 2, SICKNESS);
    let wisp = install(&mut manager, handle, 5, 20584);
    install(&mut manager, handle, 1, 99_007);
    install(&mut manager, handle, 9, 118);

    let selected_catalogs = catalogs(
        Some(&race_store),
        Some(&spells),
        Some(&options),
        Some(&difficulties),
    );
    let updates =
        collect_player_pending_selected_aura_updates(&mut manager, handle, selected_catalogs)
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
        collect_player_pending_selected_aura_updates(&mut manager, handle, selected_catalogs)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn selected_publication_delta_keeps_hidden_and_zero_duration_and_sparse_points() {
    for (duration, attributes5) in [(60_000, 0x400), (0, 0)] {
        let (mut manager, handle) = fixture();
        let race_store = races(SICKNESS);
        let mut spells = spell_catalog();
        let mut attributes = [0; 15];
        attributes[5] = attributes5;
        attributes[11] = 0x4;
        spells.insert_spell_misc_attributes_for_difficulty_like_cpp(8326, 0, attributes);
        let options = SpellAuraOptionsStore::from_entries([]);
        let difficulties = DifficultyStore::from_ids([0]);
        install_with_mask(&mut manager, handle, 6, 8326, 0b101);
        manager
            .with_player_mut_like_cpp(handle, |player| {
                let aura = player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .runtime_application_mut_like_cpp(6)
                    .unwrap();
                aura.duration_total = duration;
                aura.duration_remaining = duration / 2;
                aura.aura_flags = 0x8;
            })
            .unwrap();

        let updates = collect_player_pending_selected_aura_updates(
            &mut manager,
            handle,
            catalogs(
                Some(&race_store),
                Some(&spells),
                Some(&options),
                Some(&difficulties),
            ),
        )
        .unwrap();
        assert_eq!(updates.len(), 1);
        let (flags, active_flags, cast_level, applications, point_count, has_duration, points) =
            decode_selected_delta(updates[0].packet_bytes());
        assert_eq!(flags & 0x8, 0x8);
        assert_eq!(flags & 0x4, 0);
        assert_eq!(active_flags, 0b101);
        assert_eq!(cast_level, 63, "item level comes from persistence metadata");
        assert_eq!(applications, 3, "empty options use the canonical charges");
        assert_eq!(point_count, 3);
        assert!(!has_duration);
        assert_eq!(points, [101.0, 0.0, 103.0]);
    }
}

#[test]
fn separate_recovery_owners_collect_only_their_current_player_once() {
    let (mut manager, first_handle) = fixture();
    let race_store = races(SICKNESS);
    let spells = spell_catalog();
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_ids([0]);
    let selected_catalogs = catalogs(
        Some(&race_store),
        Some(&spells),
        Some(&options),
        Some(&difficulties),
    );
    let second_guid = ObjectGuid::create_player(1, 84_200_003);
    let second_handle = add_player(&mut manager, second_guid);
    install(&mut manager, first_handle, 3, 8326);
    install(&mut manager, second_handle, 6, SICKNESS);

    let first =
        collect_player_pending_selected_aura_updates(&mut manager, first_handle, selected_catalogs)
            .unwrap();
    assert_eq!(first.len(), 1);
    assert!(pending(&manager, first_handle).is_empty());
    assert_eq!(pending(&manager, second_handle), [6]);

    let second = collect_player_pending_selected_aura_updates(
        &mut manager,
        second_handle,
        selected_catalogs,
    )
    .unwrap();
    assert_eq!(second.len(), 1);
    assert!(pending(&manager, second_handle).is_empty());
    assert!(!first.is_empty() && !second.is_empty());
    assert_eq!(
        collect_player_pending_selected_aura_updates(
            &mut manager,
            first_handle,
            selected_catalogs,
        )
        .unwrap()
        .len(),
        0
    );

    assert!(first.iter().chain(&second).all(|update| {
        update
            .packet_bytes()
            .starts_with(&(wow_constants::ServerOpcodes::AuraUpdate as u16).to_le_bytes())
    }));
}

#[test]
fn empty_pending_needs_no_race_lookup_but_classification_requires_known_metadata() {
    let (mut manager, handle) = fixture();
    let missing = ChrRacesStore::from_entries([]);
    let empty_catalogs = catalogs(None, None, None, None);
    assert!(
        collect_player_pending_selected_aura_updates(&mut manager, handle, empty_catalogs)
            .unwrap()
            .is_empty()
    );
    assert!(
        collect_player_pending_selected_aura_updates(
            &mut manager,
            handle,
            catalogs(Some(&missing), None, None, None),
        )
        .unwrap()
        .is_empty()
    );
    install(&mut manager, handle, 0, 8326);
    let invalid = races(-1);
    for catalog in [None, Some(&missing), Some(&invalid)] {
        assert!(
            collect_player_pending_selected_aura_updates(
                &mut manager,
                handle,
                catalogs(catalog, None, None, None),
            )
            .is_err()
        );
        assert_eq!(pending(&manager, handle), [0]);
    }
    let spells = spell_catalog();
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_ids([0]);
    assert_eq!(
        collect_player_pending_selected_aura_updates(
            &mut manager,
            handle,
            catalogs(
                Some(&races(0)),
                Some(&spells),
                Some(&options),
                Some(&difficulties)
            ),
        )
        .unwrap()
        .len(),
        1
    );
    install(&mut manager, handle, 4, 99_007);
    assert!(
        collect_player_pending_selected_aura_updates(
            &mut manager,
            handle,
            catalogs(Some(&races(0)), None, None, None),
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(pending(&manager, handle), [4]);
}

#[test]
fn incomplete_pending_snapshot_never_consumes_the_other_selected_slot() {
    for malformed_slot in [false, true] {
        let (mut manager, handle) = fixture();
        let race_store = races(SICKNESS);
        let spells = spell_catalog();
        let options = SpellAuraOptionsStore::from_entries([]);
        let difficulties = DifficultyStore::from_ids([0]);
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
                catalogs(
                    Some(&race_store),
                    Some(&spells),
                    Some(&options),
                    Some(&difficulties),
                )
            )
            .is_err()
        );
        assert_eq!(pending(&manager, handle), [0, 3]);
    }
}

#[test]
fn selected_publication_bad_metadata_keeps_every_pending_slot() {
    let (mut manager, handle) = fixture();
    let race_store = races(SICKNESS);
    let spells = spell_catalog();
    let options = SpellAuraOptionsStore::from_entries([]);
    let difficulties = DifficultyStore::from_ids([0]);
    install(&mut manager, handle, 2, 8326);
    install(&mut manager, handle, 7, 20584);
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_aura_cast_provenance_like_cpp(7, AuraCastProvenanceLikeCpp::default());
        })
        .unwrap();

    let result = collect_player_pending_selected_aura_updates(
        &mut manager,
        handle,
        catalogs(
            Some(&race_store),
            Some(&spells),
            Some(&options),
            Some(&difficulties),
        ),
    );
    assert!(
        result.is_err(),
        "missing selected cast facts hold publication"
    );
    assert_eq!(pending(&manager, handle), [2, 7]);
}

#[test]
fn stale_detached_or_invalid_canonical_sources_preserve_pending() {
    for mode in 0..7 {
        let (mut manager, handle) = fixture();
        let race_store = races(SICKNESS);
        let spells = spell_catalog();
        let options = SpellAuraOptionsStore::from_entries([]);
        let difficulties = DifficultyStore::from_ids([0]);
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
            6 => {
                manager.detach_player_like_cpp(handle).unwrap();
                assert!(manager.destroy_map(MAP.map_id, MAP.instance_id));
                manager.create_world_map(MAP.map_id, MAP.instance_id);
                let replacement = add_player(&mut manager, handle.guid());
                install(&mut manager, replacement, 0, 8326);
                replacement
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
                catalogs(
                    Some(&race_store),
                    Some(&spells),
                    Some(&options),
                    Some(&difficulties),
                )
            )
            .is_err()
        );
        assert_eq!(pending(&manager, current), [0]);
    }
}
