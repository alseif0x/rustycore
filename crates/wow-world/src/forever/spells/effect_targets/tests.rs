//! Reviewed C++-derived synthetic goldens, not an executed native oracle.
use super::*;

#[test]
fn all_722_metadata_cells_match_the_source_ordinal_fingerprint() {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for id in 0..361 {
        let info = EffectTargetInfo::from_id(id).unwrap();
        assert_eq!(info.id(), id);
        for byte in [info.implicit_type() as u8, info.used_object_type() as u8] {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    assert_eq!(hash, 0xbbf9_d168_5b3a_6000);
}

#[test]
fn unknown_effect_ids_are_not_mapped_to_none_or_any_other_effect() {
    assert!(EffectTargetInfo::from_id(361).is_none());
    assert!(EffectTargetInfo::from_id(u32::MAX).is_none());
}

#[test]
fn metadata_distinguishes_implicit_kind_from_used_object() {
    let energy = EffectTargetInfo::from_id(30).unwrap();
    assert_eq!(energy.implicit_type(), EffectTargetType::None);
    assert_eq!(energy.used_object_type(), TargetObject::Unit);
    let teleport = EffectTargetInfo::from_id(13).unwrap();
    assert_eq!(teleport.implicit_type(), EffectTargetType::Explicit);
    assert_eq!(
        teleport.used_object_type(),
        TargetObject::UnitAndDestination
    );
    let resurrect = EffectTargetInfo::from_id(18).unwrap();
    assert_eq!(resurrect.used_object_type(), TargetObject::CorpseAlly);
}

#[test]
fn missing_masks_cover_all_effects_each_flag_bit_and_all_four_location_states() {
    let masks = std::iter::once(0)
        .chain((0..32).map(|bit| 1_u32 << bit))
        .chain(std::iter::once(u32::MAX))
        .collect::<Vec<_>>();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for id in 0..361 {
        let info = EffectTargetInfo::from_id(id).unwrap();
        for &provided in &masks {
            for state in 0..4 {
                let result = info.missing_target_mask(state & 1 != 0, state & 2 != 0, provided);
                for byte in result.to_le_bytes() {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
                }
            }
        }
    }
    assert_eq!(hash, 0xef3e73b4237a23e5);
}

#[test]
fn provided_groups_remove_only_the_source_covered_requirement() {
    let unit = EffectTargetInfo::from_id(2).unwrap();
    assert_eq!(unit.missing_target_mask(false, false, 0x100), 0); // ally covers unit
    assert_eq!(unit.missing_target_mask(false, false, 0x8000), 0); // corpse covers unit
    assert_eq!(unit.missing_target_mask(false, false, 0x800), 2); // gameobject is not unit
    let corpse = EffectTargetInfo::from_id(18).unwrap();
    assert_eq!(corpse.missing_target_mask(false, false, 2), 0x8000); // unit does not cover corpse
    assert_eq!(corpse.missing_target_mask(false, false, 0x200), 0);
    let lock = EffectTargetInfo::from_id(33).unwrap();
    assert_eq!(lock.missing_target_mask(false, false, 0x10), 0);
    assert_eq!(lock.missing_target_mask(false, false, 0x800), 0);
    let teleport = EffectTargetInfo::from_id(13).unwrap();
    assert_eq!(teleport.missing_target_mask(false, false, 2), 0x40);
    assert_eq!(teleport.missing_target_mask(false, true, 2), 0);
}
