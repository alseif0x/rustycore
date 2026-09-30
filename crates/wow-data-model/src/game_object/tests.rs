//! Pure GameObjectTemplateData projection tests moved from entity scenarios.
//! These cover template catalog decoding and dispatch, not live GameObject state.

use super::*;


#[test]
fn gameobject_template_get_loot_id_matches_cpp_switch() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 44;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FISHING_HOLE, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data).get_loot_id_like_cpp(),
        44
    );
    assert_eq!(
        GameObjectTemplateData::new(2, data).get_loot_id_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_is_despawn_at_action_like_cpp_matches_switch() {
    let mut chest_data = [0; MAX_GAMEOBJECT_DATA];
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest_data)
            .is_despawn_at_action_like_cpp()
    );
    chest_data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest_data)
            .is_despawn_at_action_like_cpp()
    );

    let mut goober_data = [0; MAX_GAMEOBJECT_DATA];
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober_data)
            .is_despawn_at_action_like_cpp()
    );
    goober_data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] = 1;
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober_data)
            .is_despawn_at_action_like_cpp()
    );

    let mut generic_data = [0; MAX_GAMEOBJECT_DATA];
    generic_data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    generic_data[GAMEOBJECT_DATA_GOOBER_CONSUMABLE] = 1;
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_GENERIC, generic_data)
            .is_despawn_at_action_like_cpp()
    );
}

