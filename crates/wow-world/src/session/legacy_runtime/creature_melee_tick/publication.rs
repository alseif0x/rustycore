//! Packet encoding of results captured by the synchronous Map motor.
use super::*;
use wow_map::map::{MeleeEffect, MeleePresentation, CreatureMeleePlayerHit};
use wow_packet::ServerPacket;

fn presentation(info: MeleePresentation) -> (u32, u8) {
    use wow_packet::packets::combat::*;
    let (mut flags, state) = info.outcome.map_or((HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT),
        |outcome| crate::session_rules::melee_outcome_presentation_like_cpp(outcome, false));
    if info.full_absorb { flags |= HIT_INFO_FULL_ABSORB; }
    if info.partial_absorb { flags |= HIT_INFO_PARTIAL_ABSORB; }
    if info.fake_damage { flags |= HIT_INFO_FAKE_DAMAGE; }
    (flags, state)
}

pub(super) fn effect(effect: MeleeEffect, swing: &PendingCreatureSwingLikeCpp) -> Option<RuntimeEvent> {
    use wow_packet::packets::{combat, misc};
    let broadcast = RecipientRule::MapBroadcastVisible { map_id: swing.map_id, instance_id: swing.instance_id };
    let (source_guid, recipients, packet_bytes) = match effect {
        MeleeEffect::AbsorbLog { attacker, victim, absorb_spell_id, caster, absorbed, original_damage } =>
            (victim, broadcast, combat::SpellAbsorbLog { attacker, victim, absorbed_spell_id: 0,
                absorb_spell_id, caster, absorbed, original_damage }.to_bytes()),
        MeleeEffect::AuraRemoved { unit, slot } => (unit, broadcast,
            misc::AuraUpdate { unit_guid: unit, update_all: false,
                auras: vec![misc::AuraInfoLikeCpp { slot, aura_data: None }] }.to_bytes()),
        MeleeEffect::PlayerHealth { guid, health } => (guid, RecipientRule::ExplicitPlayer(guid),
            combat::HealthUpdate { guid, health }.to_bytes()),
        MeleeEffect::Values { guid, update } => (guid, broadcast,
            crate::entity_update_bridge::unit_values_update_to_update_object(guid, swing.map_id, &update)?.to_bytes()),
        MeleeEffect::AttackState { attacker, victim, presentation: info, damage, original_damage,
            over_damage, blocked, absorbed, target_level } => {
            let (hit_info, victim_state) = presentation(info);
            (attacker, broadcast, combat::AttackerStateUpdate { attacker, victim, hit_info, damage,
                original_damage, over_damage, blocked, absorbed, victim_state, school_mask: 1,
                target_level, expansion: 2 }.to_bytes())
        }
        MeleeEffect::SplitMiss { spell_id, caster, victim } => (swing.victim_guid, broadcast,
            combat::SpellMissLog { spell_id, caster,
                entries: vec![combat::SpellMissLogEntry { victim, miss_reason: 7 }] }.to_bytes()),
        MeleeEffect::SplitDamage { target, caster, cast_id, spell_id, visual_id, damage,
            original_damage, overkill, school_mask, absorbed } => (swing.victim_guid, broadcast,
            combat::SpellNonMeleeDamageLog { target, caster, cast_id, spell_id, visual_id, damage,
                original_damage, overkill, school_mask, absorbed, resisted: 0, shield_block: 0,
                periodic: false, flags: 0 }.to_bytes()),
    };
    Some(RuntimeEvent { source_guid, recipients, packet_bytes })
}

pub(super) fn command(hit: CreatureMeleePlayerHit) -> crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
    let (hit_info, victim_state) = presentation(hit.presentation);
    crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: hit.swing.attacker_guid, victim_guid: hit.swing.victim_guid,
        map_id: hit.swing.map_id, instance_id: hit.swing.instance_id, damage: hit.damage,
        over_damage: hit.over_damage, target_level: hit.target_level,
        victim_health_after: hit.victim_health_after,
        victim_health_state_revision_after: hit.victim_health_state_revision_after,
        hit_info, victim_state, original_damage: hit.original_damage,
        absorbed: hit.absorbed, mana_spent: hit.mana_spent,
        absorb_consumptions: hit.absorb_consumptions.into_iter().map(|consumed|
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp {
                slot: consumed.slot, consumed: consumed.consumed, removed: consumed.removed }).collect(),
        split_combat_log_packets: hit.split_combat_log_packets.into_iter()
            .filter_map(|item| effect(item, &hit.swing).map(|event| event.packet_bytes)).collect(),
        self_share_health_updates: hit.self_share_health_updates,
    }
}

