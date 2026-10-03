//! Goldens derived from pinned C++ table/enums and mask control flow.
//! Synthetic public metadata only. Not a native capture or executed acceptance.
use super::*;
use std::cell::Cell;

fn fnv(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
    })
}

#[test]
fn every_target_retains_all_five_source_metadata_columns() {
    let mut bytes = Vec::new();
    for id in 0..153 {
        let target = ImplicitTargetInfo::from_id(id).unwrap();
        assert_eq!(target.id(), id);
        bytes.extend([
            target.object_type() as u8,
            target.reference_type() as u8,
            target.selection_category() as u8,
            target.check_type() as u8,
            target.direction_type() as u8,
        ]);
    }
    assert_eq!(bytes.len(), 765);
    assert_eq!(fnv(bytes), 0x2c65_6d80_c2bb_9917);
    assert!(ImplicitTargetInfo::from_id(153).is_none());
    assert!(ImplicitTargetInfo::from_id(u32::MAX).is_none());
}

#[test]
fn explicit_mask_and_location_transitions_cover_all_612_initial_states() {
    let mut bytes = Vec::new();
    for id in 0..153 {
        let target = ImplicitTargetInfo::from_id(id).unwrap();
        for state in 0..4 {
            let mut source = state & 1 != 0;
            let mut destination = state & 2 != 0;
            let mask = target.explicit_target_mask(&mut source, &mut destination);
            bytes.extend(mask.to_le_bytes());
            bytes.extend([u8::from(source), u8::from(destination)]);
        }
    }
    assert_eq!(bytes.len(), 3672);
    assert_eq!(fnv(bytes), 0x3fc3_99c1_f874_73a5);
}

#[test]
fn a_then_b_order_reads_old_flags_before_marking_this_object() {
    let source = ImplicitTargetInfo::from_id(22).unwrap();
    let trajectory = ImplicitTargetInfo::from_id(89).unwrap();
    let (mut src, mut dst) = (false, false);
    assert_eq!(source.explicit_target_mask(&mut src, &mut dst), 0);
    assert_eq!((src, dst), (true, false));
    assert_eq!(trajectory.explicit_target_mask(&mut src, &mut dst), 0x40);
    assert_eq!((src, dst), (true, true));
    let (mut src, mut dst) = (false, false);
    assert_eq!(trajectory.explicit_target_mask(&mut src, &mut dst), 0x60);
    assert_eq!((src, dst), (false, true)); // trajectory itself supplies DEST, not SRC
    assert_eq!(source.explicit_target_mask(&mut src, &mut dst), 0);
    assert_eq!((src, dst), (true, true));
}

#[test]
fn destination_provision_and_unit_destination_do_not_invent_unit_requirements() {
    let mut src = false;
    let mut dst = false;
    assert_eq!(
        ImplicitTargetInfo::from_id(8)
            .unwrap()
            .explicit_target_mask(&mut src, &mut dst),
        0x40
    );
    assert!(!dst); // UNIT references DEST but does not supply it
    assert_eq!(
        ImplicitTargetInfo::from_id(18)
            .unwrap()
            .explicit_target_mask(&mut src, &mut dst),
        0
    );
    assert!(dst);
    assert_eq!(
        ImplicitTargetInfo::from_id(8)
            .unwrap()
            .explicit_target_mask(&mut src, &mut dst),
        0
    );
    dst = false;
    assert_eq!(
        ImplicitTargetInfo::from_id(116)
            .unwrap()
            .explicit_target_mask(&mut src, &mut dst),
        0
    );
    assert!(dst); // UNIT_AND_DEST / LAST supplies destination without a target flag
}

