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
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        wow_world_spell::set_player_aura_duration(&mut session, 84_301, other_caster_guid, 120_000),
        0
    );
    assert_eq!(canonical_aura(&session, 0).0.duration_remaining, 600_000);
    assert!(
        send_rx.try_recv().is_err(),
        "caster mismatch publishes nothing"
    );

    assert_eq!(
        wow_world_spell::set_player_aura_duration(&mut session, 84_301, caster_guid, 120_000),
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
        wow_world_spell::set_player_aura_duration(&mut session, 84_301, caster_guid, 0),
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

    assert_eq!(
        wow_world_spell::set_player_aura_duration(&mut session, 84_301, caster_guid, 120_000),
        1
    );
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        wow_world_spell::set_player_aura_duration(&mut session, 84_302, caster_guid, 0),
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
        aura.spell_id == 84_302 && aura.duration_total == 0 && aura.duration_remaining == 0
    }));
}
