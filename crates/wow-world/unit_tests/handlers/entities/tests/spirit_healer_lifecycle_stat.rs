use super::*;
use std::sync::Arc;
use wow_constants::DeathState;
use wow_core::Position;
use wow_data::character_progression::ChrClassesStore;
use wow_entities::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
};

fn install_stats_catalog(session: &mut WorldSession, strength: u16) {
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 11),
        wow_data::PlayerLevelStats {
            strength,
            agility: 10,
            stamina: 10,
            intellect: 10,
            spirit: 10,
            base_mana: 100,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([{
        let mut entry = wow_data::character_progression::ChrClassesEntry::default();
        entry.id = 1;
        entry
    }])));
}

fn install_unrelated_strength_aura(session: &mut WorldSession, spell_id: i32) {
    let mut spells = wow_data::SpellStore::new();
    spells.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -26,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
                effect_base_points: -26,
                effect_die_sides: 1,
                effect_misc_value_1: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spells));
}

#[tokio::test]
async fn lifecycle_stat_fresh_sickness_updates_canonical_stats_through_map_values() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_901).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, None);
    install_stats_catalog(&mut session, 10);

    session.core.with_owned_player_mut_like_cpp(|player| {
        player.unit_mut().set_death_state(DeathState::Alive);
    });
    assert!(session.send_stat_update());
    let legacy_packets = drain_packets(&send_rx);
    assert!(
        legacy_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::UpdateObject as u16)
    );
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.unit_mut().set_max_health(10_000);
        player.unit_mut().set_health(10_000);
        player.clear_data_changes();
    });
    let _ = drain_packets(&send_rx);

    session
        .player_aura_application_cx_like_cpp()
        .apply_aura_with_effect_mask_like_cpp(SICKNESS_SPELL, player_guid, 600_000, 0x15, 0x7)
        .expect("generic selected sickness application defers publication");
    wow_world_spell::hydrate_canonical_threat_relevant_auras_like_cpp(&mut session);
    let (sickness_slot, sickness_snapshot) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let slot = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL)
                .expect("generic sickness application is canonical")
                .slot;
            (slot, auras.threat_snapshot_like_cpp(slot).cloned())
        })
        .expect("canonical Player owner");
    let sickness_snapshot = sickness_snapshot.expect("selected sickness effect snapshot");
    assert_eq!(
        sickness_snapshot.effects(),
        &[
            (0b001, 80, -75, -1),
            (0b010, 79, -75, 127),
            (0b100, 159, 0, 127),
        ],
        "generic application and hydration preserve all selected sickness effects"
    );
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&sickness_slot))
            .expect("canonical Player owner")
    );

    let effective = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical effective stats after sickness");
    assert_eq!(effective.stats[wow_constants::Stats::Strength as usize], 2);
    assert_eq!(effective.mod_damage_done_percent, [0.25; 7]);
    let (health, max_health) = session
        .core
        .with_owned_player_like_cpp(|player| {
            (player.unit().data().health, player.unit().data().max_health)
        })
        .expect("canonical Player vitals after sickness");
    assert!(
        max_health < 10_000,
        "stamina sickness lowers maximum health"
    );
    assert_eq!(health, max_health, "SetMaxHealth clamps current health");
    let applied_packets = drain_packets(&send_rx);
    assert!(
        applied_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::AuraUpdate as u16),
        "selected sickness remains pending until the Unit update"
    );
    assert!(
        applied_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );
    session.update(0).await;
    let recovery_packets = drain_packets(&send_rx);
    let recovery_sickness = recovery_packets
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|aura| aura.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert_eq!(
        recovery_sickness.len(),
        1,
        "the actual Recovery consumer publishes the selected sickness once"
    );
    assert!(
        recovery_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );

    let manager = crate::session::hub_ref(&session)
        .core
        .canonical_map_manager
        .clone()
        .expect("canonical map manager");
    let applied_summary = manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .expect("current World map")
        .map_mut()
        .send_object_updates_like_cpp();
    assert_eq!(applied_summary.player_values_updates.len(), 1);
    let applied_values = &applied_summary.player_values_updates[0];
    assert_eq!(applied_values.guid, player_guid);
    let applied_derived = applied_values
        .values_update
        .derived_stats
        .as_ref()
        .expect("Map seals derived stats with Player values");
    assert_eq!(
        applied_derived.effective_combat_stats.stats[wow_constants::Stats::Strength as usize],
        2
    );
    assert!(
        applied_values
            .values_update
            .unit_data
            .as_ref()
            .is_some_and(|unit| unit.mask.is_set(6))
    );
    let applied_bytes =
        wow_world_core::entity_update_bridge::player_values_update_to_update_object(
            player_guid,
            571,
            &applied_values.values_update,
        )
        .expect("derived Player snapshot builds an UpdateObject")
        .to_bytes();
    assert_eq!(
        packet_opcode(&applied_bytes),
        ServerOpcodes::UpdateObject as u16
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .send_object_updates_like_cpp()
            .player_values_updates
            .is_empty()
    );

    let slot = visible_sickness_aura(&session, SICKNESS_SPELL)
        .first()
        .expect("active sickness application")
        .slot;
    session
        .player_aura_application_cx_like_cpp()
        .remove_aura_like_cpp(slot)
        .expect("sickness removal recalculates canonical stats");
    let removed_effective = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical effective stats after removal");
    assert_eq!(
        removed_effective.stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert_eq!(removed_effective.mod_damage_done_percent, [1.0; 7]);
    let removed_packets = drain_packets(&send_rx);
    assert!(
        removed_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
    );
    assert!(
        removed_packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );

    let removed_summary = manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .send_object_updates_like_cpp();
    assert_eq!(removed_summary.player_values_updates.len(), 1);
    let removed_values = &removed_summary.player_values_updates[0];
    assert_eq!(removed_values.guid, player_guid);
    assert_eq!(
        removed_values
            .values_update
            .derived_stats
            .as_ref()
            .unwrap()
            .effective_combat_stats
            .stats[wow_constants::Stats::Strength as usize],
        10
    );
    let removed_bytes =
        wow_world_core::entity_update_bridge::player_values_update_to_update_object(
            player_guid,
            571,
            &removed_values.values_update,
        )
        .expect("removed aura snapshot builds an UpdateObject")
        .to_bytes();
    assert_eq!(
        packet_opcode(&removed_bytes),
        ServerOpcodes::UpdateObject as u16
    );

    let unrelated_spell_id = 43_904;
    install_unrelated_strength_aura(&mut session, unrelated_spell_id);
    session
        .player_aura_application_cx_like_cpp()
        .apply_aura_with_effect_mask_like_cpp(unrelated_spell_id, player_guid, 60_000, 0x1, 0x1)
        .expect("unrelated Alive stat aura retains legacy application");
    let alive_apply_packets = drain_packets(&send_rx);
    assert!(
        alive_apply_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
    );
    assert!(
        alive_apply_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::UpdateObject as u16)
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .send_object_updates_like_cpp()
            .player_values_updates
            .is_empty()
    );

    let unrelated_slot = visible_sickness_aura(&session, unrelated_spell_id)
        .first()
        .expect("unrelated stat aura remains applied")
        .slot;
    session
        .player_aura_application_cx_like_cpp()
        .remove_aura_like_cpp(unrelated_slot)
        .expect("unrelated Alive stat aura retains legacy removal");
    let alive_remove_packets = drain_packets(&send_rx);
    assert!(
        alive_remove_packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::UpdateObject as u16)
    );
    assert!(
        manager
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .send_object_updates_like_cpp()
            .player_values_updates
            .is_empty()
    );

    for (death_state, strength) in [(DeathState::Dead, 12), (DeathState::Alive, 14)] {
        install_stats_catalog(&mut session, strength);
        session.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_death_state(death_state);
            player
                .unit_mut()
                .set_health(if death_state == DeathState::Dead {
                    1
                } else {
                    50
                });
            if death_state == DeathState::Alive {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = Some(PlayerResurrectionContinuation {
                    operation: PlayerResurrectionOperation::TeleportRevivePercent {
                        map_id: 571,
                        position: Position::default(),
                        restore_percent: 0.5,
                    },
                    phase: PlayerResurrectionPhase::ReadyToSavePlayer,
                });
            }
            player.clear_data_changes();
        });
        let deferred = session.send_stat_update();
        assert!(deferred);
        assert!(
            drain_packets(&send_rx).is_empty(),
            "dead or continuation-owned stat recomputation waits for Map VALUES"
        );

        let summary = manager
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .send_object_updates_like_cpp();
        assert_eq!(summary.player_values_updates.len(), 1);
        assert_eq!(
            summary.player_values_updates[0]
                .values_update
                .derived_stats
                .as_ref()
                .unwrap()
                .effective_combat_stats
                .stats[wow_constants::Stats::Strength as usize],
            i32::from(strength)
        );
    }
}

