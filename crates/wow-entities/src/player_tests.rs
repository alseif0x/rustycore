//! Behaviour tests for [`super`].
//!
//! Extracted from `player.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::*;

use wow_constants::{
    BagFamilyMask, InventoryResult, InventoryType, ItemBondingType, ItemClass, ItemContext,
    ItemFieldFlags, ItemSubClassContainer, ItemSubclassProfession,
};
use wow_core::guid::HighGuid;

fn can_store_args<'a>(
    bag: u8,
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    count: u32,
) -> CanStoreItemArgs<'a> {
    CanStoreItemArgs {
        bag,
        slot,
        entry: proto.map_or(0, |proto| proto.entry),
        count,
        proto,
        source_item: None,
        source_is_not_empty_bag: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn item_with_guid_entry(low: i64, entry: u32) -> Item {
    let mut item = Item::default();
    item.object_mut().create(ObjectGuid::create_item(1, low));
    item.object_mut().set_entry(entry);
    item
}

struct StubPowerResolver;

impl PlayerPowerIndexResolver for StubPowerResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize> {
        if class_id != CLASS_PALADIN {
            return None;
        }
        match power {
            PowerType::Mana => Some(0),
            PowerType::Energy => Some(3),
            PowerType::ComboPoints => Some(9),
            PowerType::AlternateMount => Some(MAX_POWERS_PER_CLASS),
            _ => None,
        }
    }
}

fn lifecycle_create_record() -> PlayerCreateLifecycleRecord {
    PlayerCreateLifecycleRecord {
        guid: ObjectGuid::create_player(1, 42),
        name: "Lifecycle".to_string(),
        race: 1,
        class_id: CLASS_PALADIN,
        gender: Gender::Female,
        level: 12,
        xp: 345,
        money: 678,
        inventory_slot_count: INVENTORY_DEFAULT_SIZE,
        bank_bag_slot_count: 2,
        map_id: 571,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        max_health: 1000,
        health: 750,
        powers: vec![
            PlayerLifecyclePower::new(PowerType::Mana, 400, 900),
            PlayerLifecyclePower::new(PowerType::Energy, 40, 100),
            PlayerLifecyclePower::new(PowerType::Focus, 99, 100),
        ],
        display_power: PowerType::Mana,
        faction_template: Some(35),
        display_id: Some(1234),
        player_flags: 0x10,
        player_flags_ex: 0x20,
        extra_flags: 0x40,
        create_time: Some(1_700_000_000),
        create_mode: Some(0),
        played_time_total: 11,
        played_time_level: 7,
        active_talent_group: Some(0),
    }
}

