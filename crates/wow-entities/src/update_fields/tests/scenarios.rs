//! Update-field regressions.
//!
//! Moved out of update_fields.rs under #685; every test is unchanged.

use super::*;

#[test]
fn update_field_section_metadata_matches_cpp_template_constants() {
    let expected = [
        (
            UpdateFieldSectionKind::ObjectData,
            4,
            1,
            1,
            Some(TYPEID_OBJECT),
        ),
        (
            UpdateFieldSectionKind::ItemData,
            43,
            2,
            1,
            Some(TYPEID_ITEM),
        ),
        (
            UpdateFieldSectionKind::ContainerData,
            39,
            2,
            1,
            Some(TYPEID_CONTAINER),
        ),
        (
            UpdateFieldSectionKind::UnitData,
            227,
            8,
            1,
            Some(TYPEID_UNIT),
        ),
        (
            UpdateFieldSectionKind::PlayerData,
            108,
            4,
            1,
            Some(TYPEID_PLAYER),
        ),
        (
            UpdateFieldSectionKind::ActivePlayerData,
            1525,
            48,
            2,
            Some(TYPEID_ACTIVE_PLAYER),
        ),
        (
            UpdateFieldSectionKind::GameObjectData,
            20,
            1,
            1,
            Some(TYPEID_GAME_OBJECT),
        ),
        (
            UpdateFieldSectionKind::DynamicObjectData,
            7,
            1,
            1,
            Some(TYPEID_DYNAMIC_OBJECT),
        ),
        (
            UpdateFieldSectionKind::CorpseData,
            32,
            1,
            1,
            Some(TYPEID_CORPSE),
        ),
        (
            UpdateFieldSectionKind::AreaTriggerData,
            20,
            1,
            1,
            Some(TYPEID_AREA_TRIGGER),
        ),
        (
            UpdateFieldSectionKind::SceneObjectData,
            5,
            1,
            1,
            Some(TYPEID_SCENE_OBJECT),
        ),
        (
            UpdateFieldSectionKind::ConversationData,
            4,
            1,
            1,
            Some(TYPEID_CONVERSATION),
        ),
    ];

    assert_eq!(UpdateFieldSectionKind::ALL.len(), expected.len());
    for (kind, bit_count, block_count, blocks_mask_count, client_type_bit) in expected {
        let metadata = kind.metadata();
        assert_eq!(metadata.bit_count, bit_count, "{kind:?} bit count");
        assert_eq!(metadata.block_count, block_count, "{kind:?} block count");
        assert_eq!(
            metadata.blocks_mask_count, blocks_mask_count,
            "{kind:?} blocks-mask count"
        );
        assert_eq!(
            metadata.client_type_bit, client_type_bit,
            "{kind:?} type bit"
        );
    }
}

#[test]
fn update_mask_set_reset_and_blocks_mask_match_cpp_semantics() {
    let mut mask = UpdateMask::new(64);

    mask.set(0);
    mask.set(31);
    mask.set(32);

    assert_eq!(mask.get_block(0), 0x8000_0001);
    assert_eq!(mask.get_block(1), 0x0000_0001);
    assert_eq!(mask.get_blocks_mask(0), 0x0000_0003);
    assert!(mask.is_any_set());

    mask.reset(31);
    assert_eq!(mask.get_block(0), 0x0000_0001);
    assert_eq!(mask.get_blocks_mask(0), 0x0000_0003);

    mask.reset(0);
    assert_eq!(mask.get_block(0), 0);
    assert_eq!(mask.get_blocks_mask(0), 0x0000_0002);
}

#[test]
fn update_mask_set_all_marks_used_blocks_and_masks_unused_tail_bits() {
    let mut mask = UpdateMask::new(35);

    mask.set_all();

    assert_eq!(mask.get_block(0), u32::MAX);
    assert_eq!(mask.get_block(1), 0x0000_0007);
    assert_eq!(mask.get_blocks_mask(0), 0x0000_0003);
}

#[test]
fn update_mask_from_blocks_masks_unused_tail_bits() {
    let mask = UpdateMask::from_blocks(35, &[u32::MAX, u32::MAX]);

    assert_eq!(mask.blocks(), &[u32::MAX, 0x0000_0007]);
    assert_eq!(mask.blocks_mask(), &[0x0000_0003]);
}

#[test]
fn update_mask_and_or_recompute_empty_block_masks() {
    let mut left = UpdateMask::new(64);
    left.set(0);
    left.set(32);
    let mut right = UpdateMask::new(64);
    right.set(32);

    let anded = left.clone() & right.clone();
    assert_eq!(anded.get_block(0), 0);
    assert_eq!(anded.get_block(1), 1);
    assert_eq!(anded.get_blocks_mask(0), 0x0000_0002);

    let ored = anded | right;
    assert_eq!(ored.get_block(1), 1);
    assert_eq!(ored.get_blocks_mask(0), 0x0000_0002);
}

