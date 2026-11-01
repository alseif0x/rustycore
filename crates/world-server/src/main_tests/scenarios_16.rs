//! Scenarios for [`super`], part 16.
//!
//! #1263 F6-8D3b-1/D3b-2: the canonical map creature runtime, end to end
//! (#1263 F6-8E: the only creature runtime).
//!
//! The integration regression boots the real canonical map update loop
//! (`spawn_canonical_map_update_loop`) with the canonical creature runtime on a
//! fixture with one hostile creature
//! and one **real** `WorldSession` in the world. The loop's production delivery
//! reaches the session's own mailbox, and the session runs the update steps the
//! driver runs for them (mailbox drain, then the creature-kill phase), so the
//! kill hooks and `setDeathState(JUST_DIED)` go through the canonical-first
//! mutation root, the loot is opened and released with the registered packet
//! handlers, and the respawn CREATE is built by the session from the canonical
//! map.

use std::sync::atomic::Ordering;

use super::*;
use crate::runtime::canonical_creature_runtime::{
    CanonicalCreatureRuntimeLikeCpp, CanonicalCreatureRuntimeProbeLikeCpp,
};

// #1263 F6-8D3b-3: chase and the first creature swing, end to end.
#[path = "scenarios_16/chase.rs"]
mod chase;
#[path = "scenarios_16/native_death.rs"]
mod native_death;
#[path = "scenarios_16/selected_recovery.rs"]
mod selected_recovery;
// #1444 H2: `world.update` on the booted loop, every tick guard released.
#[path = "scenarios_16/world_hook.rs"]
mod world_hook;

const D3B_TICK_MS_LIKE_CPP: u32 = 20;
const D3B_CREATURE_HP_LIKE_CPP: u32 = 20;
const D3B_PLAYER_HP_LIKE_CPP: u64 = 1_000;
const D3B_SPAWN_ID_LIKE_CPP: u64 = 96_301;
const D3B_CREATURE_GOLD_LIKE_CPP: u32 = 7;

fn d3b_aggro_config_like_cpp() -> wow_world::session::CreatureAggroConfigLikeCpp {
    let template = |id, faction, enemies| wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group: 0,
        friend_group: 0,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    };
    wow_world::session::CreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                template(14, 72, [930, 0, 0, 0, 0, 0, 0, 0]),
                template(1, 930, [0; 8]),
            ]),
        )),
        faction_store: Some(Arc::new(
            wow_data::progression_rewards::FactionStore::from_entries([
                wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
            ]),
        )),
        ..Default::default()
    }
}

fn d3b_canonical_runtime_like_cpp() -> CanonicalCreatureRuntimeLikeCpp {
    CanonicalCreatureRuntimeLikeCpp::new_like_cpp(
        wow_world::MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        },
        None,
        d3b_aggro_config_like_cpp(),
        None,
        None,
    )
}

/// One hostile persistent-spawn creature on the canonical map only, as the
/// production registration and loaded-grid paths now leave it, and the map
/// data the sessions and the loop share.
struct D3bWorldLikeCpp {
    canonical: wow_world::SharedCanonicalMapManager,
    registry: Arc<PlayerRegistry>,
    map_store: Arc<wow_data::MapStore>,
    creature: ObjectGuid,
    writer: RespawnDbWriterSenderLikeCpp,
}