fn assert_player_lifecycle_is_clean(player: &Player) {
    assert_eq!(player.unit().changed_object_type_mask(), 0);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

fn player_gameplay_sample_state() -> PlayerGameplayState {
    PlayerGameplayState {
        quests: PlayerQuestGameplayState {
            statuses: BTreeMap::from([(
                100,
                PlayerQuestStatusRecord {
                    quest_id: 100,
                    status: 3,
                    explored: true,
                    accept_time_secs: 1_700_000_000,
                    end_time_secs: 1_700_000_100,
                    objective_counts: vec![4],
                    slot: 1,
                },
            )]),
            objective_progress: vec![PlayerQuestObjectiveProgress {
                quest_id: 100,
                objective_id: 7,
                counter: 4,
            }],
            rewarded_quest_ids: BTreeSet::from([90]),
            daily_quest_ids: BTreeSet::from([101]),
            weekly_quest_ids: BTreeSet::from([102]),
            monthly_quest_ids: BTreeSet::from([103]),
            seasonal_quests: BTreeMap::from([(1, BTreeMap::from([(104, 0)]))]),
            ..Default::default()
        },
        skills: vec![PlayerSkillRecord {
            skill_line_id: SKILL_PLATE_MAIL,
            current_value: 225,
            max_value: 300,
            step: 2,
            profession_slot: -1,
            state: PlayerSkillLoadState::Unchanged,
        }],
        spells: PlayerSpellRuntimeState {
            known_spells: vec![635],
            rows: std::collections::BTreeMap::from([(
                635,
                PlayerKnownSpellRecord {
                    spell_id: 635,
                    state: PlayerSpellLoadState::Unchanged,
                    active: true,
                    disabled: false,
                    favorite: false,
                    dependent: false,
                },
            )]),
            rows_loaded: true,
            rows_complete: true,
            ..Default::default()
        },
        talents: talents::hydrated_talent_runtime_like_cpp(),
        action_buttons: vec![PlayerActionButtonRecord {
            button: 1,
            action_id: 635,
            action_type: 0,
        }],
        taxi: PlayerTaxiState {
            known_node_mask: vec![0b0000_0011, 0b1000_0000],
            known_node_mask_text: Some("3 128".to_string()),
            source_node_id: Some(1),
            destination_node_id: Some(2),
            destinations: vec![1, 2, 3],
            ..Default::default()
        },
        social: PlayerSocialState {
            friend_guids: vec![ObjectGuid::create_player(1, 1001)],
            ignore_guids: vec![ObjectGuid::create_player(1, 1002)],
            ..Default::default()
        },
        mails: vec![PlayerMailRecord {
            mail_id: 55,
            message_type: 0,
            sender: 1003,
            receiver: 42,
            template_id: Some(9),
            deliver_time: 1_700_000_000,
            expire_time: 1_700_086_400,
            checked_flags: 0x2,
            stationery_id: 0,
        }],
        group: Some(PlayerGroupState {
            group_guid: ObjectGuid::new(1, 77),
            leader_guid: ObjectGuid::create_player(1, 1001),
            role_mask: 0x1,
            subgroup: 0,
        }),
        guild: PlayerGuildState {
            guild_id: Some(12),
            invited_guild_id: Some(13),
            rank_id: Some(4),
            authority_complete: true,
        },
        battleground: PlayerBattlegroundState {
            queues: vec![PlayerBattlegroundQueueRecord {
                queue_id: 30,
                bracket_id: 4,
                joined_at: 1_700_000_050,
                team_id: TEAM_ALLIANCE_ID,
            }],
            current_bg_instance_id: Some(7001),
            current_bg_team: Some(TEAM_ALLIANCE_ID),
            random: PlayerRandomBattlegroundState {
                reward_claimed_today: true,
                last_reward_time: Some(1_700_000_060),
            },
            ..Default::default()
        },
        reputation: reputation::alliance_reputation_state_like_cpp(),
        achievements: vec![PlayerAchievementRecord {
            achievement_id: 6,
            completed_at: Some(1_700_000_070),
        }],
        achievement_criteria: vec![PlayerAchievementCriteriaRecord {
            criteria_id: 10,
            counter: 99,
            completed_at: None,
        }],
        currencies: HashMap::from([(
            395,
            crate::PlayerCurrency {
                state: crate::PlayerCurrencyState::Unchanged,
                quantity: 12,
                weekly_quantity: 3,
                tracked_quantity: 20,
                increased_cap_quantity: 0,
                earned_quantity: 0,
                flags: 0,
            },
        )]),
        spell_cooldowns: vec![PlayerSpellCooldownRecord {
            spell_id: 642,
            item_id: None,
            category_id: Some(100),
            cooldown_expires_at: 1_700_000_200,
            category_cooldown_expires_at: Some(1_700_000_150),
        }],
        spell_charges: vec![PlayerSpellChargeRecord {
            category_id: 100,
            consumed_charges: 1,
            recharge_started_at: Some(1_700_000_120),
            recharge_ends_at: Some(1_700_000_180),
        }],
        rest: PlayerRestState {
            rest_xp: 1234,
            rest_bonus: 1.5,
            rest_honor_bonus: 0.25,
            rest_state: 2,
            logout_time: Some(1_699_999_999),
            logout_was_resting: true,
            is_resting_now: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn can_bank_args<'a>(
    bag: u8,
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanBankItemArgs<'a> {
    CanBankItemArgs {
        bag,
        slot,
        proto,
        source_item,
        source_is_not_empty_bag: false,
        source_is_bag: false,
        source_is_currency_token: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        can_use_result: InventoryResult::Ok,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn find_equip_args<'a>(
    proto: &'a ItemStorageTemplate,
    slot: u8,
    swap: bool,
    equipped_items: &'a [ItemSlotRef<'a>],
) -> FindEquipSlotArgs<'a> {
    FindEquipSlotArgs {
        proto,
        slot,
        swap,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        has_required_profession_skill: false,
        profession_slot: None,
        equipped_items,
    }
}

fn can_equip_args<'a>(
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanEquipItemArgs<'a> {
    CanEquipItemArgs {
        slot,
        proto,
        source_item,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        not_loading: true,
        is_stunned: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
        weapon_change_timer_active: false,
        current_generic_spell_allows_equip: None,
        current_channeled_spell_allows_equip: None,
        heirloom_required_level_failed: false,
        can_use_result: InventoryResult::Ok,
        can_equip_unique_result: InventoryResult::Ok,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        proto_always_allow_dual_wield: false,
        has_required_profession_skill: false,
        profession_slot: None,
        offhand_can_unequip_result: InventoryResult::Ok,
        offhand_can_store_result: InventoryResult::Ok,
        limit_category: None,
        equipped_items: &[],
        stored_items: &[],
    }
}

fn can_unequip_args<'a>(
    pos: u16,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanUnequipItemArgs<'a> {
    CanUnequipItemArgs {
        pos,
        source_item,
        proto,
        swap: false,
        source_is_not_empty_bag: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
    }
}

