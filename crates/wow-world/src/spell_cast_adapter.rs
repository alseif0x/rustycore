//! Lossless packet/domain conversion for data retained by active and queued casts.
//! No validation, normalization, serialization, or gameplay decisions occur here.

use wow_entities::{SpellCastLocationLikeCpp, SpellCastTargetsLikeCpp, SpellCastVisualLikeCpp};
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData, TargetLocation};

pub(crate) fn retain_targets(value: SpellTargetData) -> SpellCastTargetsLikeCpp {
    SpellCastTargetsLikeCpp {
        flags: value.flags,
        unit: value.unit,
        item: value.item,
        src_location: value.src_location.map(|location| SpellCastLocationLikeCpp {
            transport: location.transport,
            position: location.position,
        }),
        dst_location: value.dst_location.map(|location| SpellCastLocationLikeCpp {
            transport: location.transport,
            position: location.position,
        }),
        orientation: value.orientation,
        map_id: value.map_id,
        name: value.name,
    }
}

pub(crate) fn present_targets(value: SpellCastTargetsLikeCpp) -> SpellTargetData {
    SpellTargetData {
        flags: value.flags,
        unit: value.unit,
        item: value.item,
        src_location: value.src_location.map(|location| TargetLocation {
            transport: location.transport,
            position: location.position,
        }),
        dst_location: value.dst_location.map(|location| TargetLocation {
            transport: location.transport,
            position: location.position,
        }),
        orientation: value.orientation,
        map_id: value.map_id,
        name: value.name,
    }
}

pub(crate) fn retain_visual(value: SpellCastVisual) -> SpellCastVisualLikeCpp {
    SpellCastVisualLikeCpp {
        spell_visual_id: value.spell_visual_id,
        script_visual_id: value.script_visual_id,
    }
}

pub(crate) fn present_visual(value: SpellCastVisualLikeCpp) -> SpellCastVisual {
    SpellCastVisual {
        spell_visual_id: value.spell_visual_id,
        script_visual_id: value.script_visual_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::{ObjectGuid, Position};
    use wow_packet::WorldPacket;

    #[test]
    fn retained_targets_preserve_every_optional_combination_and_wire_bytes() {
        for mask in 0..16 {
            let source = SpellTargetData {
                flags: 0x0abc_def0,
                unit: ObjectGuid::create_player(1, 42),
                item: ObjectGuid::new(12, 34),
                src_location: (mask & 1 != 0).then_some(TargetLocation {
                    transport: ObjectGuid::new(56, 78),
                    position: Position::new(1.25, -2.5, 3.75, 0.5),
                }),
                dst_location: (mask & 2 != 0).then_some(TargetLocation {
                    transport: ObjectGuid::new(90, 12),
                    position: Position::new(-4.25, 5.5, -6.75, -0.5),
                }),
                orientation: (mask & 4 != 0).then_some(-0.25),
                map_id: (mask & 8 != 0).then_some(-1),
                name: "target-é".to_owned(),
            };
            let retained = retain_targets(source.clone());
            let restored = present_targets(retained.clone());
            assert_eq!(restored, source);
            assert_eq!(retain_targets(restored.clone()), retained);
            let mut before = WorldPacket::new_empty();
            let mut after = WorldPacket::new_empty();
            source.write(&mut before);
            restored.write(&mut after);
            assert_eq!(before.data(), after.data());
        }
        assert_eq!(
            present_targets(retain_targets(Default::default())),
            SpellTargetData::default()
        );
    }

    #[test]
    fn retained_visual_preserves_unserialized_script_value_without_adding_wire_bytes() {
        for (spell_visual_id, script_visual_id) in [(0, 0), (42, 91), (u32::MAX, u32::MAX)] {
            let value = SpellCastVisual {
                spell_visual_id,
                script_visual_id,
            };
            let restored = present_visual(retain_visual(value.clone()));
            assert_eq!(restored.spell_visual_id, spell_visual_id);
            assert_eq!(restored.script_visual_id, script_visual_id);
            let mut before = WorldPacket::new_empty();
            let mut after = WorldPacket::new_empty();
            value.write(&mut before);
            restored.write(&mut after);
            assert_eq!(before.data(), after.data());
            assert_eq!(after.data().len(), 4);
        }
    }
}