#[test]
fn gameobject_template_condition_id1_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 7),
        (GAMEOBJECT_TYPE_BUTTON, 9),
        (GAMEOBJECT_TYPE_QUESTGIVER, 10),
        (GAMEOBJECT_TYPE_CHEST, 17),
        (GAMEOBJECT_TYPE_GENERIC, 6),
        (GAMEOBJECT_TYPE_TRAP, 15),
        (GAMEOBJECT_TYPE_CHAIR, 4),
        (GAMEOBJECT_TYPE_SPELL_FOCUS, 8),
        (GAMEOBJECT_TYPE_TEXT, 4),
        (GAMEOBJECT_TYPE_GOOBER, 22),
        (GAMEOBJECT_TYPE_CAMERA, 4),
        (GAMEOBJECT_TYPE_RITUAL, 8),
        (GAMEOBJECT_TYPE_MAILBOX, 0),
        (GAMEOBJECT_TYPE_SPELLCASTER, 5),
        (GAMEOBJECT_TYPE_FLAGSTAND, 8),
        (GAMEOBJECT_TYPE_AURA_GENERATOR, 3),
        (GAMEOBJECT_TYPE_GUILD_BANK, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG, 4),
        (GAMEOBJECT_TYPE_ITEM_FORGE, 0),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 11),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 7_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_condition_id1_like_cpp(),
            7_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(25, data).get_condition_id1_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_interact_radius_override_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 12),
        (GAMEOBJECT_TYPE_BUTTON, 10),
        (GAMEOBJECT_TYPE_QUESTGIVER, 12),
        (GAMEOBJECT_TYPE_CHEST, 9),
        (GAMEOBJECT_TYPE_BINDER, 0),
        (GAMEOBJECT_TYPE_GENERIC, 9),
        (GAMEOBJECT_TYPE_TRAP, 21),
        (GAMEOBJECT_TYPE_CHAIR, 5),
        (GAMEOBJECT_TYPE_SPELL_FOCUS, 9),
        (GAMEOBJECT_TYPE_TEXT, 6),
        (GAMEOBJECT_TYPE_GOOBER, 33),
        (GAMEOBJECT_TYPE_AREADAMAGE, 8),
        (GAMEOBJECT_TYPE_CAMERA, 5),
        (GAMEOBJECT_TYPE_FISHING_NODE, 0),
        (GAMEOBJECT_TYPE_RITUAL, 9),
        (GAMEOBJECT_TYPE_MAILBOX, 1),
        (GAMEOBJECT_TYPE_SPELLCASTER, 8),
        (GAMEOBJECT_TYPE_MEETINGSTONE, 3),
        (GAMEOBJECT_TYPE_FLAGSTAND, 13),
        (GAMEOBJECT_TYPE_FISHING_HOLE, 5),
        (GAMEOBJECT_TYPE_FLAGDROP, 10),
        (GAMEOBJECT_TYPE_AURA_GENERATOR, 7),
        (GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, 11),
        (GAMEOBJECT_TYPE_BARBER_CHAIR, 3),
        (GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING, 27),
        (GAMEOBJECT_TYPE_GUILD_BANK, 1),
        (GAMEOBJECT_TYPE_NEW_FLAG, 14),
        (GAMEOBJECT_TYPE_NEW_FLAG_DROP, 2),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 24),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 10_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_interact_radius_override_like_cpp(),
            10_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAP_OBJECT, data)
            .get_interact_radius_override_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_lock_id_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 1),
        (GAMEOBJECT_TYPE_BUTTON, 1),
        (GAMEOBJECT_TYPE_QUESTGIVER, 0),
        (GAMEOBJECT_TYPE_CHEST, 0),
        (GAMEOBJECT_TYPE_TRAP, 0),
        (GAMEOBJECT_TYPE_GOOBER, 0),
        (GAMEOBJECT_TYPE_AREADAMAGE, 0),
        (GAMEOBJECT_TYPE_CAMERA, 0),
        (GAMEOBJECT_TYPE_FLAGSTAND, 0),
        (GAMEOBJECT_TYPE_FISHING_HOLE, 4),
        (GAMEOBJECT_TYPE_FLAGDROP, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG, 0),
        (GAMEOBJECT_TYPE_NEW_FLAG_DROP, 0),
        (GAMEOBJECT_TYPE_GATHERING_NODE, 3),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = 20_000 + go_type;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_lock_id_like_cpp(),
            20_000 + go_type
        );
    }

    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 999;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAP_OBJECT, data).get_lock_id_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_usable_mounted_matches_cpp_switch() {
    assert!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MAILBOX, [0; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );
    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_BARBER_CHAIR, [1; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );

    let cases = [
        (GAMEOBJECT_TYPE_QUESTGIVER, 8),
        (GAMEOBJECT_TYPE_TEXT, 3),
        (GAMEOBJECT_TYPE_GOOBER, 17),
        (GAMEOBJECT_TYPE_SPELLCASTER, 3),
        (GAMEOBJECT_TYPE_UI_LINK, 1),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        assert!(
            !GameObjectTemplateData::new(go_type, data).is_usable_mounted_like_cpp(),
            "type {go_type} should default to not usable mounted"
        );
        data[index] = 1;
        assert!(
            GameObjectTemplateData::new(go_type, data).is_usable_mounted_like_cpp(),
            "type {go_type} should read allowMounted from data[{index}]"
        );
    }

    assert!(
        !GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [1; MAX_GAMEOBJECT_DATA])
            .is_usable_mounted_like_cpp()
    );
}

#[test]
fn gameobject_template_no_damage_immune_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 3),
        (GAMEOBJECT_TYPE_BUTTON, 4),
        (GAMEOBJECT_TYPE_QUESTGIVER, 5),
        (GAMEOBJECT_TYPE_GOOBER, 11),
        (GAMEOBJECT_TYPE_FLAGSTAND, 5),
        (GAMEOBJECT_TYPE_FLAGDROP, 3),
    ];

    for (go_type, index) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_no_damage_immune_like_cpp(),
            0
        );
        data[index] = 7;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_no_damage_immune_like_cpp(),
            7
        );
    }

    let mut chest = [0; MAX_GAMEOBJECT_DATA];
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest).get_no_damage_immune_like_cpp(),
        1
    );
    chest[22] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, chest).get_no_damage_immune_like_cpp(),
        0
    );

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TEXT, [1; MAX_GAMEOBJECT_DATA])
            .get_no_damage_immune_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_cooldown_matches_cpp_switch() {
    let mut trap = [0; MAX_GAMEOBJECT_DATA];
    trap[5] = 12;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, trap).get_cooldown_like_cpp(),
        12
    );

    let mut goober = [0; MAX_GAMEOBJECT_DATA];
    goober[6] = 34;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, goober).get_cooldown_like_cpp(),
        34
    );

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [99; MAX_GAMEOBJECT_DATA])
            .get_cooldown_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_auto_close_time_matches_cpp_switch() {
    let cases = [
        (GAMEOBJECT_TYPE_DOOR, 2, 11),
        (GAMEOBJECT_TYPE_BUTTON, 2, 22),
        (GAMEOBJECT_TYPE_TRAP, 6, 33),
        (GAMEOBJECT_TYPE_GOOBER, 3, 44),
    ];

    for (go_type, index, value) in cases {
        let mut data = [0; MAX_GAMEOBJECT_DATA];
        data[index] = value;
        assert_eq!(
            GameObjectTemplateData::new(go_type, data).get_auto_close_time_like_cpp(),
            value
        );
    }

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, [99; MAX_GAMEOBJECT_DATA])
            .get_auto_close_time_like_cpp(),
        0
    );
}

