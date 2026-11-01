//! #1263 F6-8D3b-3: chase and the first creature swing under the production
//! `RuntimeTickOwner::CanonicalMap` owner, end to end.
//!
//! The real canonical map loop runs the executor; a real `WorldSession` is the
//! victim and moves through its registered `CMSG_MOVE_HEARTBEAT` handler.
//!
//! * First swing: C++ `Unit::Unit` value-initializes `m_attackTimer`
//!   (`Unit.cpp:321`) and `isAttackReady` is `m_attackTimer == 0`, so a fresh
//!   creature swings on its first `DoMeleeAttackIfReady` in range.
//! * Chase: `ChaseMovementGenerator::Update` launches a spline toward a victim
//!   that left melee range (`SMSG_ON_MONSTER_MOVE` to the victim), the creature
//!   reaches it and swings again, and `Map::CreatureRelocation` keeps its grid
//!   cell on its position.

use super::*;

/// A `CMSG_MOVE_HEARTBEAT` body for `mover` at `position` (3.4.3
/// `MovementInfo`: no flags, no transport, no fall).
fn d3b3_heartbeat_like_cpp(
    mover: ObjectGuid,
    position: Position,
    time: u32,
) -> wow_packet::WorldPacket {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_packed_guid(&mover);
    packet.write_uint32(0);
    packet.write_uint32(0);
    packet.write_uint32(0);
    packet.write_uint32(time);
    packet.write_float(position.x);
    packet.write_float(position.y);
    packet.write_float(position.z);
    packet.write_float(position.orientation);
    packet.write_float(0.0);
    packet.write_float(0.0);
    packet.write_uint32(0);
    packet.write_uint32(0);
    // StandingOnGameObject, transport, fall, spline, height change, remote
    // time, inertia and advanced-flying bits: all clear.
    packet.write_uint8(0);
    packet.reset_read();
    packet
}

/// One driver step of the session: the committed commands and the kill phase,
/// then the map's Unit VALUES step, with every packet recorded.
async fn d3b3_session_step_like_cpp(
    world: &D3bWorldLikeCpp,
    session: &mut wow_world::session::WorldSession,
    send_rx: &flume::Receiver<Vec<u8>>,
    observed: &mut D3bObservedLikeCpp,
) {
    tokio::time::sleep(Duration::from_millis(u64::from(D3B_TICK_MS_LIKE_CPP))).await;
    let stage = observed.stage_like_cpp();
    let creature = world.creature;
    session
        .run_session_commands_and_kills_for_test_like_cpp(|command| {
            observed.record_like_cpp(creature, stage, command);
        })
        .await;
    let _ = d3b_drain_like_cpp(world, send_rx, observed, false);
    if session.send_map_unit_values_updates_for_test_like_cpp() > 0 {
        let _ = d3b_drain_like_cpp(world, send_rx, observed, true);
    }
}

fn d3b3_creature_cell_holds_it_like_cpp(world: &D3bWorldLikeCpp) -> bool {
    let guard = world.canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    let Some(position) = map
        .get_typed_creature(world.creature)
        .map(wow_entities::Creature::position)
    else {
        return false;
    };
    let cell = map.nearby_cell_guids_like_cpp(position.x, position.y, 0.0);
    cell.grid.creatures.contains(&world.creature) || cell.world.creatures.contains(&world.creature)
}

