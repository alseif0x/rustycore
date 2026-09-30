//! Real map actors and existing lifetime fixtures; never Clone a motor as setup.
use super::*;
use crate::manager::actor_tick_access::fixtures;
use crate::manager::{MapObjectUpdateSelectionLikeCpp, ObjectMapTickError};
use crate::map_manager::{
    AggroAiKind, AggroAiSelection, AggroAttackDecision, AggroEffectKind, AggroFactionTarget,
    WorldCreature,
};
use wow_constants::{UnitFlags, UnitState};
use wow_entities::{MapObjectRecord, PhaseShift, Player, UnitVisibilityDetectionStateLikeCpp};

mod equivalence;
mod primary;
mod rejection;
mod resolver;
mod staged;

fn settings() -> AggroSettings {
    AggroSettings {
        no_gray_aggro_above: 0,
        no_gray_aggro_below: 0,
        creature_aggro_rate: 1.0,
        max_player_level_config: 80,
        family_assistance_radius: 1_000.0,
        family_assistance_delay_ms: 100,
        map_is_dungeon: false,
        map_visibility_range: 90.0,
    }
}

fn policies<R>(apply: impl FnOnce(&mut AggroPolicies<'_>) -> R) -> R {
    let mut hostility = |_: i32, _: AggroFactionTarget<'_>| Some(true);
    let mut select_ai =
        |_: crate::map_manager::AggroAiFacts<'_>| AggroAiSelection::Selected(AggroAiKind::Base);
    let mut can_attack = |_| AggroAttackDecision::Allowed;
    let mut attack_distance = |_| 20.0;
    apply(&mut AggroPolicies {
        hostility: &mut hostility,
        select_ai: &mut select_ai,
        can_attack: &mut can_attack,
        attack_distance: &mut attack_distance,
    })
}

fn candidate(guid: ObjectGuid) -> AggroCandidate {
    AggroCandidate {
        player_guid: guid,
        map_id: 1,
        instance_id: 0,
        map_difficulty_id: 0,
        position: Position::xyz(11.0, 20.0, 30.0),
        player_visibility_represented: true,
        player_phase_shift: PhaseShift::default(),
        player_visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
        player_combat_reach: 1.5,
        player_detected_range_aura_mod: 0.0,
        player_liquid_status: 0,
        player_level: 25,
        player_gray_level: 0,
        player_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        player_unit_flags2: 0,
        player_unit_state: 0,
        player_is_game_master: false,
        player_is_contested_pvp: false,
        player_faction_template_id: 1,
        player_reputation_standings: Vec::new(),
        player_reputation_state_flags: Vec::new(),
        player_forced_reputation_ranks: Vec::new(),
        player_forced_reputation_faction_ids: Vec::new(),
        player_school_immunity_mask: 0,
        player_damage_immunity_mask: 0,
        player_has_confuse_aura: false,
        player_has_breakable_stun_aura: false,
    }
}

fn add_player(manager: &mut MapManager, counter: i64) -> ObjectGuid {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(candidate(guid).position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_level(25);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    manager.adopt_active_player_like_cpp(guid).unwrap();
    guid
}

fn actor_mut(manager: &mut MapManager, guid: ObjectGuid) -> &mut WorldCreature {
    manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .creature_actor_mut(guid)
        .unwrap()
}
fn actor(manager: &MapManager, guid: ObjectGuid) -> &WorldCreature {
    manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .creature_actor(guid)
        .unwrap()
}
fn configure(manager: &mut MapManager, guid: ObjectGuid) {
    let actor = actor_mut(manager, guid);
    actor.creature.set_faction(14);
    actor
        .creature
        .set_react_state(wow_entities::ReactState::Aggressive);
    actor.creature.unit_mut().set_level(25);
    actor.creature.ai_ownership_mut().aggro_radius = 20.0;
    actor.seed_runtime_rng_like_cpp(0x5757);
}
fn complete(progress: ActorAggroProgress) -> AggroOutcome {
    match progress {
        ActorAggroProgress::Complete(outcome) => outcome,
        _ => panic!("unexpected LOS"),
    }
}

fn pending_fixture(
    counter: i64,
) -> (
    MapManager,
    MapObjectTickContinuation,
    ObjectMapUpdateToken,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
    ActorAggroLosRequest,
) {
    let (mut manager, caller) = fixtures::manager_with_actor(counter);
    configure(&mut manager, caller);
    let assistant = fixtures::insert_actor(
        &mut manager,
        counter + 1,
        Position::xyz(500.0, 20.0, 30.0),
        false,
    );
    configure(&mut manager, assistant);
    actor_mut(&mut manager, assistant)
        .creature
        .unit_mut()
        .add_unit_state(UnitState::SIGHTLESS.bits());
    let victim = add_player(&mut manager, counter + 2);
    actor_mut(&mut manager, caller).enter_combat(victim);
    let (tick, mut token) = fixtures::start(
        &mut manager,
        77,
        MapObjectUpdateSelectionLikeCpp::NearbyCells,
    );
    assert!(
        !manager
            .selected_actor_guids(&tick, &mut token)
            .unwrap()
            .contains(&assistant)
    );
    let progress = policies(|policies| {
        manager.prepare_aggro(
            &tick,
            &mut token,
            vec![candidate(victim)],
            settings(),
            true,
            policies,
        )
    })
    .unwrap();
    let ActorAggroProgress::Pending(request) = progress else {
        panic!("assistance queries LOS")
    };
    (manager, tick, token, caller, assistant, victim, request)
}