fn d3b_world_like_cpp(base: i64) -> D3bWorldLikeCpp {
    let canonical: wow_world::SharedCanonicalMapManager =
        Arc::new(Mutex::new(wow_map::MapManager::default()));
    canonical.lock().unwrap().create_world_map(0, 0);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9001, base);
    let mut world_creature = wow_world::creature_projection::new_runtime_creature_like_cpp(
        creature,
        9001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        D3B_CREATURE_HP_LIKE_CPP,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    world_creature.set_spawn_id(D3B_SPAWN_ID_LIKE_CPP);
    // A money drop (C++ `Creature::GetLootMode` / `mingold..maxgold`), so the
    // corpse is lootable without an item template store.
    world_creature.ai_ownership_mut().gold_min = D3B_CREATURE_GOLD_LIKE_CPP;
    world_creature.ai_ownership_mut().gold_max = D3B_CREATURE_GOLD_LIKE_CPP;
    world_creature.set_corpse_delay(1, false);
    world_creature.set_respawn_delay(1);
    world_creature.ai_ownership_mut().respawn_time_secs = 1;
    wow_world::creature_projection::test_support::seed_runtime_rng_like_cpp(
        &mut world_creature,
        0xD3B2,
    );
    let mut canonical_creature = world_creature;
    canonical_creature
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    // `Map::AddToMap` puts the creature in its grid cell, so the `NearbyCells`
    // selection sees it, as production admission does.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_creature(canonical_creature).unwrap(),
        )
        .unwrap();
    let registry = Arc::new(PlayerRegistry::default());
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    D3bWorldLikeCpp {
        canonical,
        registry,
        map_store: Arc::new(runtime_world_map_store_like_cpp()),
        creature,
        writer: RespawnDbWriterSenderLikeCpp::new_like_cpp(),
    }
}

/// A real session entering the canonical world as the login path leaves it.
async fn d3b_session_like_cpp(
    world: &D3bWorldLikeCpp,
    player: ObjectGuid,
    name: &str,
    position: Position,
) -> (
    wow_world::session::WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Sender<wow_packet::WorldPacket>,
) {
    let (pkt_tx, pkt_rx) = flume::bounded(16);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = wow_world::session::WorldSession::new(
        1,
        name.to_string(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
        wow_world::session::registry::build_dispatch_table(),
    );
    assert!(
        session
            .enter_canonical_world_for_test_like_cpp(
                Arc::clone(&world.canonical),
                Arc::clone(&world.map_store),
                Arc::clone(&world.registry),
                player,
                name,
                position,
                80,
            )
            .await,
        "the canonical map admits the player"
    );
    (session, send_rx, pkt_tx)
}

/// The player swings at the creature: the attack state the session's
/// `CMSG_ATTACKSWING` leaves on the canonical player.
fn d3b_player_attacks_like_cpp(world: &D3bWorldLikeCpp, player: ObjectGuid) {
    let mut guard = world.canonical.lock().unwrap();
    let map = guard.find_map_mut(0, 0).unwrap().map_mut();
    let unit = map.get_typed_player_mut(player).unwrap().unit_mut();
    unit.set_faction(1);
    unit.set_max_health(D3B_PLAYER_HP_LIKE_CPP);
    unit.set_health(D3B_PLAYER_HP_LIKE_CPP);
    unit.set_death_state(wow_constants::DeathState::Alive);
    unit.subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
    unit.set_attacking(Some(world.creature));
    unit.set_target(world.creature);
    unit.add_unit_state(wow_constants::UnitState::MELEE_ATTACKING.bits());
    unit.set_base_attack_time_like_cpp(wow_constants::WeaponAttackType::BaseAttack, 300);
    unit.set_weapon_damage(wow_constants::WeaponAttackType::BaseAttack, 10.0, 10.0);
    unit.reset_attack_timer_like_cpp(wow_constants::WeaponAttackType::BaseAttack);
}

fn d3b_canonical_creature_like_cpp<R>(
    world: &D3bWorldLikeCpp,
    read: impl FnOnce(&wow_entities::Creature) -> R,
) -> Option<R> {
    world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_creature(world.creature)
        .map(read)
}

fn d3b_guid_packet_like_cpp(guid: ObjectGuid) -> wow_packet::WorldPacket {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.reset_read();
    packet
}

fn d3b_packed_guid_like_cpp(guid: ObjectGuid) -> Vec<u8> {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.data().to_vec()
}

fn d3b_server_opcodes_like_cpp(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    send_rx
        .try_iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect()
}

/// The creature's state on the canonical map when a packet about it is sent.
fn d3b_packet_stage_like_cpp(world: &D3bWorldLikeCpp, killed: bool) -> D3bStageLikeCpp {
    match (
        d3b_canonical_creature_like_cpp(world, wow_entities::Creature::is_alive),
        killed,
    ) {
        (None, _) => D3bStageLikeCpp::Despawned,
        (Some(false), _) => D3bStageLikeCpp::Corpse,
        (Some(true), true) => D3bStageLikeCpp::Respawned,
        (Some(true), false) => D3bStageLikeCpp::Combat,
    }
}

/// Drain everything the session sent so far, recording every
/// `SMSG_UPDATE_OBJECT` and `SMSG_ON_MONSTER_MOVE` that carries the creature's
/// GUID, whichever session step sent it (the login and respawn CREATE, the
/// DESTROY, the kill path's own VALUES and the map's VALUES step alike).
/// `values_step`: the drain right after the map's Unit VALUES step, whose
/// creature packets are also counted on their own.
fn d3b_drain_like_cpp(
    world: &D3bWorldLikeCpp,
    send_rx: &flume::Receiver<Vec<u8>>,
    observed: &mut D3bObservedLikeCpp,
    values_step: bool,
) -> Vec<ServerOpcodes> {
    let stage = d3b_packet_stage_like_cpp(world, observed.kills > 0);
    let packed = d3b_packed_guid_like_cpp(world.creature);
    let mut opcodes = Vec::new();
    for bytes in send_rx.try_iter() {
        let opcode = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode();
        let about_creature = bytes.windows(packed.len()).any(|window| window == packed);
        match opcode {
            Some(ServerOpcodes::AttackStart) => observed.attack_starts += 1,
            Some(ServerOpcodes::AttackerStateUpdate) => {
                let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
                packet.skip_opcode();
                let _log_data = packet.read_uint8().unwrap();
                let _round_size = packet.read_uint32().unwrap();
                let _hit_info = packet.read_uint32().unwrap();
                if packet.read_packed_guid().unwrap() == world.creature {
                    observed.creature_melee += 1;
                }
            }
            Some(ServerOpcodes::UpdateObject) if about_creature => {
                *observed.creature_update_objects.entry(stage).or_default() += 1;
                if values_step {
                    *observed.creature_values_packets.entry(stage).or_default() += 1;
                }
                observed
                    .creature_update_object_payloads
                    .push((values_step, bytes.clone()));
            }
            Some(ServerOpcodes::OnMonsterMove) if about_creature => {
                observed.creature_monster_moves += 1;
            }
            _ => {}
        }
        opcodes.extend(opcode);
    }
    opcodes
}

/// The lifecycle stage the run is in, for the per-stage VALUES counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum D3bStageLikeCpp {
    Combat,
    Corpse,
    Despawned,
    Respawned,
}

