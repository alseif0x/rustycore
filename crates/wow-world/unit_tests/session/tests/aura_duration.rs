//! Canonical Player aura-duration updates and expiry.

use super::*;

struct DecodedAuraInfo {
    cast_id: ObjectGuid,
    spell_id: i32,
    spell_visual_id: i32,
    duration_ms: Option<u32>,
    remaining_ms: Option<u32>,
}

fn decode_aura_update(bytes: &[u8]) -> Vec<DecodedAuraInfo> {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(packet.server_opcode(), Some(ServerOpcodes::AuraUpdate));
    packet.skip_opcode();
    assert!(!packet.read_bit().unwrap());
    let aura_count = packet.read_bits(9).unwrap();
    let mut auras = Vec::new();
    for _ in 0..aura_count {
        let _slot = packet.read_uint8().unwrap();
        if !packet.read_bit().unwrap() {
            packet.reset_bits();
            continue;
        }
        let cast_id = packet.read_packed_guid().unwrap();
        let spell_id = packet.read_int32().unwrap();
        let spell_visual_id = packet.read_int32().unwrap();
        let _flags = packet.read_uint16().unwrap();
        let _active_flags = packet.read_uint32().unwrap();
        let _cast_level = packet.read_uint16().unwrap();
        let _applications = packet.read_uint8().unwrap();
        let _content_tuning_id = packet.read_int32().unwrap();
        let has_cast_unit = packet.read_bit().unwrap();
        let has_duration = packet.read_bit().unwrap();
        let has_remaining = packet.read_bit().unwrap();
        let _has_time_mod = packet.read_bit().unwrap();
        let point_count = packet.read_bits(6).unwrap();
        let _estimated_point_count = packet.read_bits(6).unwrap();
        let _has_content_tuning = packet.read_bit().unwrap();
        packet.reset_bits();
        if has_cast_unit {
            let _caster_guid = packet.read_packed_guid().unwrap();
        }
        let duration_ms = has_duration.then(|| packet.read_uint32().unwrap());
        let remaining_ms = has_remaining.then(|| packet.read_uint32().unwrap());
        for _ in 0..point_count {
            let _point = packet.read_float().unwrap();
        }
        auras.push(DecodedAuraInfo {
            cast_id,
            spell_id,
            spell_visual_id,
            duration_ms,
            remaining_ms,
        });
    }
    auras
}

fn canonical_aura(
    session: &WorldSession,
    slot: u8,
) -> (AuraApplication, wow_entities::AuraCastProvenanceLikeCpp) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            (
                auras
                    .runtime_application_like_cpp(slot)
                    .expect("runtime aura application")
                    .clone(),
                auras.aura_cast_provenance_like_cpp(slot),
            )
        })
        .expect("canonical player owner")
}

fn install_known_empty_sickness_race(session: &mut WorldSession, race_id: u8) {
    session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_race(race_id))
        .expect("canonical Player owner");
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([race_entry(
            u32::from(race_id),
            0,
        )]),
    ));
}

fn race_entry(
    id: u32,
    res_sickness_spell_id: i32,
) -> wow_data::character_progression::ChrRacesEntry {
    wow_data::character_progression::ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: "Aura duration test race".into(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id,
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
    }
}

fn selected_sickness_info(spell_id: i32) -> wow_data::SpellInfo {
    use wow_data::spell::aura_types as aura;
    use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: SPELL_EFFECT_APPLY_AURA,
        effect_base_points: -76,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura::SPELL_AURA_MOD_PERCENT_STAT),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![
            wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura::SPELL_AURA_MOD_PERCENT_STAT,
                effect_base_points: -76,
                effect_die_sides: 1,
                effect_misc_value_1: -1,
                ..Default::default()
            },
            wow_data::SpellEffectInfo {
                effect_index: 1,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
                effect_base_points: -76,
                effect_die_sides: 1,
                effect_misc_value_1: 127,
                ..Default::default()
            },
            wow_data::SpellEffectInfo {
                effect_index: 2,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura::SPELL_AURA_NO_PVP_CREDIT,
                effect_base_points: -1,
                effect_die_sides: 1,
                effect_misc_value_1: 127,
                ..Default::default()
            },
        ],
    }
}

