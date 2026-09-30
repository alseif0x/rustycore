//! Packet bytes/recipient rules stay in the application, outside map guards.
use super::*;
use wow_packet::ServerPacket;
use crate::map_manager::{RecipientRule, RuntimeEvent};

pub(in crate::session) fn append_outcome(target: &mut LegacyCreatureAggroTickOutcomeLikeCpp, outcome: AggroOutcome) {
    target.maps_seen += outcome.maps_seen;
    target.creatures_seen += outcome.creatures_seen;
    target.sightless_creatures_skipped += outcome.sightless_creatures_skipped;
    target.candidates_seen += outcome.candidates_seen;
    target.targetability_rejections += outcome.targetability_rejections;
    target.visibility_unrepresented += outcome.visibility_unrepresented;
    target.visibility_rejections += outcome.visibility_rejections;
    target.hostility_rejections += outcome.hostility_rejections;
    target.hostility_unrepresented += outcome.hostility_unrepresented;
    target.accessibility_rejections += outcome.accessibility_rejections;
    target.owner_position_unrepresented += outcome.owner_position_unrepresented;
    target.attacker_evade_rejections += outcome.attacker_evade_rejections;
    target.home_range_rejections += outcome.home_range_rejections;
    target.gray_aggro_rejections += outcome.gray_aggro_rejections;
    target.ai_selection_unrepresented += outcome.ai_selection_unrepresented;
    target.ai_los_suppressed += outcome.ai_los_suppressed;
    target.ai_can_attack_unrepresented += outcome.ai_can_attack_unrepresented;
    target.ai_can_attack_rejections += outcome.ai_can_attack_rejections;
    target.alert_triggers += outcome.alert_triggers;
    target.alert_rejections += outcome.alert_rejections;
    target.movement_interrupts += outcome.movement_interrupts;
    target.victim_switches += outcome.victim_switches;
    target.evades_started += outcome.evades_started;
    target.assistance_scheduled += outcome.assistance_scheduled;
    target.assistance_starts += outcome.assistance_starts;
    target.aggro_starts += outcome.aggro_starts;

    for effect in &outcome.effects { append_effect(&mut target.plan, effect); }
    target.commands.extend(outcome.commands.into_iter().map(|command| CreatureAttackStartLikeCppCommand {
        attacker_guid: command.attacker_guid, victim_guid: command.victim_guid,
        previous_victim_guid: command.previous_victim_guid, map_id: command.map_id, instance_id: command.instance_id,
        packet_already_broadcast: command.packet_already_broadcast,
    }));
    target.stop_commands.extend(outcome.stop_commands.into_iter().map(|command| CreatureAttackStopLikeCppCommand {
        attacker_guid: command.attacker_guid, victim_guid: command.victim_guid,
        map_id: command.map_id, instance_id: command.instance_id,
    }));
}

pub(in crate::session) fn append_effect(plan: &mut RuntimePlan, effect: &AggroEffect) {
    let (map_id, instance_id) = (effect.map_id, effect.instance_id);
    let packet_bytes = match &effect.kind {
        AggroEffectKind::RemoveAuras(slots) => wow_packet::packets::misc::AuraUpdate {
            unit_guid: effect.source_guid, update_all: false,
            auras: slots.iter().map(|slot| wow_packet::packets::misc::AuraInfoLikeCpp { slot: *slot, aura_data: None }).collect(),
        }.to_bytes(),
        AggroEffectKind::AttackStart { victim } => wow_packet::packets::combat::AttackStart {
            attacker: effect.source_guid, victim: *victim,
        }.to_bytes(),
        AggroEffectKind::AttackStop { victim } => wow_packet::packets::combat::SAttackStop {
            attacker: effect.source_guid, victim: *victim, now_dead: false,
        }.to_bytes(),
        AggroEffectKind::MoveStop(stop) => wow_packet::packets::movement::MonsterMoveStop {
            mover_guid: effect.source_guid, current_pos: stop.position, spline_id: stop.spline_id,
        }.to_bytes(),
        AggroEffectKind::Alert => wow_packet::packets::combat::AIReaction {
            unit_guid: effect.source_guid, reaction: wow_constants::creature::AiReaction::Alert,
        }.to_bytes(),
    };
    let recipients = match &effect.kind {
        AggroEffectKind::Alert => RecipientRule::MapBroadcastVisible { map_id, instance_id },
        AggroEffectKind::MoveStop(_) => RecipientRule::NearbyVisible {
            source_guid: effect.source_guid, map_id, instance_id,
            source_position: effect.position, range: effect.visibility_range, required_3d: false,
        },
        _ => RecipientRule::NearbyVisibleDurable {
            source_guid: effect.source_guid, map_id, instance_id,
            source_position: effect.position, range: effect.visibility_range, required_3d: false,
        },
    };
    plan.events.push(RuntimeEvent { source_guid: effect.source_guid, recipients, packet_bytes });
}