/// What reached the session and the writer over one run.
#[derive(Debug, Default)]
struct D3bObservedLikeCpp {
    attack_starts: usize,
    creature_melee: usize,
    player_melee_results: usize,
    kills: usize,
    refreshes: usize,
    /// `SendVisibleObjectValuesUpdate` commands for the creature the session
    /// received, per lifecycle stage.
    creature_values_updates: BTreeMap<D3bStageLikeCpp, usize>,
    /// Creature VALUES `SMSG_UPDATE_OBJECT` packets the session sent from the
    /// map's Unit VALUES step, per lifecycle stage.
    creature_values_packets: BTreeMap<D3bStageLikeCpp, usize>,
    /// Every `SMSG_UPDATE_OBJECT` carrying the creature's GUID the session
    /// sent over the whole run, per lifecycle stage, and their bytes in order.
    creature_update_objects: BTreeMap<D3bStageLikeCpp, usize>,
    /// `(from the map's VALUES step, bytes)` of each of them, in order.
    creature_update_object_payloads: Vec<(bool, Vec<u8>)>,
    /// `SMSG_ON_MONSTER_MOVE` packets for the creature the session sent.
    creature_monster_moves: usize,
    destroy_delivered: bool,
    update_after_despawn: bool,
    db_mutations: Vec<RespawnPersistenceMutationLikeCpp>,
    just_died: bool,
    loot_response: bool,
    loot_released: bool,
    despawned: bool,
    respawned: bool,
    respawn_create_delivered: bool,
}

