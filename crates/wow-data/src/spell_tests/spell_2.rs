//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_implicit_target_effect_mask_normalizes_like_cpp_conditionmgr() {
    let spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![
            SpellEffectInfo {
                effect_index: 0,
                effect: 0,
                chain_targets: 0,
                implicit_target_1: 6,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 1,
                effect: 0,
                chain_targets: 0,
                implicit_target_1: 7,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 2,
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
                chain_targets: 0,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 3,
                effect: 0,
                chain_targets: 2,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            },
        ],
    };

    assert_eq!(
        spell.normalized_implicit_target_effect_mask_like_cpp(0b1111),
        0b1110
    );
    assert_eq!(
        spell.normalized_implicit_target_effect_mask_like_cpp(0b0001),
        0
    );
}
#[test]
fn spell_effect_detects_mounted_aura_like_cpp() {
    let mounted = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 11,
        effect_misc_value_1: 22,
        effect_misc_value_2: 33,
        ..Default::default()
    };
    let other_aura = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_HASTE_SPELLS,
        ..Default::default()
    };

    assert!(mounted.is_mounted_aura_like_cpp());
    assert!(!other_aura.is_mounted_aura_like_cpp());
    assert_eq!(mounted.effect_base_points, 11);
    assert_eq!(mounted.effect_misc_value_1, 22);
    assert_eq!(mounted.effect_misc_value_2, 33);
}
#[test]
fn spell_effect_calc_value_no_caster_rolls_die_sides_like_cpp() {
    let no_die = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 0,
        ..Default::default()
    };
    assert_eq!(
        no_die.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        10
    );

    let one_sided = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 1,
        ..Default::default()
    };
    assert_eq!(
        one_sided.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        11
    );

    let positive_range = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 7,
        ..Default::default()
    };
    assert_eq!(
        positive_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (1, 7));
            4
        }),
        14
    );

    let negative_range = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: -3,
        ..Default::default()
    };
    assert_eq!(
        negative_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (-3, 1));
            -2
        }),
        8
    );
}
#[test]
fn spell_effect_calc_value_no_caster_uses_cpp_double_accumulator() {
    let overflowing_int_add = SpellEffectInfo {
        effect_base_points: i32::MAX,
        effect_die_sides: 1,
        ..Default::default()
    };
    assert_eq!(
        overflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        i32::MAX
    );

    let underflowing_int_add = SpellEffectInfo {
        effect_base_points: i32::MIN,
        effect_die_sides: -1,
        ..Default::default()
    };
    assert_eq!(
        underflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (-1, 1));
            -1
        }),
        i32::MIN
    );
}
#[test]
fn spell_effect_constants_match_cpp_shared_defines() {
    // C++ `SharedDefines.h`: `SpellEffects` enum.
    assert_eq!(spell_effect_types::SPELL_EFFECT_NONE, 0);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE, 2);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT, 4);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AURA, 6);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE, 7);
    assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_DRAIN, 8);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEALTH_LEECH, 9);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL, 10);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BIND, 11);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL, 12);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_BASE, 13);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE, 14);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL, 15);
    assert_eq!(spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE, 16);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS, 19);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DODGE, 20);
    assert_eq!(spell_effect_types::SPELL_EFFECT_EVADE, 21);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PARRY, 22);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BLOCK, 23);
    assert_eq!(spell_effect_types::SPELL_EFFECT_WEAPON, 25);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DEFENSE, 26);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE, 30);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY, 35);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SPELL, 36);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE, 37);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LANGUAGE, 39);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DUAL_WIELD, 40);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SKILL, 118);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PLAY_MOVIE, 45);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SPAWN, 46);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TRADE_SKILL, 47);
    assert_eq!(spell_effect_types::SPELL_EFFECT_STEALTH, 48);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DETECT, 49);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT, 51);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT, 52);
    assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_BURN, 62);
    assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT, 63);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID, 65);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH, 67);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DISTRACT, 69);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PULL, 70);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL, 75);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK, 78);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SANCTUARY, 79);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_HOUSE, 81);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BIND_SIGHT, 82);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DUEL, 83);
    assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT, 90);
    assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT_ALL, 91);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_DESELECT, 93);
    assert_eq!(spell_effect_types::SPELL_EFFECT_INEBRIATE, 100);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DISMISS_PET, 102);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION, 103);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SURVEY, 105);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER, 106);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT, 107);
    assert_eq!(spell_effect_types::SPELL_EFFECT_112, 112);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK_ME, 114);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET, 119);
    assert_eq!(spell_effect_types::SPELL_EFFECT_122, 122);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT, 125);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND, 128);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, 129);
    assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT2, 134);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CALL_PET, 135);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_PCT, 136);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT, 137);
    assert_eq!(spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM, 163);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET, 168);
    assert_eq!(spell_effect_types::SPELL_EFFECT_175, 175);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
        177
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_178, 178);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER, 180);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER, 182);
    assert_eq!(spell_effect_types::SPELL_EFFECT_183, 183);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION_2, 184);
    assert_eq!(spell_effect_types::SPELL_EFFECT_185, 185);
    assert_eq!(spell_effect_types::SPELL_EFFECT_186, 186);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
        187
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
        188
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_LOOT, 189);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS, 190);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE, 191);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET, 192);
    assert_eq!(spell_effect_types::SPELL_EFFECT_START_PET_BATTLE, 193);
    assert_eq!(spell_effect_types::SPELL_EFFECT_194, 194);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON, 199);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
        202
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
        204
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_ALTER_ITEM, 206);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK, 207);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_REPUTATION, 208);
    assert_eq!(spell_effect_types::SPELL_EFFECT_209, 209);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
        210
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
        211
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_GARRISON, 214);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
        215
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT, 216);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON, 217);
    assert_eq!(spell_effect_types::SPELL_EFFECT_218, 218);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER, 220);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION, 221);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES, 223);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
        224
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL, 225);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET, 226);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
        227
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_228, 228);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY, 229);
    assert_eq!(spell_effect_types::SPELL_EFFECT_230, 230);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
        231
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_PHASE, 232);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
        233
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_234, 234);
    assert_eq!(spell_effect_types::SPELL_EFFECT_235, 235);
    assert_eq!(spell_effect_types::SPELL_EFFECT_INCREASE_SKILL, 238);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
        239
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER, 240);
    assert_eq!(spell_effect_types::SPELL_EFFECT_241, 241);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
        242
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY, 244);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM, 245);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
        246
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
        247
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT, 248);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM, 249);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT, 250);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
        251
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS, 252);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_HONOR, 253);
    assert_eq!(spell_effect_types::SPELL_EFFECT_JUMP_CHARGE, 254);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET, 255);
    assert_eq!(spell_effect_types::SPELL_EFFECT_256, 256);
    assert_eq!(spell_effect_types::SPELL_EFFECT_257, 257);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE, 258);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
        259
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET, 260);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SCRAP_ITEM, 261);
    assert_eq!(spell_effect_types::SPELL_EFFECT_262, 262);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPAIR_ITEM, 263);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_GEM, 264);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
        265
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
        266
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT, 268);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
        269
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_270, 270);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
        271
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_COVENANT, 272);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
        273
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_274, 274);
    assert_eq!(spell_effect_types::SPELL_EFFECT_275, 275);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
        276
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME, 277);
    assert_eq!(spell_effect_types::SPELL_EFFECT_278, 278);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT, 279);
    assert_eq!(spell_effect_types::SPELL_EFFECT_280, 280);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT, 281);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
        282
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN, 283);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2, 285);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
        286
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
        287
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ITEM, 288);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_AURA_STACKS, 289);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN, 290);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS, 291);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS_BY_CATEGORY,
        292
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES, 293);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_LOOT, 294);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM, 295);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM, 296);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM, 297);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
        298
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_299, 299);
    assert_eq!(spell_effect_types::SPELL_EFFECT_300, 300);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT, 301);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GATHERING, 302);
    assert_eq!(spell_effect_types::SPELL_EFFECT_305, 305);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS, 306);
    assert_eq!(spell_effect_types::SPELL_EFFECT_307, 307);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD, 308);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD, 309);
    assert_eq!(spell_effect_types::SPELL_EFFECT_310, 310);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED, 311);
    assert_eq!(spell_effect_types::SPELL_EFFECT_312, 312);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2, 313);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS, 314);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
        315
    );

    // C++ `SpellAuraDefines.h`: selected `AuraType` enum anchors.
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SPEED, 31);
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED, 32);
    assert_eq!(aura_types::SPELL_AURA_MOD_DECREASE_SPEED, 33);
    assert_eq!(aura_types::SPELL_AURA_MOD_SHAPESHIFT, 36);
    assert_eq!(aura_types::SPELL_AURA_TRANSFORM, 56);
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED, 58);
    assert_eq!(aura_types::SPELL_AURA_MOD_SCALE, 61);
    assert_eq!(aura_types::SPELL_AURA_MOUNTED, 78);
    assert_eq!(aura_types::SPELL_AURA_MOD_DETECT_RANGE, 91);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_ALWAYS, 129);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS, 130);
    assert_eq!(aura_types::SPELL_AURA_MOD_DETECTED_RANGE, 152);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK, 171);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK, 172);
    assert_eq!(aura_types::SPELL_AURA_FLY, 201);
    assert_eq!(
        aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
        207
    );
    assert_eq!(aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED, 191);
    assert_eq!(
        aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED,
        206
    );
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED, 208);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS, 209);
    assert_eq!(aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK, 211);
    assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED, 305);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL, 373);
    assert_eq!(aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT, 420);
    assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE, 437);
    assert_eq!(aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION, 499);

    // C++ `SharedDefines.h`: selected SpellAttr0 anchors.
    assert_eq!(attributes::SPELL_ATTR0_ONLY_INDOORS, 0x0000_4000);
    assert_eq!(attributes::SPELL_ATTR0_ONLY_OUTDOORS, 0x0000_8000);
    assert_eq!(attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED, 0x0100_0000);
}
#[test]
fn spell_effect_null_or_unused_classifier_matches_cpp_dispatch_subset() {
    for effect in [
        spell_effect_types::SPELL_EFFECT_NONE,
        spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT,
        spell_effect_types::SPELL_EFFECT_PORTAL,
        spell_effect_types::SPELL_EFFECT_RITUAL_BASE,
        spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE,
        spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL,
        spell_effect_types::SPELL_EFFECT_DODGE,
        spell_effect_types::SPELL_EFFECT_EVADE,
        spell_effect_types::SPELL_EFFECT_WEAPON,
        spell_effect_types::SPELL_EFFECT_DEFENSE,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE,
        spell_effect_types::SPELL_EFFECT_LANGUAGE,
        spell_effect_types::SPELL_EFFECT_SPAWN,
        spell_effect_types::SPELL_EFFECT_STEALTH,
        spell_effect_types::SPELL_EFFECT_DETECT,
        spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT,
        spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        spell_effect_types::SPELL_EFFECT_ATTACK,
        spell_effect_types::SPELL_EFFECT_CREATE_HOUSE,
        spell_effect_types::SPELL_EFFECT_BIND_SIGHT,
        spell_effect_types::SPELL_EFFECT_THREAT_ALL,
        spell_effect_types::SPELL_EFFECT_SURVEY,
        spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT,
        spell_effect_types::SPELL_EFFECT_112,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET,
        spell_effect_types::SPELL_EFFECT_122,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY,
        spell_effect_types::SPELL_EFFECT_CALL_PET,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_OWNER,
        spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM,
        spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET,
        spell_effect_types::SPELL_EFFECT_175,
        spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
        spell_effect_types::SPELL_EFFECT_178,
        spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER,
        spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER,
        spell_effect_types::SPELL_EFFECT_183,
        spell_effect_types::SPELL_EFFECT_REPUTATION_2,
        spell_effect_types::SPELL_EFFECT_185,
        spell_effect_types::SPELL_EFFECT_186,
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
        spell_effect_types::SPELL_EFFECT_LOOT,
        spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS,
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE,
        spell_effect_types::SPELL_EFFECT_START_PET_BATTLE,
        spell_effect_types::SPELL_EFFECT_194,
        spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
        spell_effect_types::SPELL_EFFECT_ALTER_ITEM,
        spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK,
        spell_effect_types::SPELL_EFFECT_SET_REPUTATION,
        spell_effect_types::SPELL_EFFECT_209,
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
        spell_effect_types::SPELL_EFFECT_CREATE_GARRISON,
        spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
        spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT,
        spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON,
        spell_effect_types::SPELL_EFFECT_218,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION,
        spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES,
        spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
        spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET,
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
        spell_effect_types::SPELL_EFFECT_228,
        spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY,
        spell_effect_types::SPELL_EFFECT_230,
        spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
        spell_effect_types::SPELL_EFFECT_REMOVE_PHASE,
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
        spell_effect_types::SPELL_EFFECT_234,
        spell_effect_types::SPELL_EFFECT_235,
        spell_effect_types::SPELL_EFFECT_INCREASE_SKILL,
        spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER,
        spell_effect_types::SPELL_EFFECT_241,
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
        spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY,
        spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
        spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT,
        spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM,
        spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT,
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
        spell_effect_types::SPELL_EFFECT_256,
        spell_effect_types::SPELL_EFFECT_257,
        spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE,
        spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET,
        spell_effect_types::SPELL_EFFECT_SCRAP_ITEM,
        spell_effect_types::SPELL_EFFECT_262,
        spell_effect_types::SPELL_EFFECT_REPAIR_ITEM,
        spell_effect_types::SPELL_EFFECT_REMOVE_GEM,
        spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
        spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
        spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT,
        spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
        spell_effect_types::SPELL_EFFECT_270,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
        spell_effect_types::SPELL_EFFECT_SET_COVENANT,
        spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
        spell_effect_types::SPELL_EFFECT_274,
        spell_effect_types::SPELL_EFFECT_275,
        spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME,
        spell_effect_types::SPELL_EFFECT_278,
        spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT,
        spell_effect_types::SPELL_EFFECT_280,
        spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT,
        spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
        spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN,
        spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2,
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
        spell_effect_types::SPELL_EFFECT_CRAFT_ITEM,
        spell_effect_types::SPELL_EFFECT_CRAFT_LOOT,
        spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM,
        spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM,
        spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM,
        spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
        spell_effect_types::SPELL_EFFECT_299,
        spell_effect_types::SPELL_EFFECT_300,
        spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT,
        spell_effect_types::SPELL_EFFECT_GATHERING,
        spell_effect_types::SPELL_EFFECT_305,
        spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS,
        spell_effect_types::SPELL_EFFECT_307,
        spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD,
        spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD,
        spell_effect_types::SPELL_EFFECT_310,
        spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED,
        spell_effect_types::SPELL_EFFECT_312,
        spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2,
        spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
    ] {
        assert!(
            spell_effect_types::is_cpp_null_or_unused_noop(effect),
            "effect {effect} should mirror C++ EffectNULL/EffectUnused"
        );
    }

    assert!(
        !spell_effect_types::is_cpp_null_or_unused_noop(3),
        "C++ SPELL_EFFECT_DUMMY dispatches EffectDummy and remains script-driven"
    );
    assert!(!spell_effect_types::is_cpp_null_or_unused_noop(
        spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE
    ));
    for real_handler_effect in [
        spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL,
        243,
        spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM,
        spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
        spell_effect_types::SPELL_EFFECT_JUMP_CHARGE,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
        284,
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
        289,
        290,
        291,
        292,
        293,
        303,
        304,
    ] {
        assert!(
            !spell_effect_types::is_cpp_null_or_unused_noop(real_handler_effect),
            "effect {real_handler_effect} has a real C++ dispatch handler in this range"
        );
    }
}
#[test]
fn spell_effect_detects_provide_spell_focus_aura_like_cpp() {
    let focus = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
        effect_misc_value_1: 181,
        ..Default::default()
    };
    let other_effect = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_HEAL,
        effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
        ..Default::default()
    };

    assert!(focus.is_provide_spell_focus_aura_like_cpp());
    assert!(!other_effect.is_provide_spell_focus_aura_like_cpp());
    assert_eq!(focus.effect_misc_value_1, 181);
}
#[test]
fn spell_effect_detects_focus_destination_implicit_targets_like_cpp() {
    let mut effect = SpellEffectInfo {
        implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
        ..Default::default()
    };
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_1 = 0;
    effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_2;
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_2 = 40;
    assert!(!effect.has_focus_destination_implicit_target_like_cpp());
}
#[test]
fn spell_target_position_store_loads_or_db_targets_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        710,
        SpellInfo {
            spell_id: 710,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 1,
                implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 710,
            effect_index: 1,
            target_map_id: 571,
            x: 100.0,
            y: 200.0,
            z: 30.0,
            orientation: Some(1.25),
        }],
        &spell_store,
        |map_id| map_id == 571,
    );

    assert_eq!(store.load_report_like_cpp().loaded, 1);
    assert_eq!(
        store.get(710, 1).map(|target| target.position),
        Some(wow_core::Position::new(100.0, 200.0, 30.0, 1.25))
    );
}
#[test]
fn spell_target_position_store_uses_effect_facing_when_orientation_is_null_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        9268,
        SpellInfo {
            spell_id: 9268,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                position_facing: 90.0,
                implicit_target_1: implicit_targets::TARGET_DEST_DB,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 9268,
            effect_index: 0,
            target_map_id: 0,
            x: -10.0,
            y: 20.0,
            z: 5.0,
            orientation: None,
        }],
        &spell_store,
        |map_id| map_id == 0,
    );

    let position = store.get(9268, 0).expect("target position").position;
    assert!((position.orientation - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
}
#[test]
fn spell_target_position_store_rejects_wrong_effect_target_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        711,
        SpellInfo {
            spell_id: 711,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 711,
            effect_index: 0,
            target_map_id: 571,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: Some(0.0),
        }],
        &spell_store,
        |_| true,
    );

    assert!(store.is_empty());
    assert_eq!(store.load_report_like_cpp().skipped_unsupported_target, 1);
}