#[tokio::test]
async fn sickness_original_same_map_player_caster_level_is_captured() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_907).await;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(245_039));
    install_stats_catalog(&mut session, 10);
    let target_level = session
        .core
        .with_owned_player_like_cpp(|player| player.unit().data().level)
        .expect("canonical Player owner");
    let foreign_guid = ObjectGuid::create_player(1, 43_908);
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    {
        let mut manager = manager.lock().expect("canonical MapManager");
        let mut foreign = Box::new(wow_entities::Player::new(Some(43_908), false));
        foreign
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(foreign_guid);
        foreign.unit_mut().set_level(44);
        let handle = manager
            .install_detached_player_like_cpp(foreign)
            .expect("foreign Player owner");
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), Position::ZERO)
            .expect("same-map foreign Player");
    }

    let generators = session.id_generators_for_test_like_cpp();
    let spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        &mut session,
        generators.item.as_ref(),
        &spawns,
        SICKNESS_SPELL,
        player_guid,
        wow_entities::SpellCastMetadata {
            triggered_ignores_global_cooldown_like_cpp: true,
            original_caster_guid: Some(foreign_guid),
            ..Default::default()
        },
    )
    .await
    .expect("selected sickness accepts a resolved same-map original caster");

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
        .expect("canonical Player owner");
    assert_eq!(application.caster_guid, foreign_guid);
    assert_eq!(provenance.caster_level, Some(44));
    assert_ne!(provenance.caster_level, Some(target_level as u8));

    let spell_go = drain_packets(&send_rx)
        .into_iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("self-visible sickness cast emits SpellGo");
    let mut spell_go = wow_packet::WorldPacket::from_bytes(&spell_go);
    assert_eq!(
        spell_go.read_uint16().unwrap(),
        ServerOpcodes::SpellGo as u16
    );
    assert_eq!(spell_go.read_packed_guid().unwrap(), player_guid);
}