impl D3bObservedLikeCpp {
    fn record_like_cpp(
        &mut self,
        creature: ObjectGuid,
        stage: D3bStageLikeCpp,
        command: &SessionCommand,
    ) {
        match command {
            SessionCommand::ApplyPlayerMeleeResultLikeCpp(result) => {
                self.player_melee_results += 1;
                self.kills += usize::from(result.killed_creature.is_some());
            }
            SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(_) => self.refreshes += 1,
            SessionCommand::SendVisibleObjectValuesUpdate(update)
                if update.object_guid == creature =>
            {
                *self.creature_values_updates.entry(stage).or_default() += 1;
            }
            _ => {}
        }
    }

    fn stage_like_cpp(&self) -> D3bStageLikeCpp {
        if self.respawned {
            D3bStageLikeCpp::Respawned
        } else if self.despawned {
            D3bStageLikeCpp::Despawned
        } else if self.kills > 0 {
            D3bStageLikeCpp::Corpse
        } else {
            D3bStageLikeCpp::Combat
        }
    }
}

/// Boot the production canonical map update loop with the canonical creature
/// owner over `world`.
fn d3b_spawn_canonical_loop_like_cpp(
    world: &D3bWorldLikeCpp,
) -> (
    tokio::task::JoinHandle<()>,
    Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
) {
    d3b_spawn_canonical_loop_with_like_cpp(
        world,
        &D3bLoopGuardsLikeCpp::default(),
        crate::module_hooks::WorldModuleHooksLikeCpp::new(Arc::new(
            wow_module_api::ModuleRegistry::new(),
        )),
    )
}

/// The loop's own guards, shared so a test can probe them (#1444 H2).
#[derive(Clone)]
struct D3bLoopGuardsLikeCpp {
    spawn_metadata: Arc<Mutex<spawn_store_loader::CanonicalSpawnMetadataLikeCpp>>,
    respawn_db_mutation_order: Arc<Mutex<()>>,
    world_state_mgr: crate::SharedWorldStateMgrLikeCpp,
}

impl Default for D3bLoopGuardsLikeCpp {
    fn default() -> Self {
        Self {
            spawn_metadata: Arc::new(Mutex::new(
                spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
                    SpawnStore::new(),
                    BTreeMap::new(),
                ),
            )),
            respawn_db_mutation_order: Arc::new(Mutex::new(())),
            world_state_mgr: Arc::new(Mutex::new(Default::default())),
        }
    }
}

fn d3b_spawn_canonical_loop_with_like_cpp(
    world: &D3bWorldLikeCpp,
    guards: &D3bLoopGuardsLikeCpp,
    world_module_hooks: crate::module_hooks::WorldModuleHooksLikeCpp,
) -> (
    tokio::task::JoinHandle<()>,
    Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
) {
    let runtime = d3b_canonical_runtime_like_cpp();
    let probe = Arc::clone(&runtime.probe);
    let canonical_handle = crate::runtime::spawn_canonical_map_update_loop(
        Arc::clone(&world.canonical),
        D3B_TICK_MS_LIKE_CPP,
        60_000,
        Arc::clone(&guards.spawn_metadata),
        Arc::new(wow_data::ConditionEntriesByTypeStore::default()),
        Arc::clone(&world.map_store),
        Arc::new(FakeGameEventPersistencePortLikeCpp::default()),
        world.writer.clone(),
        Arc::clone(&guards.respawn_db_mutation_order),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        empty_loaded_grid_creature_respawn_caches_like_cpp(),
        Arc::new(wow_data::AreaTriggerTemplateStore::default()),
        CanonicalGameEventSchedulerLikeCpp::start_system(3_600_000),
        Arc::clone(&world.registry),
        Arc::new(ActiveWorldSessionRegistryLikeCpp::new()),
        Arc::new(wow_data::BattlemasterListStore::from_entries([])),
        Arc::clone(&guards.world_state_mgr),
        Some(runtime),
        world_module_hooks,
    );
    (canonical_handle, probe)
}