#[test]
fn item_visibility_masks_match_cpp_write_update_filters() {
    let base = allowed_mask_for_visibility(
        UpdateFieldSectionKind::ItemData,
        UpdateFieldVisibilityFlags::empty(),
    );
    assert_eq!(base.blocks(), &ITEM_BASE_ALLOWED_BLOCKS);

    let owner = allowed_mask_for_visibility(
        UpdateFieldSectionKind::ItemData,
        UpdateFieldVisibilityFlags::OWNER,
    );
    assert_eq!(owner.blocks(), &[0xFFFF_FFFF, 0x0000_07FF]);

    let mut filtered = UpdateMask::all_bits(ITEM_DATA_BITS);
    filter_disallowed_fields(
        UpdateFieldSectionKind::ItemData,
        &mut filtered,
        UpdateFieldVisibilityFlags::empty(),
    );
    assert_eq!(filtered.blocks(), &ITEM_BASE_ALLOWED_BLOCKS);
}

#[test]
fn unit_visibility_masks_match_cpp_write_update_filters() {
    let base = allowed_mask_for_visibility(
        UpdateFieldSectionKind::UnitData,
        UpdateFieldVisibilityFlags::empty(),
    );
    assert_eq!(base.blocks(), &UNIT_BASE_ALLOWED_BLOCKS);

    let owner_extra = extra_allowed_mask_for_visibility(
        UpdateFieldSectionKind::UnitData,
        UpdateFieldVisibilityFlags::OWNER,
    );
    assert_eq!(owner_extra.blocks(), &UNIT_OWNER_ALLOWED_BLOCKS);

    let unit_all_extra = extra_allowed_mask_for_visibility(
        UpdateFieldSectionKind::UnitData,
        UpdateFieldVisibilityFlags::UNIT_ALL,
    );
    assert_eq!(unit_all_extra.blocks(), &UNIT_ALL_ALLOWED_BLOCKS);

    let empath_extra = extra_allowed_mask_for_visibility(
        UpdateFieldSectionKind::UnitData,
        UpdateFieldVisibilityFlags::EMPATH,
    );
    assert_eq!(empath_extra.blocks(), &UNIT_EMPATH_ALLOWED_BLOCKS);

    let all_flags = allowed_mask_for_visibility(
        UpdateFieldSectionKind::UnitData,
        UpdateFieldVisibilityFlags::OWNER
            | UpdateFieldVisibilityFlags::UNIT_ALL
            | UpdateFieldVisibilityFlags::EMPATH,
    );
    assert_eq!(
        all_flags.blocks(),
        &[
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            0x0000_0007,
        ]
    );
}

#[test]
fn player_visibility_masks_match_cpp_write_update_filters() {
    let base = allowed_mask_for_visibility(
        UpdateFieldSectionKind::PlayerData,
        UpdateFieldVisibilityFlags::empty(),
    );
    assert_eq!(base.blocks(), &PLAYER_BASE_ALLOWED_BLOCKS);

    let party = allowed_mask_for_visibility(
        UpdateFieldSectionKind::PlayerData,
        UpdateFieldVisibilityFlags::PARTY_MEMBER,
    );
    assert_eq!(
        party.blocks(),
        &[0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF, 0x0000_0FFF]
    );
}

#[test]
fn unfiltered_sections_return_all_bits_truncated_to_bit_count() {
    for kind in [
        UpdateFieldSectionKind::ObjectData,
        UpdateFieldSectionKind::ContainerData,
        UpdateFieldSectionKind::ActivePlayerData,
        UpdateFieldSectionKind::GameObjectData,
        UpdateFieldSectionKind::DynamicObjectData,
        UpdateFieldSectionKind::CorpseData,
        UpdateFieldSectionKind::AreaTriggerData,
        UpdateFieldSectionKind::SceneObjectData,
        UpdateFieldSectionKind::ConversationData,
    ] {
        let mask = allowed_mask_for_visibility(kind, UpdateFieldVisibilityFlags::OWNER);
        assert_eq!(mask.bits(), kind.bit_count());
        assert_eq!(mask.block_count(), kind.metadata().block_count);
        for bit in 0..kind.bit_count() {
            assert!(mask.is_set(bit), "{kind:?} bit {bit} should be allowed");
        }
        if kind.bit_count() % 32 != 0 {
            let expected_last = u32::MAX >> (32 - (kind.bit_count() % 32));
            assert_eq!(mask.blocks().last().copied(), Some(expected_last));
        }
    }
}