#[tokio::test]
async fn sickness_missing_planner_catalog_fails_before_aura_or_publication() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_902).await;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, None);
    session.catalogs.player_stats = None;
    session.core.with_owned_player_mut_like_cpp(|player| {
        player.unit_mut().set_death_state(DeathState::Alive);
        player.unit_mut().set_max_health(500);
        player.unit_mut().set_health(300);
        player.clear_data_changes();
    });
    let effective_before = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical effective stats before failed calculation");
    let pending_aura_updates_before = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .clone()
        })
        .expect("canonical Player aura owner");
    let _ = drain_packets(&send_rx);

    let result = session
        .player_aura_application_cx_like_cpp()
        .apply_aura_with_effect_mask_like_cpp(SICKNESS_SPELL, player_guid, 600_000, 0x15, 0x7);

    assert_eq!(
        result,
        Err("Unable to calculate stats for the race-selected sickness aura"),
        "missing PlayerStatsStore cannot silently claim an applied calculation"
    );
    assert_eq!(
        session.canonical_player_effective_combat_stats_like_cpp(),
        Some(effective_before),
        "failed planning leaves the canonical derived snapshot untouched"
    );
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .visible_auras_to_update
                    .clone()
            })
            .expect("canonical Player aura owner"),
        pending_aura_updates_before,
        "a failed sickness plan leaves pre-existing pending Aura updates unchanged"
    );
    let packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::AuraUpdate as u16)
    );
    assert!(
        packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );
}