/// Boot the canonical map loop with the canonical creature owner and drive the
/// real session until the creature has died, been looted, despawned,
/// respawned and been re-created at the client, or `deadline` passes.
async fn d3b_run_like_cpp(
    world: &D3bWorldLikeCpp,
    session: &mut wow_world::session::WorldSession,
    send_rx: &flume::Receiver<Vec<u8>>,
    deadline: Duration,
    mut on_first_creature_melee: impl FnMut(
        &D3bWorldLikeCpp,
        &wow_map::MapManager,
        &CanonicalCreatureRuntimeProbeLikeCpp,
    ),
) -> (
    D3bObservedLikeCpp,
    Arc<CanonicalCreatureRuntimeProbeLikeCpp>,
) {
    let (canonical_handle, probe) = d3b_spawn_canonical_loop_like_cpp(world);

    let mut observed = D3bObservedLikeCpp::default();
    let mut seen_creature_melee = false;
    let started = Instant::now();
    while started.elapsed() < deadline && !observed.respawn_create_delivered {
        tokio::time::sleep(Duration::from_millis(u64::from(D3B_TICK_MS_LIKE_CPP))).await;
        // The session update the driver runs for delivered commands: the
        // mailbox, then the creature-kill phase.
        let stage = observed.stage_like_cpp();
        let creature = world.creature;
        session
            .run_session_commands_and_kills_for_test_like_cpp(|command| {
                observed.record_like_cpp(creature, stage, command);
            })
            .await;
        let mut opcodes = d3b_drain_like_cpp(world, send_rx, &mut observed, false);
        // The driver's logged-in step that publishes the map's Unit VALUES.
        // The stage is the creature's state on the canonical map when sent.
        if session.send_map_unit_values_updates_for_test_like_cpp() > 0 {
            opcodes.extend(d3b_drain_like_cpp(world, send_rx, &mut observed, true));
        }
        if observed.creature_melee > 0 && !seen_creature_melee {
            seen_creature_melee = true;
            while started.elapsed() < deadline {
                let tick_completed = {
                    let manager = world.canonical.lock().unwrap();
                    if manager.tick_coordination_like_cpp()
                        == wow_map::MapTickCoordinationStateLikeCpp::Idle
                    {
                        on_first_creature_melee(world, &manager, &probe);
                        true
                    } else {
                        false
                    }
                };
                if tick_completed {
                    break;
                }

                // The producer may still be waiting for another Session phase
                // reply. Drain it and its resulting packets before yielding so
                // the admitted Map tick can reach its completed boundary.
                let stage = observed.stage_like_cpp();
                let creature = world.creature;
                session
                    .run_session_commands_and_kills_for_test_like_cpp(|command| {
                        observed.record_like_cpp(creature, stage, command);
                    })
                    .await;
                opcodes.extend(d3b_drain_like_cpp(world, send_rx, &mut observed, false));
                if session.send_map_unit_values_updates_for_test_like_cpp() > 0 {
                    opcodes.extend(d3b_drain_like_cpp(world, send_rx, &mut observed, true));
                }
                tokio::task::yield_now().await;
            }
        }
        let alive = d3b_canonical_creature_like_cpp(world, wow_entities::Creature::is_alive);
        if observed.kills > 0 && !observed.just_died && alive == Some(false) {
            // `setDeathState(JUST_DIED)` ran in the session's kill phase,
            // through the canonical-first mutation root.
            observed.just_died = true;
            session
                .dispatch_client_packet_for_test_like_cpp(
                    wow_constants::ClientOpcodes::LootUnit,
                    d3b_guid_packet_like_cpp(world.creature),
                )
                .await;
            observed.loot_response = d3b_drain_like_cpp(world, send_rx, &mut observed, false)
                .contains(&ServerOpcodes::LootResponse);
            session
                .dispatch_client_packet_for_test_like_cpp(
                    wow_constants::ClientOpcodes::LootRelease,
                    d3b_guid_packet_like_cpp(world.creature),
                )
                .await;
            observed.loot_released = d3b_drain_like_cpp(world, send_rx, &mut observed, false)
                .contains(&ServerOpcodes::LootRelease);
        }
        if observed.just_died && alive.is_none() {
            observed.despawned = true;
        }
        // The test is the writer: it takes every due statement in order.
        loop {
            let attempt = world
                .writer
                .mailbox
                .state
                .lock()
                .unwrap()
                .queue
                .take_due(Instant::now() + Duration::from_secs(60));
            let Some(attempt) = attempt else {
                break;
            };
            observed.db_mutations.push(attempt.pending.mutation);
        }
        if observed.despawned && !observed.respawned && alive == Some(true) {
            observed.respawned = observed.db_mutations.iter().any(|mutation| {
                matches!(mutation, RespawnPersistenceMutationLikeCpp::Delete { .. })
            });
        }
        // The despawn DESTROY removed the creature from the client; only the
        // respawn CREATE puts it back.
        if observed.respawned
            && (opcodes.contains(&ServerOpcodes::UpdateObject) || observed.update_after_despawn)
            && session.client_sees_guid_for_test_like_cpp(world.creature)
        {
            observed.respawn_create_delivered = true;
        }
        if observed.despawned && !session.client_sees_guid_for_test_like_cpp(world.creature) {
            observed.destroy_delivered = true;
        }
        if observed.destroy_delivered && opcodes.contains(&ServerOpcodes::UpdateObject) {
            observed.update_after_despawn = true;
        }
    }
    canonical_handle.abort();
    let _ = canonical_handle.await;
    (observed, probe)
}

