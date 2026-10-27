use super::*;
use std::sync::Arc;
use wow_core::Position;

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
    creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp, u8), &'static str>,
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
            creation,
            wow_entities::SpellCastMetadata::default(),
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
        Ok((0x0011, old_provenance, 0)),
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
        Ok((0x0010, provenance(43_823, 24), 0)),
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

fn install_mixed_sickness_polarity(session: &mut WorldSession) {
    let existing = session
        .catalogs
        .spell_store()
        .cloned()
        .expect("sickness SpellStore");
    let mut spells = crate::selected_spell_fixtures::canonical_spell_store(
        existing.spell_info_keys_in_order_like_cpp(),
    );
    for spell_id in [8326, 20584, SICKNESS_SPELL] {
        if let Some(info) = existing.get(spell_id) {
            spells.insert(spell_id, info.clone());
        }
        for difficulty_id in u8::MIN..=u8::MAX {
            if let Some(attributes) =
                existing.misc_attributes_for_exact_difficulty(spell_id, difficulty_id)
            {
                spells.insert_spell_misc_attributes_for_difficulty_like_cpp(
                    spell_id,
                    difficulty_id,
                    attributes,
                );
            }
        }
    }
    let mut spell = existing
        .get(SICKNESS_SPELL)
        .cloned()
        .expect("sickness SpellInfo");
    for (index, points) in [(0, 80), (1, -79), (2, 159)] {
        let effect = spell
            .effects
            .iter_mut()
            .find(|effect| effect.effect_index == index)
            .expect("supported sickness effect index");
        effect.effect_base_points = points;
        effect.polarity_metadata = Some(wow_data::SpellEffectPolarityMetadata::default());
    }
    spells.insert(SICKNESS_SPELL, spell);
    let mut sickness_attributes = existing
        .misc_attributes_for_difficulty_like_cpp(SICKNESS_SPELL, 0, None)
        .expect("stock sickness SpellMisc attributes");
    sickness_attributes[0] &= !0x0400_0000;
    spells.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL, sickness_attributes);
    session.set_spell_store(Arc::new(spells));
    let mut misc_rows = crate::session::hub_ref(session)
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .map(|store| store.entries_like_cpp().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let sickness_misc = misc_rows
        .iter_mut()
        .find(|row| row.spell_id == SICKNESS_SPELL as u32 && row.difficulty_id == 0)
        .expect("stock sickness SpellMisc row");
    sickness_misc.attributes = sickness_attributes.map(|attribute| attribute as i32);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries(Vec::new()),
    ));
    session.catalogs.factions.template_store = Some(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries(Vec::new()),
    ));
}

async fn execute_mixed_sickness(
    session: &mut WorldSession,
    target: ObjectGuid,
    caster: ObjectGuid,
) {
    let generators = session.id_generators_for_test_like_cpp();
    let spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        session,
        generators.item.as_ref(),
        &spawns,
        SICKNESS_SPELL,
        target,
        wow_entities::SpellCastMetadata {
            trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
            original_caster_guid: Some(caster),
            ..Default::default()
        },
    )
    .await
    .expect("selected sickness cast");
}

async fn mixed_sickness_creation(
    guid_counter: i64,
    friendly_group: bool,
    refresh_after_relation_becomes_unavailable: bool,
) -> (u32, Option<u8>, u16) {
    let (mut session, send_rx, target_guid, _) = sickness_session(guid_counter).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    install_mixed_sickness_polarity(&mut session);
    let owner = session
        .core
        .player_handle_like_cpp
        .expect("canonical sickness Player handle");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let caster_guid = ObjectGuid::create_player(1, guid_counter + 1);
    let group_guid = ObjectGuid::create_group(
        u64::try_from(guid_counter + 2).expect("positive test group GUID counter"),
    );
    let caster_handle = {
        let mut manager = manager.lock().expect("canonical MapManager");
        let mut caster = Box::new(wow_entities::Player::new(Some((guid_counter + 1) as u64), false));
        caster.unit_mut().world_mut().object_mut().create(caster_guid);
        caster.unit_mut().set_level(44);
        caster.unit_mut().set_unit_flags_like_cpp(
            wow_constants::unit::UnitFlags::PLAYER_CONTROLLED,
        );
        let caster_handle = manager
            .install_detached_player_like_cpp(caster)
            .expect("foreign caster Player");
        manager
            .attach_player_like_cpp(caster_handle, wow_map::MapKey::new(571, 0), Position::ZERO)
            .expect("same-map foreign caster");

        manager
            .with_player_mut_like_cpp(owner, |player| {
                player.unit_mut().set_unit_flags_like_cpp(
                    wow_constants::unit::UnitFlags::PLAYER_CONTROLLED,
                );
                if friendly_group {
                    player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                        group_guid,
                        leader_guid: target_guid,
                        role_mask: 0,
                        subgroup: 0,
                    }));
                }
            })
            .expect("canonical target Player");
        manager
            .with_player_mut_like_cpp(caster_handle, |player| {
                player.unit_mut().set_unit_flags_like_cpp(
                    wow_constants::unit::UnitFlags::PLAYER_CONTROLLED,
                );
                if friendly_group {
                    player.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                        group_guid,
                        leader_guid: target_guid,
                        role_mask: 0,
                        subgroup: 0,
                    }));
                } else {
                    player.set_duel_opponent_in_progress_like_cpp(target_guid);
                }
            })
            .expect("foreign caster Player");
        caster_handle
    };

    execute_mixed_sickness(&mut session, target_guid, caster_guid).await;
    if refresh_after_relation_becomes_unavailable {
        manager
            .lock()
            .expect("canonical MapManager")
            .with_player_mut_like_cpp(caster_handle, |player| {
                player.set_group_like_cpp(None);
            })
            .expect("foreign caster Player");
        session.catalogs.factions.template_store = None;
        execute_mixed_sickness(&mut session, target_guid, caster_guid).await;
    }

    let (application, provenance) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let application = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL)
                .expect("fresh selected sickness application");
            (
                application.clone(),
                auras.aura_cast_provenance_like_cpp(application.slot),
            )
        })
        .expect("canonical sickness Player");
    assert_eq!(application.caster_guid, caster_guid);
    assert_eq!(provenance.caster_level, Some(44));

    session.update(0).await;
    let wire_flags = drain_packets(&send_rx)
        .into_iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(&bytes))
        .filter(|update| update.spell_id == SICKNESS_SPELL)
        .map(|update| update.aura_flags)
        .last()
        .expect("selected sickness AuraUpdate after recovery");
    (application.aura_flags, provenance.caster_level, wire_flags)
}

#[tokio::test]
async fn selected_caster_reaction_uses_actual_foreign_group_and_duel() {
    let friendly = mixed_sickness_creation(43_841, true, true).await;
    assert_eq!(friendly, (0x0102, Some(44), 0x0106));

    let hostile = mixed_sickness_creation(43_851, false, false).await;
    assert_eq!(hostile, (0x0010, Some(44), 0x0014));
}