#[test]
fn dynamic_and_nested_descriptors_cover_key_generated_fields() {
    let artifact_powers = descriptor(UpdateFieldSectionKind::ItemData, "ArtifactPowers").unwrap();
    assert_eq!(artifact_powers.kind, UpdateFieldDescriptorKind::Dynamic);
    assert_eq!(artifact_powers.bit, 1);

    let enchantment = descriptor(UpdateFieldSectionKind::ItemData, "Enchantment").unwrap();
    assert_eq!(enchantment.first_child_bit, Some(30));
    assert_eq!(enchantment.element_count, Some(13));
    assert_eq!(enchantment.last_child_bit(), Some(42));
    assert_eq!(enchantment.nested_bit_count, Some(6));

    let quest_log = descriptor(UpdateFieldSectionKind::PlayerData, "QuestLog").unwrap();
    assert_eq!(quest_log.bit, 35);
    assert_eq!(quest_log.first_child_bit, Some(36));
    assert_eq!(quest_log.last_child_bit(), Some(60));
    assert_eq!(quest_log.nested_bit_count, Some(29));

    let visible_items = descriptor(UpdateFieldSectionKind::PlayerData, "VisibleItems").unwrap();
    assert_eq!(visible_items.bit, 61);
    assert_eq!(visible_items.first_child_bit, Some(62));
    assert_eq!(visible_items.last_child_bit(), Some(80));
    assert_eq!(visible_items.nested_bit_count, Some(4));

    let corpse_items = descriptor(UpdateFieldSectionKind::CorpseData, "Items").unwrap();
    assert_eq!(corpse_items.bit, 12);
    assert_eq!(corpse_items.first_child_bit, Some(13));
    assert_eq!(corpse_items.last_child_bit(), Some(31));

    let lines = descriptor(UpdateFieldSectionKind::ConversationData, "Lines").unwrap();
    assert_eq!(lines.kind, UpdateFieldDescriptorKind::Dynamic);
    assert_eq!(lines.bit, 1);
    let actors = descriptor(UpdateFieldSectionKind::ConversationData, "Actors").unwrap();
    assert_eq!(actors.kind, UpdateFieldDescriptorKind::Dynamic);
    assert_eq!(actors.bit, 2);
    let last_line_end_time =
        descriptor(UpdateFieldSectionKind::ConversationData, "LastLineEndTime").unwrap();
    assert_eq!(last_line_end_time.kind, UpdateFieldDescriptorKind::Scalar);
    assert_eq!(last_line_end_time.bit, 3);
}

#[test]
fn active_player_descriptors_capture_large_generated_sections() {
    let skill = descriptor(UpdateFieldSectionKind::ActivePlayerData, "Skill").unwrap();
    assert_eq!(skill.kind, UpdateFieldDescriptorKind::Nested);
    assert_eq!(skill.bit, 32);
    assert_eq!(skill.nested_bit_count, Some(1793));

    let inv_slots = descriptor(UpdateFieldSectionKind::ActivePlayerData, "InvSlots").unwrap();
    assert_eq!(inv_slots.first_child_bit, Some(125));
    assert_eq!(inv_slots.element_count, Some(141));
    assert_eq!(inv_slots.last_child_bit(), Some(265));

    let pet_stable = descriptor(UpdateFieldSectionKind::ActivePlayerData, "PetStable").unwrap();
    assert_eq!(pet_stable.kind, UpdateFieldDescriptorKind::Optional);
    assert_eq!(pet_stable.bit, 122);
    assert_eq!(pet_stable.nested_bit_count, Some(3));

    let quest_completed =
        descriptor(UpdateFieldSectionKind::ActivePlayerData, "QuestCompleted").unwrap();
    assert_eq!(quest_completed.first_child_bit, Some(637));
    assert_eq!(quest_completed.element_count, Some(875));
    assert_eq!(quest_completed.last_child_bit(), Some(1511));
}

#[test]
fn values_update_sections_sets_client_type_bits_from_section_masks() {
    let mut sections = ValuesUpdateSections::empty();
    sections.push(UpdateFieldSectionUpdate {
        kind: UpdateFieldSectionKind::ItemData,
        mask: UpdateMask::new(ITEM_DATA_BITS),
    });
    assert!(!sections.has_data());

    let mut unit_mask = UpdateMask::new(UNIT_DATA_BITS);
    unit_mask.set(5);
    sections.push(UpdateFieldSectionUpdate {
        kind: UpdateFieldSectionKind::UnitData,
        mask: unit_mask,
    });
    assert!(sections.has_data());
    assert_eq!(sections.changed_object_type_mask, 1 << TYPEID_UNIT);
}