#[test]
fn trap_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 20;
    data[3] = 123;
    data[4] = 1;
    data[5] = 9;
    data[7] = 3;
    data[14] = 1;
    data[20] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, data).trap_use_source_like_cpp(),
        Some(TrapUseSource {
            radius: 20,
            spell_id: 123,
            charges: 1,
            cooldown_secs: 9,
            start_delay_secs: 3,
            ignore_totems: true,
            check_all_units: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).trap_use_source_like_cpp(),
        None
    );
}

#[test]
fn chair_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 3;
    data[1] = 2;
    data[3] = 77;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHAIR, data).chair_use_source_like_cpp(),
        Some(ChairUseSource {
            chair_slots: 3,
            chair_height: 2,
            triggered_event_id: 77,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).chair_use_source_like_cpp(),
        None
    );
}

#[test]
fn barber_chair_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 2;
    data[2] = 345;
    data[4] = 9;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_BARBER_CHAIR, data)
            .barber_chair_use_source_like_cpp(),
        Some(BarberChairUseSource {
            chair_height: 2,
            sit_anim_kit: 345,
            customization_scope: 9,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHAIR, data).barber_chair_use_source_like_cpp(),
        None
    );
}

#[test]
fn ui_link_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 3;
    data[6] = 99;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_UI_LINK, data).ui_link_use_source_like_cpp(),
        Some(UiLinkUseSource { ui_link_type: 3 })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).ui_link_use_source_like_cpp(),
        None
    );
}

#[test]
fn item_forge_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 77;
    data[5] = 4;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_ITEM_FORGE, data)
            .item_forge_use_source_like_cpp(),
        Some(ItemForgeUseSource {
            condition_id: 77,
            forge_type: 4,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).item_forge_use_source_like_cpp(),
        None
    );
}

#[test]
fn capture_point_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 60_000;
    data[4] = 401;
    data[5] = 501;
    data[6] = 601;
    data[7] = 701;
    data[8] = 801;
    data[9] = 901;
    data[10] = 123;
    data[11] = 456;
    data[12] = 1200;
    data[13] = 1300;
    data[14] = 789;
    data[15] = 1500;
    data[16] = 1600;
    data[17] = 1700;
    data[18] = 1800;
    data[19] = 1900;
    data[20] = 2000;
    data[21] = 2100;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CAPTURE_POINT, data)
            .capture_point_use_source_like_cpp(),
        Some(CapturePointUseSource {
            capture_time_ms: 60_000,
            assault_broadcast_horde: 401,
            capture_broadcast_horde: 501,
            defended_broadcast_horde: 601,
            assault_broadcast_alliance: 701,
            capture_broadcast_alliance: 801,
            defended_broadcast_alliance: 901,
            world_state_id: 123,
            contested_event_horde: 456,
            capture_event_horde: 1200,
            defended_event_horde: 1300,
            contested_event_alliance: 789,
            capture_event_alliance: 1500,
            defended_event_alliance: 1600,
            spell_visual_ids: [1700, 1800, 1900, 2000, 2100],
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .capture_point_use_source_like_cpp(),
        None
    );
}

