//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn inventory_authority_proof_belongs_to_one_player_runtime_like_cpp() {
    let mut inventory = PlayerInventoryRuntime::default();
    assert!(!inventory.equipment_inventory_authority_complete_like_cpp());

    inventory.set_equipment_inventory_authority_complete_like_cpp(true);
    assert!(inventory.equipment_inventory_authority_complete_like_cpp());

    inventory.set_equipment_inventory_authority_complete_like_cpp(false);
    assert!(!inventory.equipment_inventory_authority_complete_like_cpp());
}
#[test]
fn player_is_valid_pos_matches_cpp_top_level_and_bag_rules() {
    let bag_guid = ObjectGuid::create_item(1, 300);
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);

    assert!(player.is_valid_pos(NULL_BAG, NULL_SLOT, false));
    assert!(!player.is_valid_pos(NULL_BAG, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, false));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, 0, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, PROFESSION_SLOT_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, REAGENT_BAG_SLOT_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 16, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_BAG_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START, true));

    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, 0, true));
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_START, NULL_SLOT, false));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_START, 3, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, 4, true));
    assert!(player.is_valid_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_START, 3), true));
}
#[test]
fn find_equip_slot_maps_inventory_types_like_cpp() {
    let player = Player::new(None, false);
    let head = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(1, 1)
    };
    let robe = ItemStorageTemplate {
        inventory_type: InventoryType::Robe,
        ..ItemStorageTemplate::regular_item(2, 1)
    };
    let bag = ItemStorageTemplate {
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(3, 1)
    };
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(4, 1)
    };
    let two_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        ..ItemStorageTemplate::regular_item(5, 1)
    };

    assert_eq!(
        player.find_equip_slot(find_equip_args(&head, NULL_SLOT, false, &[])),
        EQUIPMENT_SLOT_HEAD
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&robe, NULL_SLOT, false, &[])),
        EQUIPMENT_SLOT_CHEST
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&bag, NULL_SLOT, false, &[])),
        INVENTORY_SLOT_BAG_START
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&weapon, EQUIPMENT_SLOT_OFFHAND, false, &[])),
        NULL_SLOT
    );

    let mut dual_args = find_equip_args(&weapon, EQUIPMENT_SLOT_OFFHAND, false, &[]);
    dual_args.can_dual_wield = true;
    assert_eq!(player.find_equip_slot(dual_args), EQUIPMENT_SLOT_OFFHAND);

    let mut titan_args = find_equip_args(&two_hand, EQUIPMENT_SLOT_OFFHAND, false, &[]);
    titan_args.can_dual_wield = true;
    assert_eq!(player.find_equip_slot(titan_args), NULL_SLOT);
    titan_args.can_titan_grip = true;
    assert_eq!(player.find_equip_slot(titan_args), EQUIPMENT_SLOT_OFFHAND);
}
#[test]
fn find_equip_slot_requested_free_and_swap_paths_match_cpp() {
    let player = Player::new(None, false);
    let finger = ItemStorageTemplate {
        inventory_type: InventoryType::Finger,
        ..ItemStorageTemplate::regular_item(10, 1)
    };
    let mut ring1 = Item::default();
    ring1.set_debug_item_level(120);
    let mut ring2 = Item::default();
    ring2.set_debug_item_level(45);
    let equipped = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1, &ring1),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER2, &ring2),
    ];

    assert_eq!(
        player.find_equip_slot(find_equip_args(
            &finger,
            EQUIPMENT_SLOT_FINGER1,
            false,
            &equipped
        )),
        NULL_SLOT
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(
            &finger,
            EQUIPMENT_SLOT_FINGER1,
            true,
            &equipped
        )),
        EQUIPMENT_SLOT_FINGER1
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&finger, NULL_SLOT, true, &equipped)),
        EQUIPMENT_SLOT_FINGER2
    );

    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER1,
        &ring1,
    )];
    assert_eq!(
        player.find_equip_slot(find_equip_args(&finger, NULL_SLOT, false, &equipped)),
        EQUIPMENT_SLOT_FINGER2
    );
}
#[test]
fn find_equip_slot_twohand_offhand_and_professions_match_cpp_edges() {
    let player = Player::new(None, false);
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(20, 1)
    };
    let mut mainhand = Item::default();
    mainhand.set_debug_item_level(100);
    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        &mainhand,
    )];
    let mut args = find_equip_args(&weapon, NULL_SLOT, false, &equipped);
    args.can_dual_wield = true;
    args.is_two_hand_used = true;
    assert_eq!(player.find_equip_slot(args), NULL_SLOT);

    let cooking_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Cooking as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(21, 1)
    };
    let fishing_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Fishing as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(22, 1)
    };
    let blacksmithing_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Blacksmithing as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(23, 1)
    };

    let mut profession_args = find_equip_args(&cooking_gear, NULL_SLOT, false, &[]);
    profession_args.has_required_profession_skill = true;
    assert_eq!(
        player.find_equip_slot(profession_args),
        PROFESSION_SLOT_COOKING_GEAR1
    );

    profession_args.proto = &fishing_gear;
    assert_eq!(player.find_equip_slot(profession_args), NULL_SLOT);

    profession_args.proto = &blacksmithing_gear;
    profession_args.profession_slot = Some(0);
    assert_eq!(
        player.find_equip_slot(profession_args),
        PROFESSION_SLOT_PROFESSION1_GEAR2
    );
}
#[test]
fn can_equip_item_preflight_and_runtime_guards_match_cpp_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), None))
            .result,
        InventoryResult::ItemNotFound
    );

    let mut swap_missing = can_equip_args(NULL_SLOT, None, Some(&source));
    swap_missing.swap = true;
    assert_eq!(
        player.can_equip_item(swap_missing).result,
        InventoryResult::CantSwap
    );

    source.set_loot_generated(true);
    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), Some(&source)))
            .result,
        InventoryResult::LootGone
    );
    source.set_loot_generated(false);

    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(ObjectGuid::create_player(1, 99));
    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), Some(&source)))
            .result,
        InventoryResult::NotOwner
    );
    source.remove_item_flag(ItemFieldFlags::SOULBOUND);

    let limited = ItemStorageTemplate {
        max_count: 1,
        ..proto
    };
    source.object_mut().create(ObjectGuid::create_item(1, 900));
    source.object_mut().set_entry(limited.entry);
    let mut stored = Item::default();
    stored.object_mut().create(ObjectGuid::create_item(1, 901));
    stored.object_mut().set_entry(limited.entry);
    stored.set_count(1);
    let stored_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &stored,
        Some(&limited),
    )];
    let mut limit_args = can_equip_args(NULL_SLOT, Some(&limited), Some(&source));
    limit_args.stored_items = &stored_items;
    assert_eq!(
        player.can_equip_item(limit_args).result,
        InventoryResult::ItemMaxCount
    );

    let mut stunned = can_equip_args(NULL_SLOT, Some(&proto), Some(&source));
    stunned.is_stunned = true;
    stunned.is_charmed = true;
    assert_eq!(
        player.can_equip_item(stunned).result,
        InventoryResult::GenericStunned
    );

    let mut combat = can_equip_args(NULL_SLOT, Some(&proto), Some(&source));
    combat.is_in_combat = true;
    assert_eq!(
        player.can_equip_item(combat).result,
        InventoryResult::NotInCombat
    );

    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(101, 1)
    };
    let mut cooldown = can_equip_args(NULL_SLOT, Some(&weapon), Some(&source));
    cooldown.is_in_combat = true;
    cooldown.weapon_change_timer_active = true;
    assert_eq!(
        player.can_equip_item(cooldown).result,
        InventoryResult::ItemCooldown
    );

    let mut casting = can_equip_args(NULL_SLOT, Some(&weapon), Some(&source));
    casting.current_generic_spell_allows_equip = Some(false);
    assert_eq!(
        player.can_equip_item(casting).result,
        InventoryResult::ClientLockedOut
    );
}
#[test]
fn can_equip_item_destination_use_and_unique_paths_match_cpp() {
    let player = Player::new(None, false);
    let head = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(200, 1)
    };
    let finger = ItemStorageTemplate {
        inventory_type: InventoryType::Finger,
        ..ItemStorageTemplate::regular_item(201, 1)
    };
    let mut source = Item::default();
    source.set_count(1);
    let mut equipped_head = Item::default();
    equipped_head.set_count(1);
    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_HEAD,
        &equipped_head,
    )];

    let outcome = player.can_equip_item(can_equip_args(NULL_SLOT, Some(&head), Some(&source)));
    assert_eq!(outcome.result, InventoryResult::Ok);
    assert_eq!(
        outcome.dest,
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD)
    );
    assert_eq!(outcome.unique_ignore_slot, Some(NULL_SLOT));

    let mut occupied = can_equip_args(NULL_SLOT, Some(&head), Some(&source));
    occupied.equipped_items = &equipped;
    assert_eq!(
        player.can_equip_item(occupied).result,
        InventoryResult::NotEquippable
    );

    let mut can_use = can_equip_args(NULL_SLOT, Some(&head), Some(&source));
    can_use.can_use_result = InventoryResult::CantEquipSkill;
    assert_eq!(
        player.can_equip_item(can_use).result,
        InventoryResult::CantEquipSkill
    );

    let mut source_ring = Item::default();
    source_ring.set_count(1);
    let other_ring = Item::default();
    let rings = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1, &other_ring),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER2, &source_ring),
    ];
    let mut unique = can_equip_args(EQUIPMENT_SLOT_FINGER1, Some(&finger), Some(&source_ring));
    unique.swap = true;
    unique.equipped_items = &rings;
    unique.can_equip_unique_result = InventoryResult::ItemUniqueEquippable;
    let outcome = player.can_equip_item(unique);
    assert_eq!(outcome.result, InventoryResult::ItemUniqueEquippable);
    assert_eq!(outcome.unique_ignore_slot, Some(EQUIPMENT_SLOT_FINGER2));
}
#[test]
fn can_equip_item_quiver_offhand_and_twohand_edges_match_cpp() {
    let player = Player::new(None, false);
    let mut source = Item::default();
    source.set_count(1);
    let bag_quiver = ItemStorageTemplate {
        class_id: ItemClass::Quiver,
        subclass_id: ItemSubClassQuiver::AmmoPouch as u32,
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(300, 1)
    };
    let existing_quiver = Item::default();
    let stored_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        &existing_quiver,
        Some(&bag_quiver),
    )];
    let mut quiver_args = can_equip_args(NULL_SLOT, Some(&bag_quiver), Some(&source));
    quiver_args.stored_items = &stored_items;
    assert_eq!(
        player.can_equip_item(quiver_args).result,
        InventoryResult::OnlyOneAmmo
    );

    let polearm = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Polearm as u32,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(301, 1)
    };
    let mut polearm_args = can_equip_args(EQUIPMENT_SLOT_OFFHAND, Some(&polearm), Some(&source));
    polearm_args.can_dual_wield = true;
    assert_eq!(
        player.can_equip_item(polearm_args).result,
        InventoryResult::TwoHandSkillNotFound
    );

    let offhand_weapon = ItemStorageTemplate {
        inventory_type: InventoryType::WeaponOffhand,
        ..ItemStorageTemplate::regular_item(302, 1)
    };
    assert_eq!(
        player
            .can_equip_item(can_equip_args(
                EQUIPMENT_SLOT_OFFHAND,
                Some(&offhand_weapon),
                Some(&source)
            ))
            .result,
        InventoryResult::TwoHandSkillNotFound
    );

    let mut twohand_used =
        can_equip_args(EQUIPMENT_SLOT_OFFHAND, Some(&offhand_weapon), Some(&source));
    twohand_used.proto_always_allow_dual_wield = true;
    twohand_used.is_two_hand_used = true;
    assert_eq!(
        player.can_equip_item(twohand_used).result,
        InventoryResult::Equipped2handed
    );

    let twohand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        ..ItemStorageTemplate::regular_item(303, 1)
    };
    let offhand_item = Item::default();
    let equipped_offhand = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_OFFHAND,
        &offhand_item,
    )];
    let mut twohand_args = can_equip_args(NULL_SLOT, Some(&twohand), Some(&source));
    twohand_args.equipped_items = &equipped_offhand;
    twohand_args.offhand_can_store_result = InventoryResult::InvFull;
    assert_eq!(
        player.can_equip_item(twohand_args).result,
        InventoryResult::InvFull
    );

    twohand_args.swap = true;
    assert_eq!(
        player.can_equip_item(twohand_args).result,
        InventoryResult::CantSwap
    );
}
#[test]
fn can_unequip_item_matches_cpp_position_template_and_runtime_guards() {
    let player = Player::new(None, false);
    let armor = ItemStorageTemplate {
        inventory_type: InventoryType::Chest,
        ..ItemStorageTemplate::regular_item(400, 1)
    };
    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(401, 1)
    };
    let bag = ItemStorageTemplate {
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(402, 1)
    };
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            Some(&armor),
            Some(&source),
        )),
        InventoryResult::Ok
    );
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            Some(&armor),
            None,
        )),
        InventoryResult::Ok
    );
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            None,
            Some(&source),
        )),
        InventoryResult::ItemNotFound
    );

    source.set_loot_generated(true);
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            Some(&armor),
            Some(&source),
        )),
        InventoryResult::LootGone
    );
    source.set_loot_generated(false);

    let mut charmed = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    charmed.is_charmed = true;
    assert_eq!(
        player.can_unequip_item(charmed),
        InventoryResult::ClientLockedOut
    );

    let mut combat = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    combat.is_in_combat = true;
    assert_eq!(
        player.can_unequip_item(combat),
        InventoryResult::NotInCombat
    );

    let mut arena = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    arena.is_in_progress_arena = true;
    assert_eq!(
        player.can_unequip_item(arena),
        InventoryResult::NotDuringArenaMatch
    );

    let mut weapon_combat = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        Some(&weapon),
        Some(&source),
    );
    weapon_combat.is_in_combat = true;
    assert_eq!(player.can_unequip_item(weapon_combat), InventoryResult::Ok);

    let mut non_empty_bag = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
        Some(&bag),
        Some(&source),
    );
    non_empty_bag.source_is_not_empty_bag = true;
    assert_eq!(
        player.can_unequip_item(non_empty_bag),
        InventoryResult::DestroyNonemptyBag
    );

    non_empty_bag.swap = true;
    assert_eq!(player.can_unequip_item(non_empty_bag), InventoryResult::Ok);
}
#[test]
fn can_use_item_template_matches_cpp_access_requirement_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(500, 1);

    assert_eq!(
        player.can_use_item_template(can_use_template_args(None)),
        InventoryResult::ItemNotFound
    );

    let mut args = can_use_template_args(Some(&proto));
    args.internal_item = true;
    args.faction_horde = true;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.internal_item = false;
    args.team = TEAM_ALLIANCE_ID;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.faction_horde = false;
    args.faction_alliance = true;
    args.team = TEAM_HORDE_ID;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.faction_alliance = false;
    args.allowable_class_matches = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.allowable_class_matches = true;
    args.allowable_race_matches = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.allowable_race_matches = true;
    args.required_skill = 164;
    args.required_skill_rank = 75;
    args.required_skill_value = 0;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ProficiencyNeeded
    );

    args.required_skill_value = 50;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipSkill
    );

    args.required_skill_value = 75;
    args.required_spell = 1000;
    args.has_required_spell = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ProficiencyNeeded
    );
}
#[test]
fn can_use_item_template_matches_cpp_late_requirement_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(501, 1);
    let mut args = can_use_template_args(Some(&proto));

    args.player_level = 20;
    args.base_required_level = 30;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipLevelI
    );

    args.skip_required_level_check = true;
    assert_eq!(player.can_use_item_template(args), InventoryResult::Ok);

    args.skip_required_level_check = false;
    args.player_level = 70;
    args.holiday_id = 1;
    args.holiday_active = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ClientLockedOut
    );

    args.holiday_active = true;
    args.required_reputation_faction = 72;
    args.required_reputation_rank = 5;
    args.player_reputation_rank = 4;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipReputation
    );

    args.player_reputation_rank = 5;
    args.effect0_spell_id = Some(483);
    args.effect1_spell_id = Some(9000);
    args.has_effect1_spell = true;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::InternalBagError
    );

    args.has_effect1_spell = false;
    args.artifact_specialization = Some(2);
    args.primary_specialization = 1;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantUseItem
    );

    args.primary_specialization = 2;
    assert_eq!(player.can_use_item_template(args), InventoryResult::Ok);
}
#[test]
fn can_use_item_object_matches_cpp_item_level_and_template_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(600, 1);
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player.can_use_item(can_use_args(Some(&proto), None)),
        InventoryResult::ItemNotFound
    );

    let mut dead = can_use_args(Some(&proto), Some(&source));
    dead.is_alive = false;
    assert_eq!(player.can_use_item(dead), InventoryResult::PlayerDead);

    dead.not_loading = false;
    assert_eq!(player.can_use_item(dead), InventoryResult::Ok);

    assert_eq!(
        player.can_use_item(can_use_args(None, Some(&source))),
        InventoryResult::ItemNotFound
    );

    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(ObjectGuid::create_player(1, 99));
    assert_eq!(
        player.can_use_item(can_use_args(Some(&proto), Some(&source))),
        InventoryResult::NotOwner
    );
    source.remove_item_flag(ItemFieldFlags::SOULBOUND);

    let mut level = can_use_args(Some(&proto), Some(&source));
    level.player_level = 20;
    level.item_required_level = 30;
    level.template_args.internal_item = true;
    assert_eq!(player.can_use_item(level), InventoryResult::CantEquipLevelI);

    let mut template = can_use_args(Some(&proto), Some(&source));
    template.template_args.internal_item = true;
    assert_eq!(
        player.can_use_item(template),
        InventoryResult::CantEquipEver
    );
}
#[test]
fn can_use_item_object_matches_cpp_skill_and_heirloom_morph() {
    let player = Player::new(None, false);
    let armor = ItemStorageTemplate {
        class_id: ItemClass::Armor,
        inventory_type: InventoryType::Chest,
        ..ItemStorageTemplate::regular_item(601, 1)
    };
    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(602, 1)
    };
    let source = Item::default();

    let mut no_skill = can_use_args(Some(&weapon), Some(&source));
    no_skill.item_skill = SKILL_MAIL;
    no_skill.item_skill_value = 0;
    assert_eq!(
        player.can_use_item(no_skill),
        InventoryResult::ProficiencyNeeded
    );

    no_skill.item_skill_value = 1;
    assert_eq!(player.can_use_item(no_skill), InventoryResult::Ok);

    let mut hunter_mail = can_use_args(Some(&armor), Some(&source));
    hunter_mail.item_skill = SKILL_MAIL;
    hunter_mail.item_skill_value = 0;
    hunter_mail.has_item_skill = false;
    hunter_mail.proto_is_heirloom = true;
    hunter_mail.player_class = CLASS_HUNTER;
    assert_eq!(player.can_use_item(hunter_mail), InventoryResult::Ok);

    let mut warrior_mail = hunter_mail;
    warrior_mail.player_class = CLASS_WARRIOR;
    assert_eq!(
        player.can_use_item(warrior_mail),
        InventoryResult::ProficiencyNeeded
    );

    let mut paladin_plate = can_use_args(Some(&armor), Some(&source));
    paladin_plate.item_skill = SKILL_PLATE_MAIL;
    paladin_plate.item_skill_value = 0;
    paladin_plate.has_item_skill = false;
    paladin_plate.proto_is_heirloom = true;
    paladin_plate.player_class = CLASS_PALADIN;
    assert_eq!(player.can_use_item(paladin_plate), InventoryResult::Ok);
}
#[test]
fn can_equip_unique_item_template_matches_cpp_unique_entry_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(700, 1);
    assert_eq!(
        player.can_equip_unique_item_template(can_equip_unique_template_args(None)),
        InventoryResult::ItemNotFound
    );

    let mut equipped = Item::default();
    equipped.object_mut().set_entry(700);
    equipped.set_count(1);
    let equipped_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER1,
        &equipped,
        Some(&proto),
    )];

    let mut args = can_equip_unique_template_args(Some(&proto));
    args.unique_equippable = true;
    args.equipped_items = &equipped_items;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemUniqueEquippable
    );

    args.except_slot = EQUIPMENT_SLOT_FINGER1;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::Ok
    );

    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 700, 0)];
    args.equipped_items = &[];
    args.equipped_gems = &equipped_gems;
    args.except_slot = NULL_SLOT;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemUniqueEquippable
    );
}
#[test]
fn can_equip_unique_item_template_matches_cpp_limit_category_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate {
        item_limit_category: 10,
        ..ItemStorageTemplate::regular_item(701, 1)
    };
    let limit = ItemLimitCategoryTemplate {
        id: 10,
        quantity: 2,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    let mut equipped = Item::default();
    equipped.object_mut().set_entry(702);
    equipped.set_count(1);
    let equipped_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_TRINKET1,
        &equipped,
        Some(&proto),
    )];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 703, 10)];

    let mut args = can_equip_unique_template_args(Some(&proto));
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::NotEquippable
    );

    args.limit_category = Some(&limit);
    args.limit_count = 3;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
    );

    args.limit_count = 2;
    args.equipped_items = &equipped_items;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
    );

    args.equipped_items = &[];
    args.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxCountEquippedSocketed
    );

    args.except_slot = EQUIPMENT_SLOT_CHEST;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::Ok
    );
}
