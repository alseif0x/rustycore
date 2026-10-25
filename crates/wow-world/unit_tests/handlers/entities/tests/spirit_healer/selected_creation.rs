use super::*;

async fn sickness_session(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    wow_data::SpellInfo,
) {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(guid_counter).await;
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, None);
    let player_guid = session.player_guid().expect("canonical Player GUID");
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("selected sickness SpellInfo");
    (session, send_rx, player_guid, spell_info)
}

fn provenance(guid_counter: i64, caster_level: u8) -> wow_entities::AuraCastProvenanceLikeCpp {
    wow_entities::AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Cast,
            3,
            1,
            571,
            0,
            SICKNESS_SPELL as u32,
            guid_counter,
        ),
        spell_visual_id: 245_039,
        caster_level: Some(caster_level),
    }
}

fn apply_sickness(
    session: &mut WorldSession,
    caster_guid: ObjectGuid,
    spell_info: &wow_data::SpellInfo,
    duration_total: i32,
    creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp), &'static str>,
) -> Result<(), &'static str> {
    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            caster_guid,
            spell_info,
            duration_total,
            0,
            0x88,
            0,
            0,
            creation,
        )
}

fn snapshot(
    session: &WorldSession,
    slot: u8,
) -> (
    wow_entities::AuraApplicationLikeCpp,
    Vec<(u32, i32, i32, i32)>,
    std::collections::HashSet<u8>,
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            (
                auras
                    .runtime_application_like_cpp(slot)
                    .expect("selected sickness application")
                    .clone(),
                auras
                    .threat_snapshot_like_cpp(slot)
                    .expect("selected sickness effect snapshot")
                    .effects()
                    .to_vec(),
                auras.visible_auras_to_update.clone(),
            )
        })
        .expect("canonical sickness Player")
}

#[tokio::test]
async fn selected_creation_sickness_uses_source_base_flags_and_finite_wire_flags() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_801).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
        sickness_level: 11,
        corpse_reclaim_delay_pve: false,
        ..Default::default()
    });
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    let position = session.player_position_like_cpp().expect("ghost position");
    let healer = insert_healer(
        &session,
        43_802,
        position,
        wow_constants::unit::NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    let applications = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].aura_flags, 0x0011);
    let provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(applications[0].slot)
        })
        .expect("sickness Aura provenance");
    assert_eq!(provenance.caster_level, Some(11));

    session.update(0).await;
    let wire_updates = drain_packets(&send_rx)
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|update| update.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert!(
        !wire_updates.is_empty(),
        "Recovery publishes the selected sickness Aura"
    );
    assert!(
        wire_updates
            .iter()
            .all(|update| update.aura_flags == 0x0015)
    );
}

#[tokio::test]
async fn selected_creation_same_key_refresh_ignores_failure_and_retains_original_provenance() {
    let (mut session, _send_rx, player_guid, spell_info) = sickness_session(43_811).await;
    let old_provenance = provenance(43_812, 17);
    apply_sickness(
        &mut session,
        player_guid,
        &spell_info,
        900_000,
        Ok((0x0011, old_provenance)),
    )
    .expect("install initial selected sickness Aura");
    let application = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("initial sickness Aura");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(application.slot)
                .expect("initial sickness Aura")
                .duration_remaining = 10_000;
        })
        .expect("canonical sickness Player");
    let before = snapshot(&session, application.slot);
    let before_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(application.slot)
        })
        .expect("initial sickness provenance");
    let mut refreshed_spell = spell_info.clone();
    refreshed_spell.effects[0].effect_base_points = -31;

    apply_sickness(
        &mut session,
        player_guid,
        &refreshed_spell,
        600_000,
        Err("creation-only metadata failure"),
    )
    .expect("same-key full-mask refresh does not need fresh-creation metadata");

    let after = snapshot(&session, application.slot);
    let after_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(application.slot)
        })
        .expect("refreshed sickness provenance");
    assert_eq!(after.0.aura_flags, before.0.aura_flags);
    assert_eq!(after.0.aura_flags, 0x0011);
    assert_eq!(after.0.duration_total, 600_000);
    assert_eq!(after.0.duration_remaining, 600_000);
    assert_ne!(
        after.1, before.1,
        "the selected effect values still refresh"
    );
    assert_eq!(after_provenance.cast_id, before_provenance.cast_id);
    assert_eq!(after_provenance.cast_id, old_provenance.cast_id);
    assert_eq!(
        after_provenance.spell_visual_id,
        old_provenance.spell_visual_id
    );
    assert_eq!(after_provenance.caster_level, Some(17));
    assert!(after.2.contains(&application.slot));
}

#[tokio::test]
async fn selected_creation_failure_preserves_existing_foreign_caster_sickness() {
    let (mut session, _send_rx, player_guid, spell_info) = sickness_session(43_821).await;
    let old_caster = ObjectGuid::create_player(1, 43_822);
    apply_sickness(
        &mut session,
        old_caster,
        &spell_info,
        900_000,
        Ok((0x0010, provenance(43_823, 24))),
    )
    .expect("install foreign-caster sickness Aura");
    let old = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("foreign-caster sickness Aura");
    let before = snapshot(&session, old.slot);
    let before_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(old.slot)
        })
        .expect("foreign-caster sickness provenance");
    let before_damage = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical sickness combat stats")
        .mod_damage_done_percent;

    let result = apply_sickness(
        &mut session,
        player_guid,
        &spell_info,
        600_000,
        Err("fresh flags unavailable"),
    );

    assert_eq!(result, Err("fresh flags unavailable"));
    let after = snapshot(&session, old.slot);
    let after_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(old.slot)
        })
        .expect("foreign-caster sickness provenance");
    assert_eq!(after.0.caster_guid, old_caster);
    assert_eq!(after.0.effect_mask, before.0.effect_mask);
    assert_eq!(after.0.aura_flags, before.0.aura_flags);
    assert_eq!(after.0.duration_remaining, before.0.duration_remaining);
    assert_eq!(after.1, before.1);
    assert_eq!(after.2, before.2);
    assert_eq!(after_provenance.cast_id, before_provenance.cast_id);
    assert_eq!(
        after_provenance.caster_level,
        before_provenance.caster_level
    );
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("canonical sickness combat stats")
            .mod_damage_done_percent,
        before_damage
    );
}