#[test]
fn battleground_flag_use_sources_use_cpp_data_indices() {
    let mut stand = [0; MAX_GAMEOBJECT_DATA];
    stand[1] = 111;
    stand[3] = 333;
    stand[4] = 444;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FLAGSTAND, stand)
            .flag_stand_use_source_like_cpp(),
        Some(FlagStandUseSource {
            pickup_spell_id: 111,
            return_aura_id: 333,
            return_spell_id: 444,
        })
    );

    let mut drop = [0; MAX_GAMEOBJECT_DATA];
    drop[1] = 222;
    drop[2] = 333;
    drop[6] = 666;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FLAGDROP, drop).flag_drop_use_source_like_cpp(),
        Some(FlagDropUseSource {
            event_id: 222,
            pickup_spell_id: 333,
            expire_duration_ms: 666,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, drop).flag_drop_use_source_like_cpp(),
        None
    );
}

#[test]
fn questgiver_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[3] = 42;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_QUESTGIVER, data)
            .questgiver_use_source_like_cpp(),
        Some(QuestgiverUseSource { gossip_id: 42 })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).questgiver_use_source_like_cpp(),
        None
    );
}

#[test]
fn ritual_and_meeting_stone_use_sources_use_cpp_data_indices() {
    let mut ritual = [0; MAX_GAMEOBJECT_DATA];
    ritual[0] = 3;
    ritual[1] = 62330;
    ritual[2] = 111;
    ritual[3] = 1;
    ritual[4] = 222;
    ritual[5] = 2;
    ritual[6] = 1;
    ritual[7] = 1;
    ritual[10] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_RITUAL, ritual).ritual_use_source_like_cpp(),
        Some(RitualUseSource {
            casters_required: 3,
            spell_id: 62330,
            anim_spell_id: 111,
            persistent: true,
            caster_target_spell_id: 222,
            caster_target_spell_targets: 2,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: true,
        })
    );

    let mut meeting_stone = [0; MAX_GAMEOBJECT_DATA];
    meeting_stone[2] = 345;
    meeting_stone[4] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_MEETINGSTONE, meeting_stone)
            .meeting_stone_use_source_like_cpp(),
        Some(MeetingStoneUseSource {
            area_id: 345,
            prevent_unfriendly_outside_instances: true,
            content_tuning_id: 0,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, ritual).ritual_use_source_like_cpp(),
        None
    );
}

#[test]
fn new_flag_use_sources_use_cpp_data_indices() {
    let mut flag = [0; MAX_GAMEOBJECT_DATA];
    flag[1] = 111;
    flag[7] = 777;
    flag[8] = 888;
    flag[9] = 999;
    flag[10] = u32::MAX;
    flag[11] = 1111;
    flag[12] = 1;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_NEW_FLAG, flag).new_flag_use_source_like_cpp(),
        Some(NewFlagUseSource {
            pickup_spell_id: 111,
            expire_duration_ms: 777,
            respawn_time_ms: 888,
            flag_drop_entry: 999,
            exclusive_category: -1,
            world_state_id: 1111,
            return_on_defender_interact: true,
        })
    );

    let mut drop = [0; MAX_GAMEOBJECT_DATA];
    drop[1] = 222;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_NEW_FLAG_DROP, drop)
            .new_flag_drop_use_source_like_cpp(),
        Some(NewFlagDropUseSource {
            spawn_vignette_id: 222,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, drop).new_flag_use_source_like_cpp(),
        None
    );
}

#[test]
fn spellcaster_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 1234;
    data[1] = 7;
    data[2] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELLCASTER, data)
            .spellcaster_use_source_like_cpp(),
        Some(SpellcasterUseSource {
            spell_id: 1234,
            charges: 7,
            party_only: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spellcaster_use_source_like_cpp(),
        None
    );
}