#[tokio::test]
async fn canonical_map_owner_runs_the_session_kill_path_end_to_end_like_cpp() {
    let world = d3b_world_like_cpp(96_300);
    let player = ObjectGuid::create_player(1, 96_301);
    let player_position = Position::new(11.0, 10.0, 0.0, std::f32::consts::PI);
    let (mut session, send_rx, _input_peer) =
        d3b_session_like_cpp(&world, player, "D3bPlayer", player_position).await;
    assert!(
        session.client_sees_guid_for_test_like_cpp(world.creature),
        "the login visibility pass created the canonical creature"
    );
    d3b_player_attacks_like_cpp(&world, player);
    // The login CREATE stays queued: the run counts it with everything else.

    let mut clock_check = None;
    let (observed, probe) = d3b_run_like_cpp(
        &world,
        &mut session,
        &send_rx,
        Duration::from_secs(10),
        |world, manager, probe| {
            // The helper invokes this only after the producer has completed its
            // Map tick, holding the manager guard across both clock reads.
            let creature_clock = manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_creature(world.creature)
                .map(|creature| creature.runtime_elapsed_ms_like_cpp());
            clock_check = Some((
                creature_clock,
                probe.clock_advanced_ms.load(Ordering::Relaxed),
            ));
        },
    )
    .await;

    assert!(
        observed.attack_starts >= 1,
        "aggro reached the victim: {observed:?}"
    );
    assert!(
        observed.creature_melee >= 1,
        "creature melee reached the victim: {observed:?}"
    );
    assert!(
        observed.kills >= 1,
        "the player's melee killed the creature: {observed:?}"
    );
    assert!(
        observed.just_died,
        "the session's kill phase completed JUST_DIED on the canonical incarnation: {observed:?}"
    );
    assert!(
        observed.loot_response,
        "the killer sees the corpse's loot: {observed:?}"
    );
    assert!(
        observed.loot_released,
        "the loot release is answered: {observed:?}"
    );
    assert!(observed.despawned, "the corpse despawned: {observed:?}");
    assert!(observed.respawned, "the creature respawned: {observed:?}");
    assert!(
        observed.respawn_create_delivered,
        "the respawn CREATE reached the session: {observed:?}"
    );
    assert!(
        observed.refreshes >= 2,
        "despawn and respawn refreshed the map: {observed:?}"
    );
    // JUST_DIED saves the respawn time; the respawn deletes it. The corpse
    // despawn re-saves only when its wall-clock conversion of the same
    // respawn instant lands one second later (the lifecycle body's existing
    // upsert, masked to one second in the F6-8D3a-3 parity test), and the
    // shared writer coalesces a re-save it has not handed out yet.
    let (saves, rest) = observed
        .db_mutations
        .split_at(observed.db_mutations.len().saturating_sub(1));
    assert!(
        matches!(
            rest,
            [RespawnPersistenceMutationLikeCpp::Delete { key }] if key.spawn_id == D3B_SPAWN_ID_LIKE_CPP
        ),
        "the respawn delete is last: {:?}",
        observed.db_mutations
    );
    let save_times: Vec<i64> = saves
        .iter()
        .map(|mutation| match mutation {
            RespawnPersistenceMutationLikeCpp::Save { key, respawn_time }
                if key.spawn_id == D3B_SPAWN_ID_LIKE_CPP =>
            {
                *respawn_time
            }
            other => panic!("only the spawn's REP_RESPAWN precedes the delete: {other:?}"),
        })
        .collect();
    assert!(
        (1..=2).contains(&save_times.len())
            && save_times.iter().max().unwrap() - save_times.iter().min().unwrap() <= 1,
        "the JUST_DIED save, re-saved at most once one second later: {:?}",
        observed.db_mutations
    );

    let executed = probe.executed_ticks.load(Ordering::Relaxed);
    assert!(
        executed >= 10,
        "the executor ran every admitted tick: {executed}"
    );
    // One creature tick owner: the creature's own clock is exactly the
    // executor's advancement (the session runs no creature tick of its own).
    let (creature_clock, probe_clock) = clock_check.expect("creature melee was observed");
    assert_eq!(creature_clock, Some(probe_clock));

    // R6 UPDATE_OBJECT flood (Rust 1900 vs C++ 11 VALUES in the paired run):
    // count the creature VALUES packets this client actually received. They
    // are published on change only — once per map update that changed the
    // creature — so a handful across the fight, at most the death and
    // loot-flag changes while the corpse lies untouched, none while despawned.
    // The generic VALUES rail never carries Unit snapshots.
    let packets = &observed.creature_values_packets;
    let total: usize = packets.values().sum();
    assert!(total >= 1, "the fight changed the creature: {packets:?}");
    assert!(
        total * 4 <= usize::try_from(executed).unwrap(),
        "VALUES are not republished every tick: {total} over {executed} ticks ({packets:?})"
    );
    assert!(
        packets.get(&D3bStageLikeCpp::Corpse).copied().unwrap_or(0) <= 3,
        "an untouched corpse republishes nothing: {packets:?}"
    );
    assert_eq!(
        packets
            .get(&D3bStageLikeCpp::Despawned)
            .copied()
            .unwrap_or(0),
        0,
        "a despawned creature has no VALUES: {packets:?}"
    );
    assert!(
        observed.creature_values_updates.is_empty(),
        "no Unit snapshot travels the generic VALUES rail: {observed:?}"
    );
    eprintln!(
        "F6-8D3b-2 VALUES: {total} creature VALUES packets over {executed} executor ticks: {packets:?}"
    );

    // #1263 F6-8D3b-3: every `SMSG_UPDATE_OBJECT` with the creature's GUID the
    // session delivered over the whole lifecycle — the login CREATE, the kill
    // path's own VALUES and the loot step's, the despawn DESTROY and the
    // respawn CREATE, not only the map's VALUES step — stays bounded, and no
    // packet repeats the previous one's bytes (no republish of unchanged
    // values).
    let all = &observed.creature_update_objects;
    let all_total: usize = all.values().sum();
    assert!(
        all_total >= total + 2,
        "the login and respawn CREATE are counted too: {all:?} vs VALUES {packets:?}"
    );
    assert!(
        all_total * 4 <= usize::try_from(executed).unwrap(),
        "creature UPDATE_OBJECT bounded over the lifecycle: {all_total} over {executed} ticks ({all:?})"
    );
    assert!(
        all.get(&D3bStageLikeCpp::Despawned).copied().unwrap_or(0) <= 1,
        "a despawned creature gets at most its DESTROY: {all:?}"
    );
    assert!(
        all.get(&D3bStageLikeCpp::Corpse).copied().unwrap_or(0) <= 4,
        "an untouched corpse republishes nothing: {all:?}"
    );
    // The map's VALUES step never sends a client the creature values it has
    // just received from another rail (the melee result, the kill path, the
    // loot-release fanout). A forced resend C++ also makes
    // (`ForceUpdateFieldChange(DynamicFlags)` on loot release) is its own rail.
    let payloads = &observed.creature_update_object_payloads;
    let republished = payloads
        .windows(2)
        .filter(|pair| pair[1].0 && pair[0].1 == pair[1].1)
        .count();
    assert_eq!(
        republished, 0,
        "the map's VALUES step republished unchanged creature values"
    );
    eprintln!(
        "F6-8D3b-3 UPDATE_OBJECT: {all_total} creature UPDATE_OBJECT packets over {executed} executor ticks: {all:?}"
    );

    // Login near the respawned creature: a fresh session entering the map
    // gets the creature's CREATE from the canonical map.
    let (newcomer, newcomer_rx, _newcomer_input_peer) = d3b_session_like_cpp(
        &world,
        ObjectGuid::create_player(1, 96_302),
        "D3bNewcomer",
        Position::new(12.0, 10.0, 0.0, 0.0),
    )
    .await;
    assert!(newcomer.client_sees_guid_for_test_like_cpp(world.creature));
    assert!(d3b_server_opcodes_like_cpp(&newcomer_rx).contains(&ServerOpcodes::UpdateObject));
    assert_eq!(
        d3b_canonical_creature_like_cpp(&world, wow_entities::Creature::is_alive),
        Some(true),
        "the respawned incarnation is alive"
    );
}