fn can_use_template_args<'a>(proto: Option<&'a ItemStorageTemplate>) -> CanUseItemTemplateArgs<'a> {
    CanUseItemTemplateArgs {
        proto,
        skip_required_level_check: false,
        player_level: 70,
        team: TEAM_HORDE_ID,
        allowable_class_matches: true,
        allowable_race_matches: true,
        internal_item: false,
        faction_horde: false,
        faction_alliance: false,
        required_skill: 0,
        required_skill_rank: 0,
        required_skill_value: 0,
        required_spell: 0,
        has_required_spell: false,
        base_required_level: 0,
        holiday_id: 0,
        holiday_active: false,
        required_reputation_faction: 0,
        required_reputation_rank: 0,
        player_reputation_rank: 0,
        effect0_spell_id: None,
        effect1_spell_id: None,
        has_effect1_spell: false,
        artifact_specialization: None,
        primary_specialization: 0,
    }
}

fn can_use_args<'a>(
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanUseItemArgs<'a> {
    CanUseItemArgs {
        source_item,
        proto,
        not_loading: true,
        is_alive: true,
        player_level: 70,
        item_required_level: 0,
        source_bop_trade_allowed_for_player: false,
        template_args: can_use_template_args(proto),
        item_skill: 0,
        item_skill_value: 0,
        has_item_skill: false,
        player_class: CLASS_WARRIOR,
        proto_is_heirloom: false,
    }
}

fn can_equip_unique_template_args<'a>(
    proto: Option<&'a ItemStorageTemplate>,
) -> CanEquipUniqueItemTemplateArgs<'a> {
    CanEquipUniqueItemTemplateArgs {
        proto,
        except_slot: NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category: None,
        equipped_items: &[],
        equipped_gems: &[],
    }
}

fn can_equip_unique_args<'a>(
    source_item: Option<&'a Item>,
    proto: Option<&'a ItemStorageTemplate>,
) -> CanEquipUniqueItemArgs<'a> {
    CanEquipUniqueItemArgs {
        source_item,
        proto,
        except_slot: NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category: None,
        equipped_items: &[],
        equipped_gems: &[],
        socketed_gems: &[],
    }
}

#[path = "player_tests/combat.rs"]
mod combat;
#[path = "player_tests/gameobject.rs"]
mod gameobject;
#[path = "player_tests/group.rs"]
mod group;
#[path = "player_tests/instance.rs"]
mod instance;
#[path = "player_tests/item_1.rs"]
mod item_1;
#[path = "player_tests/item_2.rs"]
mod item_2;
#[path = "player_tests/item_3.rs"]
mod item_3;
#[path = "player_tests/item_4.rs"]
mod item_4;
#[path = "player_tests/item_5.rs"]
mod item_5;
#[path = "player_tests/item_6.rs"]
mod item_6;
#[path = "player_tests/item_7.rs"]
mod item_7;
#[path = "player_tests/loot.rs"]
mod loot;
#[path = "player_tests/misc.rs"]
mod misc;
#[path = "player_tests/movement.rs"]
mod movement;
#[path = "player_tests/persistence.rs"]
mod persistence;
#[path = "player_tests/pet.rs"]
mod pet;
#[path = "player_tests/quest.rs"]
mod quest;
#[path = "player_tests/reputation.rs"]
mod reputation;
#[path = "player_tests/skill.rs"]
mod skill;
#[path = "player_tests/spell.rs"]
mod spell;
#[path = "player_tests/talents.rs"]
mod talents;
#[path = "player_tests/visibility.rs"]
mod visibility;