#[test]
fn spell_focus_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_SPELL_FOCUS_TYPE] = 181;
    data[GAMEOBJECT_DATA_SPELL_FOCUS_RADIUS] = 12;
    data[GAMEOBJECT_DATA_SPELL_FOCUS_LINKED_TRAP] = 987;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .spell_focus_use_source_like_cpp(),
        Some(SpellFocusUseSource {
            focus_type: 181,
            radius: 12,
            linked_trap_entry: 987,
        })
    );

    let mut ui_link_data = [0; MAX_GAMEOBJECT_DATA];
    ui_link_data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_TYPE] = 182;
    ui_link_data[GAMEOBJECT_DATA_UI_LINK_SPELL_FOCUS_RADIUS] = 34;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_UI_LINK, ui_link_data)
            .spell_focus_use_source_like_cpp(),
        Some(SpellFocusUseSource {
            focus_type: 182,
            radius: 34,
            linked_trap_entry: 0,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spell_focus_use_source_like_cpp(),
        None
    );
}

#[test]
fn guard_post_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 4321;
    data[1] = 5;
    data[2] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GUARDPOST, data)
            .guard_post_use_source_like_cpp(),
        Some(GuardPostUseSource {
            creature_id: 4321,
            charges: 5,
            prefer_only_if_in_line_of_sight: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).guard_post_use_source_like_cpp(),
        None
    );
}

#[test]
fn spell_focus_linked_trap_uses_cpp_data_index() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 987;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .spell_focus_linked_trap_like_cpp(),
        987
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).spell_focus_linked_trap_like_cpp(),
        0
    );
}

#[test]
fn gameobject_template_linked_gameobject_entry_dispatches_cpp_sources() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_BUTTON_LINKED_TRAP] = 103;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_BUTTON, data)
            .get_linked_gameobject_entry_like_cpp(),
        103
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[2] = 802;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_SPELL_FOCUS, data)
            .get_linked_gameobject_entry_like_cpp(),
        802
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[12] = 1012;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, data)
            .get_linked_gameobject_entry_like_cpp(),
        1012
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP] = 307;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .get_linked_gameobject_entry_like_cpp(),
        307
    );

    data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP] = 5020;
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data)
            .get_linked_gameobject_entry_like_cpp(),
        5020
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_TRAP, data)
            .get_linked_gameobject_entry_like_cpp(),
        0
    );
}

#[test]
fn camera_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[1] = 1234;
    data[2] = 55;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CAMERA, data).camera_use_source_like_cpp(),
        Some(CameraUseSource {
            cinematic_id: 1234,
            event_id: 55,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).camera_use_source_like_cpp(),
        None
    );
}

#[test]
fn goober_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[0] = 99;
    data[1] = 101;
    data[2] = 202;
    data[3] = 303;
    data[4] = 4;
    data[5] = 1;
    data[7] = 707;
    data[10] = 1010;
    data[12] = 1212;
    data[19] = 1919;
    data[20] = 1;
    data[23] = 1;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GOOBER, data).goober_use_source_like_cpp(),
        Some(GooberUseSource {
            lock_id: 99,
            quest_id: 101,
            event_id: 202,
            auto_close_ms: 303,
            custom_anim: 4,
            consumable: true,
            page_id: 707,
            spell_id: 1010,
            linked_trap_entry: 1212,
            gossip_id: 1919,
            allow_multi_interact: true,
            player_cast: true,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data).goober_use_source_like_cpp(),
        None
    );
}

