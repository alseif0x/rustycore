use crate::{Item, ItemValuesSnapshot};
use wow_constants::{ItemBondingType, ItemFieldFlags};

const UNRELATED_FLAG: u32 = 0x8000_0040;

#[test]
fn binding_flags_match_canonical_setters_without_mutating_the_view_source() {
    let bonding_modes = [
        ItemBondingType::None,
        ItemBondingType::OnAcquire,
        ItemBondingType::OnEquip,
        ItemBondingType::OnUse,
        ItemBondingType::Quest,
    ];
    let modes = [(true, false), (false, false), (false, true)];
    let expected_binding = [
        [false, false, false],
        [true, true, true],
        [true, false, true],
        [false, false, false],
        [true, true, true],
    ];

    for (bonding_index, bonding) in bonding_modes.into_iter().enumerate() {
        for (mode_index, (visualized, is_bag_pos)) in modes.into_iter().enumerate() {
            for already_bound in [false, true] {
                let flags = UNRELATED_FLAG
                    | if already_bound {
                        ItemFieldFlags::SOULBOUND.bits()
                    } else {
                        0
                    };
                let mut source = Item::default();
                source.set_bonding(bonding);
                source.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
                let before = ItemValuesSnapshot::capture(&source);

                let view_flags = source
                    .read_view()
                    .item_flags_after_binding(visualized, is_bag_pos);
                assert_eq!(ItemValuesSnapshot::capture(&source), before);
                let independently_expected_flags = flags
                    | if expected_binding[bonding_index][mode_index] {
                        ItemFieldFlags::SOULBOUND.bits()
                    } else {
                        0
                    };
                assert_eq!(view_flags, independently_expected_flags);

                let mut canonical = Item::default();
                canonical.set_bonding(bonding);
                canonical.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
                if visualized {
                    canonical.bind_if_visualized();
                } else {
                    canonical.bind_if_stored(is_bag_pos);
                }

                assert_eq!(view_flags, canonical.item_flags_bits());
                assert_eq!(view_flags & UNRELATED_FLAG, flags & UNRELATED_FLAG);
            }
        }
    }
}

#[test]
fn captured_binding_facts_remain_frozen_after_the_item_changes() {
    let mut item = Item::default();
    item.set_bonding(ItemBondingType::OnAcquire);
    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(UNRELATED_FLAG));
    let captured = ItemValuesSnapshot::capture(&item);
    let expected = captured.read_view().item_flags_after_binding(false, false);

    item.set_bonding(ItemBondingType::OnUse);
    item.replace_all_item_flags(ItemFieldFlags::empty());

    assert_eq!(
        captured.read_view().item_flags_after_binding(false, false),
        expected
    );
    assert_eq!(expected, UNRELATED_FLAG | ItemFieldFlags::SOULBOUND.bits());
}
