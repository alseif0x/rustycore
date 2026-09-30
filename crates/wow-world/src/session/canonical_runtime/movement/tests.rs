//! Actual packet adapters and canonical manager operations, never source mirrors.

use super::*;
use crate::map_manager::{RecipientRule, WorldCreature};
use crate::session::legacy_runtime::step_creature_movement_like_cpp;
use wow_core::{Position, guid::HighGuid};
use wow_entities::CreatureAiState;
use wow_packet::ServerPacket;

mod manager_cases;

fn guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 9999, counter)
}

fn actor(counter: i64) -> WorldCreature {
    let mut actor = WorldCreature::new(
        guid(counter), 9999, Position::new(10.0, 10.0, 0.0, 0.0),
        100, 2, 3, 5, 20.0, 100, 14, 0, 0,
    );
    actor.creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    actor.seed_runtime_rng_like_cpp(0x5757);
    actor
}

fn config() -> MMapRuntimeConfigLikeCpp {
    MMapRuntimeConfigLikeCpp { enabled: false, ..Default::default() }
}

fn trace(actor: &WorldCreature) -> MovementTrace {
    MovementTrace { entry: actor.entry(), map_id: actor.map_id(), state: actor.state() }
}

#[test]
fn home_launch_bytes_match_the_original_application_adapter() {
    let mut original = actor(610_001);
    let mut canonical = actor(610_001);
    for actor in [&mut original, &mut canonical] {
        actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
        actor.creature.set_ai_state(CreatureAiState::Returning);
    }
    let expected = step_creature_movement_like_cpp(
        &mut original, guid(610_001), &config(), None, None, None, 23,
    ).expect("original home movement launches");
    let movement = canonical.step_movement(
        23, None, None, |_, _| false,
        |_, _, _, _| panic!("disabled pathfinding must stay lazy"),
    );
    let mut plan = RuntimePlan { events: Vec::new() };
    assert!(append_completed_movement(
        &mut plan, canonical.guid(), 1, 0, canonical.position(), canonical.visibility_range_like_cpp(),
        movement, None, trace(&canonical),
    ));
    assert_eq!(plan.events.len(), 1);
    assert_eq!(plan.events[0].packet_bytes, expected);
    assert_eq!(canonical.runtime_elapsed_ms_like_cpp(), original.runtime_elapsed_ms_like_cpp());
}

#[test]
fn stop_bytes_match_the_original_application_adapter() {
    let victim_guid = ObjectGuid::create_player(1, 610_002);
    let target = ChaseTargetSnapshotLikeCpp {
        guid: victim_guid, position: Position::xyz(10.0, 10.0, 0.0),
        combat_reach: 1.5, in_world: true, in_water: None,
    };
    let mut original = actor(610_002);
    let mut canonical = actor(610_002);
    for actor in [&mut original, &mut canonical] {
        actor.enter_combat(victim_guid);
        actor.begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0)).unwrap();
    }
    let expected = step_creature_movement_like_cpp(
        &mut original, guid(610_002), &config(), None, None, Some(target), 1,
    ).expect("original chase stops its running spline in melee range");
    let movement = canonical.step_movement(
        1, Some(target), None, |_, _| false,
        |_, _, _, _| panic!("a stop must not query a path"),
    );
    assert!(matches!(&movement, Some(wow_map::CreatureMovementStep::Stop(_))));
    let mut plan = RuntimePlan { events: Vec::new() };
    assert!(append_completed_movement(
        &mut plan, canonical.guid(), 1, 0, canonical.position(), canonical.visibility_range_like_cpp(),
        movement, None, trace(&canonical),
    ));
    assert_eq!(plan.events[0].packet_bytes, expected);
}

#[test]
fn durable_home_health_precedes_movement_with_original_recipient_rules() {
    let mut actor = actor(610_003);
    actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    actor.creature.set_ai_state(CreatureAiState::Returning);
    actor.creature.unit_mut().set_health(75);
    let health = actor.creature.unit().values_update();
    let expected_health = crate::session::unit_values_update_to_update_object(actor.guid(), 1, &health)
        .expect("changed health has a VALUES update").to_bytes();
    let movement = actor.step_movement(23, None, None, |_, _| false, |_, _, _, _| None);
    let mut plan = RuntimePlan { events: Vec::new() };
    assert!(append_completed_movement(
        &mut plan, actor.guid(), 1, 0, actor.position(), actor.visibility_range_like_cpp(),
        movement, Some(health), trace(&actor),
    ));
    assert_eq!(plan.events.len(), 2);
    assert_eq!(plan.events[0].packet_bytes, expected_health);
    match (&plan.events[0].recipients, &plan.events[1].recipients) {
        (RecipientRule::NearbyVisibleDurable { source_guid, map_id, instance_id, source_position, range, required_3d },
         RecipientRule::NearbyVisible { source_guid: mover, map_id: move_map, instance_id: move_instance, source_position: move_position, range: move_range, required_3d: move_3d }) => {
            assert_eq!(*source_guid, actor.guid());
            assert_eq!(source_guid, mover);
            assert_eq!((*map_id, *instance_id), (1, 0));
            assert_eq!((map_id, instance_id), (move_map, move_instance));
            assert_eq!(*source_position, actor.position());
            assert_eq!(source_position, move_position);
            assert_eq!(*range, actor.visibility_range_like_cpp());
            assert_eq!(range, move_range);
            assert!(!*required_3d && !*move_3d);
        }
        other => panic!("expected durable health then visual movement, got {other:?}"),
    }
}