#[test]
fn object_flag_masks_include_effect_only_object_types() {
    let cases = [
        (TargetObject::None, 0),
        (TargetObject::Source, 0x20),
        (TargetObject::Destination, 0x40),
        (TargetObject::Unit, 0x2),
        (TargetObject::UnitAndDestination, 0x42),
        (TargetObject::GameObject, 0x800),
        (TargetObject::GameObjectItem, 0x4000),
        (TargetObject::Item, 0x10),
        (TargetObject::Corpse, 0x8200),
        (TargetObject::CorpseEnemy, 0x200),
        (TargetObject::CorpseAlly, 0x8000),
    ];
    for (object, mask) in cases {
        assert_eq!(object.flag_mask(), mask);
    }
}

#[test]
fn area_means_only_area_or_cone_not_line_trajectory_channel_or_nyi() {
    for id in [
        7, 8, 15, 16, 24, 30, 54, 56, 93, 108, 115, 116, 128, 129, 130, 151,
    ] {
        assert!(
            ImplicitTargetInfo::from_id(id).unwrap().is_area(),
            "target {id}"
        );
    }
    for id in [
        0, 11, 18, 36, 76, 77, 89, 126, 127, 133, 134, 135, 139, 140, 152,
    ] {
        assert!(
            !ImplicitTargetInfo::from_id(id).unwrap().is_area(),
            "target {id}"
        );
    }
    let nyi = ImplicitTargetInfo::from_id(11).unwrap();
    assert_eq!(nyi.object_type(), TargetObject::Unit);
    assert_eq!(nyi.reference_type(), TargetReference::Source);
    assert_eq!(nyi.selection_category(), TargetSelection::NotImplemented);
}

#[test]
fn fixed_direction_angles_match_cpp_double_to_float_bits_without_random_draw() {
    let cases = [
        (0, 0),
        (47, 0),
        (48, 0x4049_0fdb),
        (49, 0xbfc9_0fdb),
        (50, 0x3fc9_0fdb),
        (41, 0xbf49_0fdb),
        (42, 0xc016_cbe4),
        (43, 0x4016_cbe4),
        (44, 0x3f49_0fdb),
        (64, 0),
        (65, 0x4049_0fdb),
        (142, 0xbf49_0fdb),
    ];
    for (id, bits) in cases {
        let angle = ImplicitTargetInfo::from_id(id)
            .unwrap()
            .direction_angle(|| panic!("fixed target must not draw"));
        assert_eq!(angle.to_bits(), bits, "target {id}");
    }
}

#[test]
fn normalized_random_is_consumed_once_only_for_the_seven_random_rows() {
    let calls = Cell::new(0);
    let mut random_ids = Vec::new();
    for id in 0..153 {
        let target = ImplicitTargetInfo::from_id(id).unwrap();
        let old = calls.get();
        let angle = target.direction_angle(|| {
            calls.set(calls.get() + 1);
            0.25
        });
        if calls.get() != old {
            random_ids.push(id);
            assert_eq!(angle.to_bits(), 0x3fc9_0fdb);
            assert_eq!(calls.get(), old + 1);
        }
    }
    assert_eq!(random_ids, vec![72, 73, 74, 75, 86, 91, 149]);
    assert_eq!(calls.get(), 7);
}

#[test]
fn unnamed_and_modern_target_rows_keep_source_identity_without_fallback() {
    let last = ImplicitTargetInfo::from_id(116).unwrap();
    assert_eq!(last.object_type(), TargetObject::UnitAndDestination);
    assert_eq!(last.reference_type(), TargetReference::Last);
    assert_eq!(last.check_type(), TargetCheck::Enemy);
    let line = ImplicitTargetInfo::from_id(133).unwrap();
    assert_eq!(line.selection_category(), TargetSelection::Line);
    assert_eq!(line.reference_type(), TargetReference::Destination);
    assert_eq!(line.check_type(), TargetCheck::Ally);
    let unnamed = ImplicitTargetInfo::from_id(151).unwrap();
    assert_eq!(unnamed.selection_category(), TargetSelection::Area);
    assert_eq!(unnamed.check_type(), TargetCheck::Enemy);
}