#[test]
fn chest_loot_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 10;
    data[GAMEOBJECT_DATA_CHEST_RESTOCK_TIME] = 60;
    data[GAMEOBJECT_DATA_CHEST_CONSUMABLE] = 1;
    data[GAMEOBJECT_DATA_CHEST_TRIGGERED_EVENT] = 40;
    data[GAMEOBJECT_DATA_CHEST_LINKED_TRAP] = 50;
    data[GAMEOBJECT_DATA_CHEST_QUEST_ID] = 9999;
    data[GAMEOBJECT_DATA_CHEST_USE_GROUP_LOOT_RULES] = 1;
    data[GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER] = 1234;
    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 20;
    data[GAMEOBJECT_DATA_CHEST_PUSH_LOOT] = 30;

    let source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");

    assert_eq!(
        source,
        GameObjectLootSource {
            loot_id: 10,
            use_group_loot_rules: true,
            dungeon_encounter_id: 1234,
            personal_loot_id: 20,
            push_loot_id: 30,
            triggered_event_id: 40,
            linked_trap_entry: 50,
            chest_restock_time_secs: 60,
            chest_consumable: true,
            chest_quest_id: 9999,
        }
    );
    assert!(!source.is_empty());
    assert_eq!(source.open_loot_id_like_cpp(), 10);
    assert!(source.has_open_loot_like_cpp());
    assert!(!source.uses_personal_loot_like_cpp());
    assert!(!source.is_personal_encounter_loot_like_cpp());
    assert!(!source.should_autostore_push_loot_like_cpp());

    // Verify index 8 is read from the correct data slot: zero it and confirm field resets.
    data[GAMEOBJECT_DATA_CHEST_QUEST_ID] = 0;
    let no_quest_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert_eq!(no_quest_source.chest_quest_id, 0);

    data[GAMEOBJECT_DATA_CHEST_LOOT] = 0;
    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 0;
    let push_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert!(!push_source.is_empty());
    assert!(!push_source.has_open_loot_like_cpp());
    assert!(!push_source.uses_personal_loot_like_cpp());
    assert!(!push_source.is_personal_encounter_loot_like_cpp());
    assert!(push_source.should_autostore_push_loot_like_cpp());

    data[GAMEOBJECT_DATA_CHEST_PERSONAL_LOOT] = 25;
    let personal_encounter_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert_eq!(personal_encounter_source.open_loot_id_like_cpp(), 25);
    assert!(personal_encounter_source.has_open_loot_like_cpp());
    assert!(personal_encounter_source.uses_personal_loot_like_cpp());
    assert!(personal_encounter_source.is_personal_encounter_loot_like_cpp());

    data[GAMEOBJECT_DATA_CHEST_DUNGEON_ENCOUNTER] = 0;
    let personal_non_encounter_source = GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
        .chest_loot_source_like_cpp()
        .expect("chest templates expose a chest loot source");
    assert!(personal_non_encounter_source.uses_personal_loot_like_cpp());
    assert!(!personal_non_encounter_source.is_personal_encounter_loot_like_cpp());
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_FISHING_HOLE, data)
            .chest_loot_source_like_cpp(),
        None
    );
}

#[test]
fn gathering_node_use_source_uses_cpp_data_indices() {
    let mut data = [0; MAX_GAMEOBJECT_DATA];
    data[GAMEOBJECT_DATA_CHEST_LOOT] = 10;
    data[GAMEOBJECT_DATA_GATHERING_NODE_DESPAWN_DELAY] = 15;
    data[GAMEOBJECT_DATA_GATHERING_NODE_TRIGGERED_EVENT] = 20;
    data[GAMEOBJECT_DATA_GATHERING_NODE_XP_DIFFICULTY] = 5;
    data[GAMEOBJECT_DATA_GATHERING_NODE_SPELL] = 30;
    data[GAMEOBJECT_DATA_GATHERING_NODE_MAX_LOOTS] = 3;
    data[GAMEOBJECT_DATA_GATHERING_NODE_LINKED_TRAP] = 40;

    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_GATHERING_NODE, data)
            .gathering_node_use_source_like_cpp(),
        Some(GatheringNodeUseSource {
            loot_id: 10,
            despawn_delay_secs: 15,
            triggered_event_id: 20,
            xp_difficulty: 5,
            spell_id: 30,
            max_loots: 3,
            linked_trap_entry: 40,
        })
    );
    assert_eq!(
        GameObjectTemplateData::new(GAMEOBJECT_TYPE_CHEST, data)
            .gathering_node_use_source_like_cpp(),
        None
    );
}