#[tokio::test]
async fn sickness_replaces_foreign_or_mask_mismatched_application_before_fresh_cast() {
    for (guid, use_foreign_caster, old_mask) in [(43_905, true, 0b111), (43_906, false, 0b001)] {
        let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(guid).await;
        let player_guid = session.player_guid().expect("canonical Player GUID");
        session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
        set_race_sickness_spell(&mut session, SICKNESS_SPELL);
        install_sickness_data(&mut session, SICKNESS_SPELL, None);
        install_stats_catalog(&mut session, 10);
        let spell_info = session
            .catalogs
            .spell_store()
            .and_then(|store| store.get(SICKNESS_SPELL))
            .cloned()
            .expect("selected sickness SpellInfo");
        let old_caster = if use_foreign_caster {
            ObjectGuid::create_player(1, guid + 100)
        } else {
            player_guid
        };
        let old_provenance = wow_entities::AuraCastProvenanceLikeCpp {
            cast_id: ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::Cast,
                3,
                1,
                571,
                0,
                SICKNESS_SPELL as u32,
                guid + 200,
            ),
            spell_visual_id: 245_039,
            caster_level: None,
        };
        session
            .player_aura_application_cx_like_cpp()
            .apply_resurrection_sickness_aura(
                SICKNESS_SPELL,
                old_caster,
                &spell_info,
                900_000,
                0,
                0x88,
                0,
                0,
                old_provenance,
            )
            .expect("install old foreign or mask-mismatched selected Aura");
        let old = visible_sickness_aura(&session, SICKNESS_SPELL);
        assert_eq!(old.len(), 1);
        let old_slot = old[0].slot;
        if old_mask != 0b111 {
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    let auras = &mut player.unit_mut().subsystems_mut().auras;
                    let snapshot = auras
                        .threat_snapshot_like_cpp(old_slot)
                        .cloned()
                        .expect("selected sickness effect snapshot");
                    let effects = snapshot
                        .effects()
                        .iter()
                        .copied()
                        .filter(|(bit, _, _, _)| old_mask & *bit != 0)
                        .collect::<Vec<_>>();
                    for index in 0..3 {
                        let bit = 1_u32 << index;
                        if old_mask & bit == 0 {
                            let applied = wow_entities::AppliedAuraRef::new(
                                SICKNESS_SPELL as u32,
                                old_caster,
                                old_slot,
                                bit,
                            );
                            auras.remove_applied(applied);
                        }
                    }
                    auras.insert_threat_snapshot_like_cpp(
                        old_slot,
                        wow_entities::AuraThreatSnapshotLikeCpp::new(
                            snapshot.interrupt_flags(),
                            effects,
                        ),
                    );
                    let application = auras
                        .runtime_application_mut_like_cpp(old_slot)
                        .expect("selected sickness AuraApplication");
                    application.effect_mask = old_mask;
                    let mut metadata = auras
                        .aura_persistence_metadata(old_slot)
                        .cloned()
                        .expect("selected sickness persistence metadata");
                    metadata.recalculate_mask &= old_mask;
                    metadata
                        .base_amounts
                        .retain(|(index, _)| old_mask & (1_u32 << u32::from(*index)) != 0);
                    assert!(auras.set_aura_persistence_metadata(old_slot, metadata));
                })
                .expect("canonical partial sickness owner");
        }

        let new_provenance = wow_entities::AuraCastProvenanceLikeCpp {
            cast_id: ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::Cast,
                3,
                1,
                571,
                0,
                SICKNESS_SPELL as u32,
                guid + 300,
            ),
            spell_visual_id: 245_039,
            caster_level: None,
        };
        session
            .player_aura_application_cx_like_cpp()
            .apply_resurrection_sickness_aura(
                SICKNESS_SPELL,
                player_guid,
                &spell_info,
                600_000,
                0,
                0x88,
                0,
                0,
                new_provenance,
            )
            .expect("fresh selected application replaces the previous same-spell Aura");

        let current = visible_sickness_aura(&session, SICKNESS_SPELL);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].slot, old_slot, "removed Aura slot is reused");
        assert_eq!(current[0].caster_guid, player_guid);
        assert_eq!(current[0].effect_mask, 0b111);
        assert_eq!(
            session
                .core
                .with_owned_player_like_cpp(|player| {
                    player
                        .unit()
                        .subsystems()
                        .auras
                        .aura_cast_provenance_like_cpp(current[0].slot)
                })
                .expect("canonical new sickness provenance"),
            new_provenance
        );
        assert_ne!(new_provenance, old_provenance);
    }
}