#[tokio::test]
async fn set_aura_duration_updates_packet_provenance_and_remaining_lifetime() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 8_430);
    let caster_guid = ObjectGuid::create_player(1, 8_431);
    let other_caster_guid = ObjectGuid::create_player(1, 8_432);
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let cast_id = ObjectGuid::new(0x1234, 0x5678);
    let provenance = wow_entities::AuraCastProvenanceLikeCpp {
        cast_id,
        spell_visual_id: 245_039,
        caster_level: None,
    };

    session
        .apply_aura_with_effect_mask_and_provenance_like_cpp(
            84_301,
            caster_guid,
            600_000,
            0x0000_0015,
            0x7,
            provenance,
        )
        .expect("finite aura application");
    session
        .apply_aura_with_effect_mask_and_provenance_like_cpp(
            84_302,
            caster_guid,
            0,
            0x0000_0001,
            0x1,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
        .expect("permanent aura application");
    install_known_empty_sickness_race(&mut session, 4);
    let _ = drain_server_packet_bytes(&send_rx);
    let permanent_application = canonical_aura(&session, 1).0;
    let permanent_info =
        wow_world_spell::player_aura_info_like_cpp(&permanent_application, 80, 571);
    let permanent_data = permanent_info.aura_data.expect("permanent AuraInfo data");
    assert_eq!(permanent_data.duration_ms, None);
    assert_eq!(permanent_data.remaining_ms, None);

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_301,
            other_caster_guid,
            120_000,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        0
    );
    assert_eq!(canonical_aura(&session, 0).0.duration_remaining, 600_000);
    assert!(
        send_rx.try_recv().is_err(),
        "caster mismatch publishes nothing"
    );

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_301,
            caster_guid,
            120_000,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        1
    );
    let updates = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        updates.len(),
        1,
        "one canonical duration delta is published"
    );
    let aura = decode_aura_update(&updates[0]).pop().expect("updated aura");
    assert_eq!(aura.cast_id, cast_id);
    assert_eq!(aura.spell_id, 84_301);
    assert_eq!(aura.spell_visual_id, 245_039);
    assert_eq!(aura.duration_ms, Some(600_000));
    assert_eq!(aura.remaining_ms, Some(120_000));

    let (finite, finite_provenance) = canonical_aura(&session, 0);
    assert_eq!(finite.duration_total, 600_000);
    assert_eq!(finite.duration_remaining, 120_000);
    assert_eq!(finite_provenance, provenance);

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_301,
            caster_guid,
            0,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        1
    );
    let zero_duration_update = drain_server_packet_bytes(&send_rx);
    assert_eq!(zero_duration_update.len(), 1);
    let zero_duration_aura = decode_aura_update(&zero_duration_update[0])
        .pop()
        .expect("finite aura duration-zero update");
    assert_eq!(zero_duration_aura.duration_ms, Some(600_000));
    assert_eq!(zero_duration_aura.remaining_ms, Some(0));
    assert_eq!(canonical_aura(&session, 0).0.duration_total, 600_000);
    assert_eq!(canonical_aura(&session, 0).0.duration_remaining, 0);

    let signed_application = session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .set_aura_duration(
                    84_301,
                    caster_guid,
                    -1,
                    wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
                )
                .into_iter()
                .next()
                .expect("matching canonical aura")
                .0
        })
        .expect("canonical player owner");
    assert_eq!(signed_application.duration_total, 600_000);
    assert_eq!(signed_application.duration_remaining, -1);
    let signed_packet = wow_packet::packets::misc::AuraUpdate {
        unit_guid: player_guid,
        update_all: false,
        auras: vec![wow_world_spell::player_aura_info_like_cpp(
            &signed_application,
            80,
            571,
        )],
    };
    let signed_bytes = wow_packet::ServerPacket::to_bytes(&signed_packet);
    let signed_aura = decode_aura_update(&signed_bytes)
        .pop()
        .expect("signed duration AuraInfo");
    assert_eq!(signed_aura.duration_ms, Some(600_000));
    assert_eq!(signed_aura.remaining_ms, Some(u32::MAX));
    assert_eq!(
        signed_aura
            .remaining_ms
            .map(|bits| i32::from_ne_bytes(bits.to_ne_bytes())),
        Some(-1),
        "the packet keeps the signed -1 value as its unchanged u32 wire bits"
    );

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_301,
            caster_guid,
            120_000,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        1
    );
    let _ = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_301,
            caster_guid,
            u32::MAX,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        0,
        "unsigned legacy values outside the signed duration range are rejected"
    );
    assert_eq!(canonical_aura(&session, 0).0.duration_remaining, 120_000);
    assert!(send_rx.try_recv().is_err());

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            84_302,
            caster_guid,
            0,
            wow_entities::AuraDurationUpdatePolicy::LegacyElapsed,
        ),
        1
    );
    let _ = drain_server_packet_bytes(&send_rx);
    let stale = std::time::Instant::now() - std::time::Duration::from_secs(121);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras
                .runtime_application_mut_like_cpp(0)
                .unwrap()
                .applied_at = stale;
            auras
                .runtime_application_mut_like_cpp(1)
                .unwrap()
                .applied_at = stale;
        })
        .expect("canonical player owner");

    wow_world_spell::tick_auras(&mut session);

    let remaining = crate::session::hub_ref(&session)
        .resolved_player_visible_auras_like_cpp()
        .expect("canonical aura snapshot");
    assert!(!remaining.values().any(|aura| aura.spell_id == 84_301));
    assert!(remaining.values().any(|aura| {
        aura.spell_id == 84_302 && aura.duration_total == -1 && aura.duration_remaining == 0
    }));
}

