//! Tests for lifecycle store construction and entity projection.

use super::super::catalog::{GAMEOBJECT_DATA_CHEST_LOOT, GAMEOBJECT_TYPE_DOOR};
use super::*;

fn template(
    addon: Option<GameObjectTemplateAddonLifecycleRecordLikeCpp>,
) -> GameObjectTemplateLifecycleRecordLikeCpp {
    let mut data = [0_u32; MAX_GAMEOBJECT_DATA];
    data[3] = 333;
    GameObjectTemplateLifecycleRecordLikeCpp {
        entry: 42,
        go_type: 5,
        display_id: 700,
        name: "summoned object".to_string(),
        size: 1.25,
        data,
        content_tuning_id: 80,
        ai_name: "SmartGameObjectAI".to_string(),
        script_name: "scripted_go".to_string(),
        string_id: "string-id".to_string(),
        addon,
    }
}

fn template_with_data(
    entry: u32,
    go_type: u32,
    data: [u32; MAX_GAMEOBJECT_DATA],
) -> GameObjectTemplateLifecycleRecordLikeCpp {
    GameObjectTemplateLifecycleRecordLikeCpp {
        entry,
        go_type,
        display_id: 0,
        name: format!("go_{entry}"),
        size: 1.0,
        data,
        content_tuning_id: 0,
        ai_name: String::new(),
        script_name: String::new(),
        string_id: String::new(),
        addon: None,
    }
}

#[test]
fn gameobject_template_lifecycle_record_without_addon_uses_cpp_create_defaults() {
    let resolved = gameobject_template_lifecycle_record_like_cpp(&template(None));

    assert_eq!(resolved.entry, 42);
    assert_eq!(resolved.name, "summoned object");
    assert_eq!(resolved.go_type, 5);
    assert_eq!(resolved.display_id, 700);
    assert_eq!(resolved.scale, 1.25);
    assert_eq!(resolved.data[3], 333);
    assert_eq!(resolved.faction, 0);
    assert_eq!(resolved.flags, 0);
    assert_eq!(resolved.world_effect_id, 0);
    assert_eq!(resolved.anim_kit_id, 0);
    assert_eq!(resolved.level, 80);
    assert_eq!(resolved.percent_health, 100);
    assert_eq!(resolved.custom_param, 0);
}

#[test]
fn gameobject_template_lifecycle_record_applies_template_addon_like_cpp() {
    let addon = GameObjectTemplateAddonLifecycleRecordLikeCpp {
        entry: 42,
        faction: 35,
        flags: 0x20,
        world_effect_id: 77,
        anim_kit_id: 9,
    };

    let resolved = gameobject_template_lifecycle_record_like_cpp(&template(Some(addon)));

    assert_eq!(resolved.faction, 35);
    assert_eq!(resolved.flags, 0x20);
    assert_eq!(resolved.world_effect_id, 77);
    assert_eq!(resolved.anim_kit_id, 9);
    assert_eq!(resolved.level, 80);
}

#[test]
fn gameobject_for_quest_store_matches_cpp_template_type_filters() {
    let mut chest_quest_data = [0_u32; MAX_GAMEOBJECT_DATA];
    // C++ `GameObjectTemplate::chest.questID` is `Data8`.
    chest_quest_data[8] = 7000;
    let mut chest_loot_data = [0_u32; MAX_GAMEOBJECT_DATA];
    chest_loot_data[GAMEOBJECT_DATA_CHEST_LOOT] = 9000;
    let mut generic_data = [0_u32; MAX_GAMEOBJECT_DATA];
    generic_data[5] = 8000;
    let mut goober_data = [0_u32; MAX_GAMEOBJECT_DATA];
    goober_data[1] = 8100;
    let mut gathering_data = [0_u32; MAX_GAMEOBJECT_DATA];
    gathering_data[GAMEOBJECT_DATA_CHEST_LOOT] = 9100;

    let templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
        template_with_data(100, GAMEOBJECT_TYPE_QUESTGIVER, [0; MAX_GAMEOBJECT_DATA]),
        template_with_data(101, GAMEOBJECT_TYPE_CHEST, chest_quest_data),
        template_with_data(102, GAMEOBJECT_TYPE_CHEST, chest_loot_data),
        template_with_data(103, GAMEOBJECT_TYPE_GENERIC, generic_data),
        template_with_data(104, GAMEOBJECT_TYPE_GOOBER, goober_data),
        template_with_data(105, GAMEOBJECT_TYPE_GATHERING_NODE, gathering_data),
        template_with_data(106, GAMEOBJECT_TYPE_CHEST, [0; MAX_GAMEOBJECT_DATA]),
        template_with_data(107, GAMEOBJECT_TYPE_DOOR, [0; MAX_GAMEOBJECT_DATA]),
    ]);

    let store = GameObjectForQuestStoreLikeCpp::from_templates_like_cpp(&templates, |loot_id| {
        matches!(loot_id, 9000 | 9100)
    });

    for entry in [100, 101, 102, 103, 104, 105] {
        assert!(
            store.is_game_object_for_quests_like_cpp(entry),
            "entry {entry} should match C++ LoadGameObjectForQuests"
        );
    }
    assert!(!store.is_game_object_for_quests_like_cpp(106));
    assert!(!store.is_game_object_for_quests_like_cpp(107));
    assert_eq!(store.len(), 6);
}
