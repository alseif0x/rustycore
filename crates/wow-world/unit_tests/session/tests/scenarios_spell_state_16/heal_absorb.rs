use super::*;

/// C++ `Unit::HealBySpell`'s heal-absorb stage (`Unit.cpp:2020-2084`,
/// `6557-6564`) for the session's own player.
///
/// Every `SPELL_AURA_SCHOOL_HEAL_ABSORB` whose `MiscValue` covers the heal's
/// school spends its amount before the heal lands, publishes one
/// `SMSG_SPELL_HEAL_ABSORB_LOG` per consuming shield and is removed once spent;
/// the heal log then reports the absorbed amount and the heal the target
/// actually received.
#[tokio::test]
async fn apply_heal_spends_player_heal_absorb_shield_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let player = ObjectGuid::create_player(1, 91_700);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Victim".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(40);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, is_heal) in [
        (91_620_i32, 0_i32, 20_i32, true),
        (
            91_621,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_HEAL_ABSORB,
            30,
            false,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: if is_heal {
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL
                } else {
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                },
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: (!is_heal).then_some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: if is_heal {
                        wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL
                    } else {
                        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    },
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    effect_misc_value_1: if is_heal { 0 } else { 0x01 },
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(91_621, player, 30_000, 1)
        .expect("apply heal-absorb shield");

    let shield_amount = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == 91_621)
                    .map(|aura| {
                        aura.represented_effect_amounts
                            .iter()
                            .find(|represented| represented.effect_index == 0)
                            .map(|represented| represented.amount)
                    })
            })
            .flatten()
    };

    // The 30-point shield absorbs the whole 20-point heal, publishes the absorb
    // log and keeps 10 points.
    session
        .apply_heal(Some(91_620), player, 20)
        .await
        .expect("heal executes");
    assert_eq!(session.player_health_like_cpp(), 40);
    assert_eq!(shield_amount(&session), Some(Some(10)));
    let packets = drain_server_packet_bytes(&send_rx);
    let absorb_log = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellHealAbsorbLog)
        })
        .expect("heal absorb log");
    let mut log = wow_packet::WorldPacket::from_bytes(absorb_log);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("target"), player);
    assert_eq!(log.read_packed_guid().expect("absorb caster"), player);
    assert_eq!(log.read_packed_guid().expect("healer"), player);
    assert_eq!(log.read_int32().expect("absorb spell id"), 91_621);
    assert_eq!(log.read_int32().expect("absorbed spell id"), 91_620);
    assert_eq!(log.read_int32().expect("absorbed"), 20);
    assert_eq!(log.read_int32().expect("original heal"), 20);
    assert!(!log.has_bit().expect("content tuning"));
    assert!(log.is_empty());

    let heal_log = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellHealLog)
        })
        .expect("heal log");
    let mut log = wow_packet::WorldPacket::from_bytes(heal_log);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("target"), player);
    assert_eq!(log.read_packed_guid().expect("caster"), player);
    assert_eq!(log.read_int32().expect("spell id"), 91_620);
    // `HealInfo::AbsorbHeal` reduced both the heal and the effective heal.
    assert_eq!(log.read_int32().expect("health"), 0);
    assert_eq!(log.read_int32().expect("original heal"), 20);
    assert_eq!(log.read_int32().expect("over heal"), 0);
    assert_eq!(log.read_int32().expect("absorbed"), 20);

    // The remaining 10 points absorb half of the next heal and the spent shield
    // is removed.
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .apply_heal(Some(91_620), player, 20)
        .await
        .expect("heal executes");
    assert_eq!(session.player_health_like_cpp(), 50);
    assert_eq!(shield_amount(&session), None);
}
