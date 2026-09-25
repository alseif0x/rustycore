use super::*;

#[tokio::test]
async fn represented_form_change_refreshes_item_equip_spells_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 78);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    let item_id = 30_100_u32;
    let item_guid = ObjectGuid::create_item(1, 917);
    let equip_spell_id = 90_997_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        equip_spell_id,
        represented_aura_spell_like_cpp(
            equip_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            1,
            10,
        ),
    );
    // C++ `SpellInfo::CheckShapeshift`: the equip spell is allowed in cat form.
    spell_store.insert_spell_shapeshift_masks_like_cpp(equip_spell_id, 1 << 0, 0);
    let (shapeshift_spell_id, _form_id) =
        represented_cat_form_fixture_like_cpp(&mut session, player_guid, spell_store);
    session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([
        wow_data::ItemEffectEntry {
            id: 1,
            legacy_slot_index: 0,
            trigger_type: 1,
            charges: 0,
            cooldown_msec: 0,
            category_cooldown_msec: 0,
            spell_category_id: 0,
            spell_id: equip_spell_id,
            chr_specialization_id: 0,
            parent_item_id: item_id,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        item_id,
        InventoryType::Chest,
    );

    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(equip_spell_id),
        Some(false)
    );

    session
        .apply_aura(shapeshift_spell_id, player_guid, 30_000, 1)
        .expect("apply cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(equip_spell_id),
        Some(true),
        "C++ ApplyItemEquipSpell(item, true, true) adds the now-fitting equip spell"
    );

    session.remove_aura(0).expect("remove cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(equip_spell_id),
        Some(false),
        "C++ ApplyItemEquipSpell(item, false, true) removes the stale equip spell"
    );
}

#[tokio::test]
async fn represented_form_change_refreshes_item_set_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 79);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    let chest_guid = ObjectGuid::create_item(1, 918);
    let hands_guid = ObjectGuid::create_item(1, 919);
    let set_spell_id = 90_998_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        set_spell_id,
        represented_aura_spell_like_cpp(
            set_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            1,
            10,
        ),
    );
    spell_store.insert_spell_shapeshift_masks_like_cpp(set_spell_id, 1 << 0, 0);
    let (shapeshift_spell_id, _form_id) =
        represented_cat_form_fixture_like_cpp(&mut session, player_guid, spell_store);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 708,
        name: "Form Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 111,
            1 => 112,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 40,
            chr_spec_id: 0,
            spell_id: set_spell_id as u32,
            threshold: 2,
            item_set_id: 708,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        111,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        112,
        InventoryType::Hands,
    );
    let _ = session.record_represented_items_set_item_like_cpp(chest_guid, true);
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    // The set bonus is stance-gated, so the initial equip pass cannot apply it.
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(set_spell_id),
        Some(false)
    );

    session
        .apply_aura(shapeshift_spell_id, player_guid, 30_000, 1)
        .expect("apply cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(set_spell_id),
        Some(true),
        "C++ ApplyEquipSpell(itemSet, true, formChange) applies the now-fitting set aura"
    );

    session.remove_aura(0).expect("remove cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(set_spell_id),
        Some(false),
        "the set aura is removed once the form no longer fits"
    );
}

#[tokio::test]
async fn represented_form_change_applies_and_removes_boost_spells_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 80);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    // C++ `HandleShapeshiftBoosts` casts spell 3025 for cat form.
    let boost_spell_id = 3_025_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        boost_spell_id,
        represented_aura_spell_like_cpp(
            boost_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            1,
            25,
        ),
    );
    let (shapeshift_spell_id, _form_id) =
        represented_cat_form_fixture_like_cpp(&mut session, player_guid, spell_store);

    session
        .apply_aura(shapeshift_spell_id, player_guid, 30_000, 1)
        .expect("apply cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(boost_spell_id),
        Some(true),
        "C++ HandleShapeshiftBoosts casts the form's hardcoded boost spell"
    );

    session.remove_aura(0).expect("remove cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(boost_spell_id),
        Some(false),
        "C++ RemoveOwnedAura drops the boost when the form is lost"
    );
}