fn forbidden_stats_calculator(
    _facts: &wow_world_core::session::PlayerStatFacts<'_>,
    _catalogs: &wow_world_core::session::PlayerStatCatalogs<'_>,
    _request: &wow_world_core::session::PlayerStatComputationRequest<'_>,
) -> Option<wow_world_core::session::PlayerStatComputation> {
    panic!("stats calculator must not run for a stale Player owner")
}

#[tokio::test]
async fn sickness_stale_owner_does_not_fall_back_or_mutate_retired_player() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_904).await;
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, None);
    install_stats_catalog(&mut session, 10);
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("selected sickness SpellInfo");

    let handle = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player owner handle");
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .expect("canonical map manager");
    let retired = manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .expect("retire canonical Player to make the retained handle stale");
    let effective_before = *retired.effective_combat_stats_like_cpp();
    let vitals_before = (
        retired.unit().data().health,
        retired.unit().data().max_health,
    );
    let aura_count_before = retired
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .len();

    let stats = session.core.player_stats_access_with_fixture_refs_like_cpp(
        &session.catalogs,
        &session.config,
        &session.fixtures.identity.player_race,
        &session.fixtures.identity.player_class,
        &session.fixtures.identity.player_level,
        wow_world_core::session::StatsFixtureRefs::new_like_cpp(
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut session.fixtures.combat.player_health_like_cpp,
                &mut session.fixtures.combat.player_max_health_like_cpp,
                &mut session.fixtures.combat.player_alive_like_cpp,
                &mut session.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut session
                    .fixtures
                    .combat
                    .represented_player_max_powers_like_cpp[0],
                &mut session
                    .fixtures
                    .combat
                    .represented_player_base_mana_like_cpp,
            ),
            wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                &session.fixtures.auras.represented_shapeshift_form_like_cpp,
                &session
                    .fixtures
                    .auras
                    .player_aura_authority_complete_like_cpp,
                &session
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &session.fixtures.auras.visible_auras,
                &session
                    .fixtures
                    .auras
                    .canonical_threat_aura_snapshots_like_cpp,
            ),
        ),
    );
    let result = stats.calculate_and_apply_stats_for_values(
        forbidden_stats_calculator,
        &wow_world_core::session::PlayerStatComputationRequest::new(None, None),
        wow_world_core::session::PlayerStatApplicationCause::AllStats,
    );

    assert!(result.is_none(), "a stale handle remains unresolved");
    let _ = drain_packets(&send_rx);
    let player_guid = session.player_guid().expect("Player GUID");
    let aura_result = session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            player_guid,
            &spell_info,
            600_000,
            0,
            0x88,
            0,
            0,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        );
    assert!(
        aura_result.is_err(),
        "a stale canonical handle cannot establish selected sickness ownership or fall back to fixtures"
    );
    assert_eq!(
        retired
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .len(),
        aura_count_before
    );
    assert!(
        drain_packets(&send_rx)
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::AuraUpdate as u16)
    );
    assert_eq!(
        *retired.effective_combat_stats_like_cpp(),
        effective_before,
        "the retired Player's canonical derived snapshot is untouched"
    );
    assert_eq!(
        (
            retired.unit().data().health,
            retired.unit().data().max_health
        ),
        vitals_before,
        "the retired Player's vitals are untouched"
    );
    assert_eq!(session.core.player_handle_like_cpp, Some(handle));
}

#[tokio::test]
async fn lifecycle_stat_total_stat_mask_without_low_bits_is_an_inert_aura_effect() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_903).await;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    let spell_id = 43_903;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_misc_value_2: 0x80,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let effective_before = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical effective stats before inert aura");

    session
        .player_aura_application_cx_like_cpp()
        .apply_aura_with_effect_mask_like_cpp(spell_id, player_guid, 60_000, 0x1, 0x1)
        .expect("miscB with no selected stat bits does not reject the aura");

    assert_eq!(
        session.canonical_player_effective_combat_stats_like_cpp(),
        Some(effective_before),
        "C++'s 0..MAX_STATS loop selects no stat for this nonzero mask"
    );
    assert_eq!(visible_sickness_aura(&session, spell_id).len(), 1);
    let packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .any(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
    );
    assert!(
        packets
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::UpdateObject as u16)
    );
}
