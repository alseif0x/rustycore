//! Spell wire stays in APP; represented hit/log rules belong to Map.
use super::*;
use super::canonical_runtime::spell::projection;
pub(in crate::session) use wow_map::map_manager::SpellHitProfile as CreatureSpellHitProfileLikeCpp;

pub(in crate::session) fn represented_creature_spell_hit_profile_like_cpp(
    metadata: &wow_data::spell::SpellHitMetadataLikeCpp, active_effect_indices: &[u32], attributes: [u32; 15],
) -> Option<CreatureSpellHitProfileLikeCpp> {
    wow_map::map_manager::represented_hit_profile(&projection::hit_metadata(metadata.clone()), active_effect_indices, attributes)
}
pub(in crate::session) fn resolve_creature_spell_hit_profile_like_cpp(
    profile: CreatureSpellHitProfileLikeCpp, roll: Option<u32>,
) -> Option<CreatureSpellTargetHitResultLikeCpp> {
    wow_map::map_manager::resolve_hit_profile(profile, roll).map(projection::hit)
}
pub(in crate::session) fn creature_spell_cast_log_data_like_cpp(
    caster: &wow_entities::Creature, spell: &wow_data::SpellInfo, difficulty_id: u8,
) -> Option<wow_packet::packets::spell::SpellCastLogData> {
    wow_map::map_manager::cast_log(caster, &projection::spell_info(spell), difficulty_id).map(projection::log)
}

struct SpellWire {
    caster_guid: ObjectGuid, target_guid: ObjectGuid, map_id: u16, instance_id: u32,
    spell_id: i32, spell_x_spell_visual_id: u32, cast_time_ms: u32, spell_go_cast_flags: u32,
}

pub(in crate::session) fn append_committed_creature_spell_packets_like_cpp(
    plan: &mut RuntimePlan, command: &CreatureSpellCastPlanLikeCpp, cast_id: ObjectGuid,
    hit_result: CreatureSpellTargetHitResultLikeCpp, source_position: Position, visibility_range: f32,
    full_log_data: &wow_packet::packets::spell::SpellCastLogData,
) {
    append_wire(plan, SpellWire { caster_guid: command.caster_guid, target_guid: command.target_guid,
        map_id: command.map_id, instance_id: command.instance_id, spell_id: command.spell_id,
        spell_x_spell_visual_id: command.spell_x_spell_visual_id, cast_time_ms: command.cast_time_ms,
        spell_go_cast_flags: command.spell_go_cast_flags }, cast_id, hit_result, source_position, visibility_range, full_log_data);
}

pub(in crate::session) fn append_completion(plan: &mut RuntimePlan, completion: wow_map::map_manager::SpellCompletion) {
    let log = projection::log(completion.log);
    append_wire(plan, SpellWire { caster_guid: completion.caster_guid, target_guid: completion.target_guid,
        map_id: completion.map_id, instance_id: completion.instance_id, spell_id: completion.spell_id,
        spell_x_spell_visual_id: completion.spell_x_spell_visual_id, cast_time_ms: completion.cast_time_ms,
        spell_go_cast_flags: completion.spell_go_cast_flags }, completion.cast_id, projection::hit(completion.hit),
        completion.position, completion.visibility_range, &log);
}

fn append_wire(plan: &mut RuntimePlan, wire: SpellWire, cast_id: ObjectGuid,
    hit_result: CreatureSpellTargetHitResultLikeCpp, source_position: Position, visibility_range: f32,
    full_log_data: &wow_packet::packets::spell::SpellCastLogData,
) {
    use wow_packet::ServerPacket;
    use wow_packet::packets::spell::{
        SpellCastVisual, SpellGoPkt, SpellMissReason, SpellMissTarget, SpellStartPkt,
        SpellTargetData,
    };

    let visual = SpellCastVisual {
        spell_visual_id: wire.spell_x_spell_visual_id,
        script_visual_id: 0,
    };
    let target = SpellTargetData {
        flags: 0x2,
        unit: wire.target_guid,
        item: ObjectGuid::EMPTY,
        ..Default::default()
    };
    let start = SpellStartPkt {
        cast_data: Default::default(),
        caster: wire.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: wire.spell_id,
        visual: visual.clone(),
        cast_flags: 0x0000_0002,
        cast_flags_ex: 0,
        cast_time_ms: wire.cast_time_ms,
        target: target.clone(),
    };
    let (hit_targets, miss_targets) = match hit_result {
        CreatureSpellTargetHitResultLikeCpp::Hit => (vec![wire.target_guid], Vec::new()),
        CreatureSpellTargetHitResultLikeCpp::Miss => (
            Vec::new(),
            vec![SpellMissTarget::new(
                wire.target_guid,
                SpellMissReason::Miss,
            )],
        ),
    };
    let go = SpellGoPkt {
        cast_data: Default::default(),
        caster: wire.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: wire.spell_id,
        visual,
        cast_flags: wire.spell_go_cast_flags,
        cast_flags_ex: 0,
        cast_time_ms: crate::session::game_time_ms_like_cpp(),
        target,
        hit_targets,
        miss_targets,
    };
    plan.events.push(RuntimeEvent {
        source_guid: wire.caster_guid,
        recipients: RecipientRule::NearbyVisibleDurableSpellCast {
            source_guid: wire.caster_guid,
            map_id: wire.map_id,
            instance_id: wire.instance_id,
            source_position,
            range: visibility_range,
            required_3d: false,
            basic_go_packet_bytes: go.to_bytes(),
            full_go_packet_bytes: go.to_full_log_bytes_like_cpp(full_log_data),
        },
        packet_bytes: start.to_bytes(),
    });
}