#[tokio::test]
async fn represented_form_change_applies_and_sweeps_stance_passives_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 81);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    let passive_spell_id = 90_999_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        passive_spell_id,
        represented_aura_spell_like_cpp(
            passive_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            1,
            25,
        ),
    );
    // C++ `SpellInfo::IsPassive` and a `Stances` mask that admits cat form.
    let mut attributes = [0_u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(passive_spell_id, attributes);
    spell_store.insert_spell_shapeshift_masks_like_cpp(passive_spell_id, 1 << 0, 0);
    let (shapeshift_spell_id, _form_id) =
        represented_cat_form_fixture_like_cpp(&mut session, player_guid, spell_store);
    session.set_known_spells_like_cpp(vec![passive_spell_id]);

    session
        .apply_aura(shapeshift_spell_id, player_guid, 30_000, 1)
        .expect("apply cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(passive_spell_id),
        Some(true),
        "C++ HandleShapeshiftBoosts casts every known passive whose Stances admit the form"
    );

    session.remove_aura(0).expect("remove cat form aura");
    assert_eq!(
        session.player_has_visible_aura_spell_like_cpp(passive_spell_id),
        Some(false),
        "C++ Aura::IsRemovedOnShapeLost sweeps the self-cast aura when the form is lost"
    );
}

#[tokio::test]
async fn represented_shapeshift_form_selects_display_power_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 82);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    let mut spell_store = wow_data::SpellStore::new();
    // Cat form (1) and bear form (5) shapeshift auras.
    let cat_spell_id = 90_996_i32;
    let bear_spell_id = 91_000_i32;
    spell_store.insert(
        cat_spell_id,
        represented_aura_spell_like_cpp(
            cat_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
            1,
            0,
        ),
    );
    spell_store.insert(
        bear_spell_id,
        represented_aura_spell_like_cpp(
            bear_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
            5,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([
            wow_data::SpellShapeshiftFormEntry {
                id: 1,
                name: "Cat Form".to_string(),
                creature_type: 0,
                flags: 0,
                attack_icon_file_id: 0,
                bonus_action_bar: 0,
                combat_round_time: 1_000,
                damage_variance: 0.0,
                mount_type_id: 0,
                creature_display_id: [0; 4],
                preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
            },
            wow_data::SpellShapeshiftFormEntry {
                id: 5,
                name: "Bear Form".to_string(),
                creature_type: 0,
                flags: 0,
                attack_icon_file_id: 0,
                bonus_action_bar: 0,
                combat_round_time: 1_000,
                damage_variance: 0.0,
                mount_type_id: 0,
                creature_display_id: [0; 4],
                preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
            },
        ]),
    ));

    let display_power = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().data().display_power)
            .expect("canonical player")
    };
    // No `ChrClasses` row is loaded, so C++ falls back to `POWER_MANA`.
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::Mana as u8
    );

    session
        .apply_aura(cat_spell_id, player_guid, 30_000, 1)
        .expect("apply cat form aura");
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::Energy as u8,
        "C++ CalculateDisplayPowerType maps FORM_CAT_FORM to POWER_ENERGY"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject),
        "the changed UNIT_FIELD_DISPLAYPOWER is published"
    );

    session.remove_aura(0).expect("remove cat form aura");
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::Mana as u8
    );

    session
        .apply_aura(bear_spell_id, player_guid, 30_000, 1)
        .expect("apply bear form aura");
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::Rage as u8,
        "C++ CalculateDisplayPowerType maps FORM_BEAR_FORM to POWER_RAGE"
    );
    session.remove_aura(0).expect("remove bear form aura");
}

#[tokio::test]
async fn represented_power_display_aura_selects_display_power_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 83);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);

    let aura_spell_id = 91_001_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        aura_spell_id,
        represented_aura_spell_like_cpp(
            aura_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_DISPLAY,
            wow_constants::PowerType::RunicPower as i32,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    let display_power = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().data().display_power)
            .expect("canonical player")
    };
    session
        .apply_aura(aura_spell_id, player_guid, 30_000, 1)
        .expect("apply power display aura");
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::RunicPower as u8,
        "C++ CalculateDisplayPowerType reads the aura's GetMiscValue outside a form"
    );

    session.remove_aura(0).expect("remove power display aura");
    assert_eq!(
        display_power(&session),
        wow_constants::PowerType::Mana as u8
    );
}
