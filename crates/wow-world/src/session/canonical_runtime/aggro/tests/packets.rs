//! Aura/stop/start and Alert keep the original bytes, keys and recipient rules.
use super::*;

fn effect(kind: AggroEffectKind) -> AggroEffect {
    AggroEffect { source_guid: guid(814_001), map_id: 609, instance_id: 42,
        position: Position::xyz(10.0, 20.0, 30.0), visibility_range: 90.0, kind }
}

#[test]
fn ordered_aura_move_stop_start_bytes_and_recipient_durability() {
    let victim = ObjectGuid::create_player(1, 814_002);
    let stop = wow_movement::MoveSplineStopResult {
        position: Position::xyz(10.0, 20.0, 30.0), spline_id: 17, stop_distance_tolerance: 0,
    };
    let mut result = LegacyCreatureAggroTickOutcomeLikeCpp::default();
    super::super::packets::append_outcome(&mut result, AggroOutcome {
        effects: vec![effect(AggroEffectKind::RemoveAuras(vec![3, 7])),
            effect(AggroEffectKind::MoveStop(stop)), effect(AggroEffectKind::AttackStart { victim })],
        ..Default::default()
    });
    let expected = [
        wow_packet::packets::misc::AuraUpdate { unit_guid: guid(814_001), update_all: false,
            auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp { slot: 3, aura_data: None },
                wow_packet::packets::misc::AuraInfoLikeCpp { slot: 7, aura_data: None }] }.to_bytes(),
        wow_packet::packets::movement::MonsterMoveStop { mover_guid: guid(814_001),
            current_pos: stop.position, spline_id: stop.spline_id }.to_bytes(),
        wow_packet::packets::combat::AttackStart { attacker: guid(814_001), victim }.to_bytes(),
    ];
    assert_eq!(result.plan.events.len(), expected.len());
    for (event, bytes) in result.plan.events.iter().zip(expected) { assert_eq!(event.packet_bytes, bytes); }
    for index in [0, 2] {
        assert!(matches!(&result.plan.events[index].recipients, RecipientRule::NearbyVisibleDurable {
            map_id: 609, instance_id: 42, required_3d: false, .. }));
    }
    assert!(matches!(&result.plan.events[1].recipients, RecipientRule::NearbyVisible {
        map_id: 609, instance_id: 42, required_3d: false, .. }));
}

#[test]
fn alert_only_preserves_map_keys_without_a_combat_command() {
    let mut plan = RuntimePlan::default();
    super::super::packets::append_effect(&mut plan, &effect(AggroEffectKind::Alert));
    assert_eq!(plan.events[0].packet_bytes, wow_packet::packets::combat::AIReaction {
        unit_guid: guid(814_001), reaction: wow_constants::creature::AiReaction::Alert,
    }.to_bytes());
    assert!(matches!(&plan.events[0].recipients, RecipientRule::MapBroadcastVisible { map_id: 609, instance_id: 42 }));
}

#[test]
fn attack_stop_retains_now_dead_false_and_durable_recipient() {
    let victim = ObjectGuid::create_player(1, 814_002);
    let mut plan = RuntimePlan::default();
    super::super::packets::append_effect(&mut plan, &effect(AggroEffectKind::AttackStop { victim }));
    assert_eq!(plan.events[0].packet_bytes, wow_packet::packets::combat::SAttackStop {
        attacker: guid(814_001), victim, now_dead: false,
    }.to_bytes());
    assert!(matches!(&plan.events[0].recipients, RecipientRule::NearbyVisibleDurable { .. }));
}
