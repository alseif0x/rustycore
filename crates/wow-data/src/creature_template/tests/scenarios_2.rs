//! Creature template regressions, part 2 of 2.
//!
//! Moved out of the creature_template.rs root under #664; every test is unchanged.

use super::*;

#[test]
fn creature_addon_store_uses_spawn_addon_before_template_like_cpp() {
    let spawn = CreatureAddonRowLikeCpp {
        mount: 1234,
        stand_state: UnitStandStateType::Kneel as u8,
        pvp_flags: UnitPvpFlags::PVP.bits(),
        emote: 77,
        ai_anim_kit: 11,
        movement_anim_kit: 22,
        melee_anim_kit: 33,
        ..addon_row(44)
    };
    let template = CreatureAddonRowLikeCpp {
        mount: 5678,
        stand_state: UnitStandStateType::Sleep as u8,
        pvp_flags: UnitPvpFlags::SANCTUARY.bits(),
        emote: 88,
        ai_anim_kit: 44,
        movement_anim_kit: 55,
        melee_anim_kit: 66,
        ..addon_row(1001)
    };

    let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
        [spawn],
        [template],
        |spawn_id| spawn_id == 44,
        |entry| entry == 1001,
        |display_id| matches!(display_id, 1234 | 5678),
        |emote| matches!(emote, 77 | 88),
        |anim_kit_id| matches!(anim_kit_id, 11 | 22 | 33 | 44 | 55 | 66),
        |_| false,
        |_| false,
        |_| 0,
        |_| 0,
        |_| 0,
    );

    assert_eq!(
        store.get_for_creature_like_cpp(44, 1001),
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 0,
            mount_display_id: 1234,
            stand_state: UnitStandStateType::Kneel,
            vis_flags: 0,
            anim_tier: 0,
            sheath_state: SheathState::Unarmed,
            pvp_flags: UnitPvpFlags::PVP,
            emote: 77,
            ai_anim_kit_id: 11,
            movement_anim_kit_id: 22,
            melee_anim_kit_id: 33,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        }),
        "C++ Creature::GetCreatureAddon checks spawn id before template entry"
    );
    assert_eq!(
        store.get_for_creature_like_cpp(0, 1001),
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 0,
            mount_display_id: 5678,
            stand_state: UnitStandStateType::Sleep,
            vis_flags: 0,
            anim_tier: 0,
            sheath_state: SheathState::Unarmed,
            pvp_flags: UnitPvpFlags::SANCTUARY,
            emote: 88,
            ai_anim_kit_id: 44,
            movement_anim_kit_id: 55,
            melee_anim_kit_id: 66,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        })
    );
}

#[test]
fn creature_addon_store_normalizes_supported_fields_like_cpp() {
    let row = CreatureAddonRowLikeCpp {
        mount: 9999,
        stand_state: UnitStandStateType::Max as u8,
        anim_tier: MAX_ANIM_TIER_LIKE_CPP,
        vis_flags: 0xff,
        sheath_state: MAX_SHEATH_STATE_LIKE_CPP,
        pvp_flags: 0xff,
        emote: 333,
        ai_anim_kit: 11,
        movement_anim_kit: 22,
        melee_anim_kit: 33,
        visibility_distance_type: VisibilityDistanceTypeLikeCpp::MAX_LIKE_CPP,
        ..addon_row(44)
    };

    let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
        [row],
        [],
        |spawn_id| spawn_id == 44,
        |_| false,
        |_| false,
        |_| false,
        |_| false,
        |_| false,
        |_| false,
        |_| 0,
        |_| 0,
        |_| 0,
    );

    assert_eq!(
        store.get_for_creature_like_cpp(44, 1001),
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 0,
            mount_display_id: 0,
            stand_state: UnitStandStateType::Stand,
            vis_flags: 0xff,
            anim_tier: 0,
            sheath_state: SheathState::Unarmed,
            pvp_flags: UnitPvpFlags::from_bits_retain(0xff),
            emote: 0,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        }),
        "C++ invalid mount/emote/stand/anim/sheath/anim-kit/visibility rows are truncated; VisFlags/PvPFlags cover the full byte"
    );
}

#[test]
fn creature_addon_store_normalizes_auras_like_cpp() {
    let row = CreatureAddonRowLikeCpp {
        auras: "100 bad 200 100 300 400 500".to_string(),
        ..addon_row(44)
    };

    let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
        [row],
        [],
        |spawn_id| spawn_id == 44,
        |_| false,
        |_| true,
        |_| true,
        |_| true,
        |spell_id| matches!(spell_id, 100 | 200 | 300 | 400),
        |spell_id| spell_id == 400,
        |spell_id| if spell_id == 300 { 5_000 } else { 0 },
        |spell_id| {
            if matches!(spell_id, 100 | 200 | 400) {
                1
            } else {
                0
            }
        },
        |_| AFLAG_NOCASTER_LIKE_CPP | AFLAG_POSITIVE_LIKE_CPP | AFLAG_CANCELABLE_LIKE_CPP,
    );

    assert_eq!(
        store
            .get_for_creature_like_cpp(44, 1001)
            .map(|addon| addon.auras),
        Some(vec![100, 200, 400]),
        "C++ addon loading skips malformed, missing, duplicate, and temporary auras; control-vehicle auras log but remain stored"
    );
}

#[test]
fn creature_addon_store_mutates_waypoint_spawn_without_path_to_idle_like_cpp() {
    let spawn_without_path = CreatureAddonRowLikeCpp {
        path_id: 0,
        ..addon_row(44)
    };
    let spawn_with_path = CreatureAddonRowLikeCpp {
        path_id: 9001,
        ..addon_row(45)
    };
    let template_without_path = CreatureAddonRowLikeCpp {
        path_id: 0,
        ..addon_row(1001)
    };

    let store = CreatureAddonStoreLikeCpp::from_rows_like_cpp(
        [spawn_without_path, spawn_with_path],
        [template_without_path],
        |spawn_id| matches!(spawn_id, 44 | 45),
        |entry| entry == 1001,
        |_| true,
        |_| true,
        |_| true,
        |_| false,
        |_| false,
        |_| 0,
        |_| 0,
        |_| 0,
    );

    assert_eq!(
        store.movement_type_after_spawn_addon_load_like_cpp(44, WAYPOINT_MOTION_TYPE_LIKE_CPP),
        IDLE_MOTION_TYPE_LIKE_CPP,
        "C++ LoadCreatureAddons mutates spawn WAYPOINT_MOTION_TYPE to IDLE_MOTION_TYPE when spawn PathId is zero"
    );
    assert_eq!(
        store.movement_type_after_spawn_addon_load_like_cpp(45, WAYPOINT_MOTION_TYPE_LIKE_CPP),
        WAYPOINT_MOTION_TYPE_LIKE_CPP,
        "non-zero spawn PathId preserves C++ waypoint movement"
    );
    assert_eq!(
        store.movement_type_after_spawn_addon_load_like_cpp(0, WAYPOINT_MOTION_TYPE_LIKE_CPP),
        WAYPOINT_MOTION_TYPE_LIKE_CPP,
        "template addon PathId never triggers the spawn CreatureData movementType mutation"
    );
    assert_eq!(
        store
            .get_for_creature_like_cpp(45, 1001)
            .map(|addon| addon.path_id),
        Some(9001),
        "PathId is retained for the future path runtime seam"
    );
}