#[tokio::test]
async fn canonical_map_owner_swings_first_and_chases_a_player_out_of_range_like_cpp() {
    let world = d3b_world_like_cpp(96_400);
    // A creature that outlives the whole run.
    world
        .canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(world.creature, |creature| {
            creature.unit_mut().set_max_health(1_000_000);
            creature.unit_mut().set_health(1_000_000);
        })
        .unwrap();
    let player = ObjectGuid::create_player(1, 96_401);
    let (mut session, send_rx, _input_peer) = d3b_session_like_cpp(
        &world,
        player,
        "D3bChase",
        Position::new(11.0, 10.0, 0.0, std::f32::consts::PI),
    )
    .await;
    d3b_player_attacks_like_cpp(&world, player);
    let (canonical_handle, probe) = d3b_spawn_canonical_loop_like_cpp(&world);
    let mut observed = D3bObservedLikeCpp::default();

    // 1. Aggro and the first creature swing.
    let started = Instant::now();
    while observed.creature_melee == 0 && started.elapsed() < Duration::from_secs(5) {
        d3b3_session_step_like_cpp(&world, &mut session, &send_rx, &mut observed).await;
    }
    assert!(
        observed.attack_starts >= 1 && observed.creature_melee >= 1,
        "aggro and a creature swing reached the victim: {observed:?}"
    );
    let first_swing_ms =
        d3b_canonical_creature_like_cpp(&world, |creature| creature.ai_ownership().last_swing_ms)
            .unwrap();
    let base_attack_time =
        d3b_canonical_creature_like_cpp(&world, |creature| creature.base_attack_time_like_cpp())
            .unwrap();
    // Before F6-8D3b-3 the first swing needed `swing_timer_ms` (2000 ms) of
    // creature-local time; C++ swings as soon as the creature is in combat
    // and in range.
    assert!(
        first_swing_ms < 2_000,
        "the first swing does not wait a full attack time: {first_swing_ms} ms"
    );
    eprintln!(
        "F6-8D3b-3 first creature swing at creature-local {first_swing_ms} ms \
         (executor ticks {}, base attack time {base_attack_time} ms)",
        probe.executed_ticks.load(Ordering::Relaxed)
    );

    // 2. The victim steps out of melee range through its movement handler,
    //    across the cell boundary at x = 0 (cells are 66.6 yd wide), so the
    //    chase must also move the creature's cell.
    let away = Position::new(-8.0, 10.0, 0.0, 0.0);
    session
        .dispatch_client_packet_for_test_like_cpp(
            wow_constants::ClientOpcodes::MoveHeartbeat,
            d3b3_heartbeat_like_cpp(player, away, 1_000),
        )
        .await;
    let start_cell = d3b_canonical_creature_like_cpp(&world, |creature| {
        let position = creature.position();
        wow_map::Cell::from_world(position.x, position.y).cell_coord()
    })
    .unwrap();
    let player_position = world
        .canonical
        .lock()
        .unwrap()
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .map(|player| player.unit().world().position());
    assert_eq!(
        player_position,
        Some(away),
        "the heartbeat moved the player"
    );
    let melee_before = observed.creature_melee;
    let moves_before = observed.creature_monster_moves;

    // 3. Chase: the spline reaches the victim's client, the creature closes
    //    in, its cell follows it, and it swings again.
    let chase_started = Instant::now();
    let mut reached_ms = None;
    while (observed.creature_melee == melee_before || reached_ms.is_none())
        && chase_started.elapsed() < Duration::from_secs(8)
    {
        d3b3_session_step_like_cpp(&world, &mut session, &send_rx, &mut observed).await;
        let distance = d3b_canonical_creature_like_cpp(&world, |creature| {
            creature.position().distance_2d(&away)
        })
        .unwrap();
        if reached_ms.is_none() && distance <= 5.0 {
            reached_ms = Some(chase_started.elapsed().as_millis());
        }
    }
    canonical_handle.abort();
    let _ = canonical_handle.await;

    assert!(
        observed.creature_monster_moves > moves_before,
        "a chase SMSG_ON_MONSTER_MOVE for the creature reached the victim: {} before, {} after",
        moves_before,
        observed.creature_monster_moves
    );
    assert!(
        reached_ms.is_some(),
        "the creature reached the victim's new position"
    );
    assert!(
        observed.creature_melee > melee_before,
        "the creature swung again once back in range: {observed:?}"
    );
    let end_cell = d3b_canonical_creature_like_cpp(&world, |creature| {
        let position = creature.position();
        wow_map::Cell::from_world(position.x, position.y).cell_coord()
    })
    .unwrap();
    assert_ne!(start_cell, end_cell, "the chase crossed a cell boundary");
    assert!(
        d3b3_creature_cell_holds_it_like_cpp(&world),
        "the creature's grid cell follows its chase (Map::CreatureRelocation)"
    );
    eprintln!(
        "F6-8D3b-3 chase: {} ON_MONSTER_MOVE after the step, reached in {:?} ms, {} creature swings in total",
        observed.creature_monster_moves - moves_before,
        reached_ms,
        observed.creature_melee
    );
}