#[test]
fn respawn_entry_carries_the_template_speed_rates_and_scales_like_cpp() {
    // C++ `Creature::InitEntry` (`Creature.cpp:547-553`) re-applies the
    // template walk/run rates and the native object scale on every respawn.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 9003, 96_360);
    let mut world_creature = wow_world::creature_projection::new_runtime_creature_like_cpp(
        guid,
        9003,
        Position::new(10.0, 10.0, 0.0, 0.0),
        D3B_CREATURE_HP_LIKE_CPP,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    );
    let mut projection =
        wow_entities::creature_create::CreatureRespawnCreateProjectionLikeCpp::from_create_data_like_cpp(
            &wow_world::creature_projection::creature_create_data_like_cpp(&world_creature),
        );
    projection.speed_walk_rate = 0.8;
    projection.speed_run_rate = 1.5;
    projection.display_scale = 1.25;
    projection.native_x_display_scale = 1.1;
    projection.scale = 2.0;
    world_creature
        .runtime_like_cpp_mut()
        .set_respawn_create_projection_like_cpp(projection);

    let pending = wow_world::creature_respawn::pending_respawn_from_creature_like_cpp(
        &world_creature,
        Instant::now(),
        0,
    );
    assert_eq!(pending.create_data.speed_walk_rate, 0.8);
    assert_eq!(pending.create_data.speed_run_rate, 1.5);
    assert_eq!(pending.create_data.display_scale, 1.25);
    assert_eq!(pending.create_data.native_x_display_scale, 1.1);
    assert_eq!(pending.create_data.scale, 2.0);
    let rebuilt = wow_world::creature_respawn::creature_from_pending_respawn_like_cpp(&pending, 0);
    assert_eq!(rebuilt.unit().world().object().scale(), 2.0);
    let rates = rebuilt.unit().speed_rate();
    assert_eq!(rates[wow_constants::UnitMoveType::Run as usize], 1.5);
}

/// #1263 F6-8E: the session-owned creature lifecycle and combat ticks are
/// deleted, not only unreachable: the session driver runs no creature tick.
#[test]
fn the_session_runs_no_creature_tick_like_cpp() {
    let driver = include_str!("../../../wow-world/src/session/driver/mod.rs");
    for retired in [
        "tick_creatures_sync",
        "tick_combat_sync",
        "run_creatures_tick",
        "RuntimeTickOwner",
    ] {
        assert!(!driver.contains(retired), "`{retired}` must stay retired");
    }
}