#[test]
fn legacy_player_sweep_keeps_nonempty_auras_when_race_classification_is_unknown() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 8_435);
    let caster_guid = ObjectGuid::create_player(1, 8_436);
    const UNRELATED_AURA: i32 = 92_017;
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    install_known_empty_sickness_race(&mut session, 4);
    session
        .apply_aura_with_effect_mask_and_provenance_like_cpp(
            UNRELATED_AURA,
            caster_guid,
            30_000,
            1,
            1,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
        .expect("canonical unrelated Aura application");
    let stale = std::time::Instant::now() - std::time::Duration::from_secs(60);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(0)
                .unwrap()
                .applied_at = stale;
        })
        .expect("canonical Player owner");
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([]),
    ));

    wow_world_spell::tick_auras_expiry_only(&mut session);
    assert!(canonical_aura(&session, 0).0.spell_id == UNRELATED_AURA);

    install_known_empty_sickness_race(&mut session, 4);
    wow_world_spell::tick_auras_expiry_only(&mut session);
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(0)
                .is_none())
            .expect("canonical Player owner")
    );
}

#[test]
fn selected_sickness_duration_update_keeps_unit_clock_and_waits_for_recovery() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 8_433);
    let caster_guid = ObjectGuid::create_player(1, 8_434);
    const SICKNESS: i32 = 54_321;
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session
        .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_race(4))
        .expect("canonical Player owner");
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: 4,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "Selected sickness duration race".into(),
                flags: 0,
                male_display_id: 0,
                female_display_id: 0,
                high_res_male_display_id: 0,
                high_res_female_display_id: 0,
                res_sickness_spell_id: SICKNESS,
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
    let mut spell_store =
        crate::selected_spell_fixtures::canonical_spell_store([(SICKNESS as u32, 0)]);
    spell_store.insert_spell_misc_attributes_like_cpp(SICKNESS, [0; 15]);
    let spell_info = selected_sickness_info(SICKNESS);
    spell_store.insert(SICKNESS, spell_info.clone());
    session.set_spell_store(std::sync::Arc::new(spell_store));
    session.set_chr_classes_store(std::sync::Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([
            wow_data::character_progression::ChrClassesEntry {
                id: 1,
                starting_level: 1,
                ..Default::default()
            },
        ]),
    ));
    session.set_player_stats(std::sync::Arc::new(
        wow_data::PlayerStatsStore::from_entries([(
            (4, 1, 1),
            wow_data::PlayerLevelStats {
                strength: 10,
                agility: 10,
                stamina: 10,
                intellect: 10,
                spirit: 10,
                base_mana: 100,
            },
        )]),
    ));
    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS,
            caster_guid,
            &spell_info,
            600_000,
            0,
            0x88,
            0,
            Ok((0x0011, wow_entities::AuraCastProvenanceLikeCpp::default(), 0)),
            wow_entities::SpellCastMetadata::default(),
        )
        .expect("canonical sickness application");
    let _ = drain_server_packet_bytes(&send_rx);
    let stale = std::time::Instant::now() - std::time::Duration::from_secs(60);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(0)
                .expect("selected sickness application")
                .applied_at = stale;
        })
        .expect("canonical Player owner");

    assert_eq!(
        wow_world_spell::set_player_aura_duration(
            &mut session,
            SICKNESS,
            caster_guid,
            120_000,
            wow_entities::AuraDurationUpdatePolicy::SelectedUnitUpdate,
        ),
        1
    );
    assert!(send_rx.try_recv().is_err(), "Unit publication is deferred");
    let (application, _) = canonical_aura(&session, 0);
    assert_eq!(application.duration_total, 600_000);
    assert_eq!(application.duration_remaining, 120_000);
    assert_eq!(application.applied_at, stale);
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&0))
            .expect("canonical Player owner")
    );

    wow_world_spell::tick_auras_expiry_only(&mut session);
    assert_eq!(canonical_aura(&session, 0).0.duration_remaining, 120_000);
    assert!(send_rx.try_recv().is_err());
}
