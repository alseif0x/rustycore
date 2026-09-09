//! Cli operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct CliOptions {
    pub(crate) config_path: String,
    pub(crate) dungeon_id: Option<u32>,
    pub(crate) timeout_secs: Option<u64>,
    pub(crate) single_account: Option<String>,
    pub(crate) sequential: bool,
    pub(crate) auto_teleport: Option<bool>,
    pub(crate) cleanup_groups: Option<bool>,
    pub(crate) require_group: bool,
    pub(crate) ensure_test_accounts: bool,
    pub(crate) login_only: bool,
    pub(crate) stand_state_smoke: bool,
    pub(crate) stand_state: Option<u8>,
    pub(crate) stand_state_timeout_secs: u64,
    pub(crate) bank_smoke: bool,
    pub(crate) bank_item_entry: u32,
    pub(crate) bank_runtime_counter: Option<u64>,
    pub(crate) bank_timeout_secs: u64,
    pub(crate) void_storage_smoke: bool,
    pub(crate) void_storage_query_capture: bool,
    pub(crate) void_storage_item_entry: u32,
    pub(crate) void_storage_runtime_counter: Option<u64>,
    pub(crate) void_storage_timeout_secs: u64,
    pub(crate) homebind_smoke: bool,
    pub(crate) homebind_runtime_counter: Option<u64>,
    pub(crate) homebind_timeout_secs: u64,
    pub(crate) inventory_swap_smoke: bool,
    pub(crate) inventory_swap_item_entry_a: u32,
    pub(crate) inventory_swap_item_entry_b: u32,
    pub(crate) inventory_swap_timeout_secs: u64,
    pub(crate) vendor_smoke: bool,
    pub(crate) vendor_entry: u32,
    pub(crate) vendor_spawn_guid: u64,
    pub(crate) vendor_runtime_counter: Option<u64>,
    pub(crate) vendor_item_entry: u32,
    pub(crate) vendor_extended_cost: u32,
    pub(crate) vendor_currency_id: u32,
    pub(crate) vendor_currency_cost: u32,
    pub(crate) vendor_currency_quantity: u32,
    pub(crate) vendor_timeout_secs: u64,
    pub(crate) equipment_set_race_smoke: bool,
    pub(crate) equipment_set_account_a: String,
    pub(crate) equipment_set_account_b: String,
    pub(crate) equipment_set_timeout_secs: u64,
    pub(crate) rested_xp_smoke: bool,
    pub(crate) ack_disposable_rested_xp: bool,
    pub(crate) rested_xp_creature_entry: u32,
    pub(crate) rested_xp_creature_guid: Option<u64>,
    pub(crate) rested_xp_runtime_counter: Option<u64>,
    pub(crate) rested_xp_offline_secs: u64,
    pub(crate) rested_xp_timeout_secs: u64,
    pub(crate) detour_chase_capture: bool,
    pub(crate) ack_disposable_detour_fixture: bool,
    pub(crate) detour_fixture_manifest: Option<String>,
    pub(crate) detour_chase_timeout_secs: u64,
    pub(crate) creature_spell_capture: bool,
    pub(crate) creature_spell_fixture_manifest: Option<String>,
    pub(crate) creature_spell_capture_timeout_secs: u64,
    pub(crate) cast_lifecycle: bool,
    pub(crate) cast_lifecycle_plan: Option<String>,
    pub(crate) loot_race_smoke: bool,
    pub(crate) loot_item_capture: bool,
    pub(crate) ack_disposable_overworld_loot_race: bool,
    pub(crate) loot_race_account_a: String,
    pub(crate) loot_race_account_b: String,
    pub(crate) loot_race_creature_entry: u32,
    pub(crate) loot_race_creature_spawn_guid: u64,
    pub(crate) loot_race_runtime_counter: u64,
    pub(crate) loot_race_item_entry: u32,
    pub(crate) loot_race_timeout_secs: u64,
    pub(crate) loot_workflow_deadline_secs: u64,
    pub(crate) recover_loot_fixture: bool,
    pub(crate) group_capacity_race_smoke: bool,
    pub(crate) group_capacity_leader_account: String,
    pub(crate) group_capacity_candidate_a_account: String,
    pub(crate) group_capacity_candidate_b_account: String,
    pub(crate) group_capacity_group_id: u32,
    pub(crate) group_capacity_timeout_secs: u64,
    pub(crate) quest_smoke: bool,
    pub(crate) quest_creature_entry: Option<u32>,
    pub(crate) quest_creature_guid: Option<u64>,
    pub(crate) quest_guid_counter: Option<u64>,
    pub(crate) quest_map_id: Option<u16>,
    pub(crate) quest_expected_id: Option<u32>,
    pub(crate) quest_forbidden_id: Option<u32>,
    pub(crate) quest_forbidden_title: Option<String>,
    pub(crate) quest_query_details: bool,
    pub(crate) quest_accept: bool,
    pub(crate) quest_reset: bool,
    pub(crate) quest_relocate: bool,
    pub(crate) quest_set_level: Option<u8>,
    pub(crate) quest_set_race: Option<u8>,
    pub(crate) quest_set_class: Option<u8>,
    pub(crate) quest_objective_persist: bool,
    pub(crate) quest_objectives: Vec<QuestObjectiveDbRow>,
    pub(crate) quest_objective_status: u8,
    pub(crate) gossip_select_option_id: Option<i32>,
    pub(crate) expect_trainer_list: bool,
    pub(crate) expect_trainer_id: Option<i32>,
    pub(crate) quest_timeout_secs: u64,
    pub(crate) report_path: Option<String>,
}
pub(crate) fn parse_cli() -> Result<CliOptions> {
    let stand_state = std::env::var("WOW_BOT_STAND_STATE")
        .ok()
        .map(|value| value.parse::<u8>())
        .transpose()?;
    let stand_state_smoke = std::env::var("WOW_BOT_STAND_STATE_SMOKE")
        .ok()
        .is_some_and(|value| is_truthy(&value))
        || stand_state.is_some();

    let mut opts = CliOptions {
        config_path: std::env::var("WOW_BOT_CONFIG").unwrap_or_else(|_| "config.json".to_string()),
        dungeon_id: std::env::var("WOW_BOT_DUNGEON_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        timeout_secs: std::env::var("WOW_BOT_LFG_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok()),
        single_account: None,
        sequential: false,
        auto_teleport: std::env::var("WOW_BOT_AUTO_TELEPORT")
            .ok()
            .map(|v| is_truthy(&v)),
        cleanup_groups: std::env::var("WOW_BOT_CLEANUP_GROUPS")
            .ok()
            .map(|v| is_truthy(&v)),
        require_group: std::env::var("WOW_BOT_REQUIRE_GROUP")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        ensure_test_accounts: std::env::var("WOW_BOT_ENSURE_TEST_ACCOUNTS")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        login_only: std::env::var("WOW_BOT_LOGIN_ONLY")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        stand_state_smoke,
        stand_state,
        stand_state_timeout_secs: std::env::var("WOW_BOT_STAND_STATE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(5),
        bank_smoke: std::env::var("WOW_BOT_BANK_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        bank_item_entry: std::env::var("WOW_BOT_BANK_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_BANK_SMOKE_ITEM_ENTRY),
        bank_runtime_counter: std::env::var("WOW_BOT_BANK_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        bank_timeout_secs: std::env::var("WOW_BOT_BANK_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        void_storage_smoke: std::env::var("WOW_BOT_VOID_STORAGE_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        void_storage_query_capture: std::env::var("WOW_BOT_VOID_STORAGE_QUERY_CAPTURE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        void_storage_item_entry: std::env::var("WOW_BOT_VOID_STORAGE_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VOID_STORAGE_SMOKE_ITEM_ENTRY),
        void_storage_runtime_counter: std::env::var("WOW_BOT_VOID_STORAGE_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        void_storage_timeout_secs: std::env::var("WOW_BOT_VOID_STORAGE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        homebind_smoke: std::env::var("WOW_BOT_HOMEBIND_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        homebind_runtime_counter: std::env::var("WOW_BOT_HOMEBIND_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        homebind_timeout_secs: std::env::var("WOW_BOT_HOMEBIND_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        inventory_swap_smoke: std::env::var("WOW_BOT_INVENTORY_SWAP_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        inventory_swap_item_entry_a: std::env::var("WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_A")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A),
        inventory_swap_item_entry_b: std::env::var("WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_B")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_B),
        inventory_swap_timeout_secs: std::env::var("WOW_BOT_INVENTORY_SWAP_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        vendor_smoke: std::env::var("WOW_BOT_VENDOR_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        vendor_entry: std::env::var("WOW_BOT_VENDOR_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_ENTRY),
        vendor_spawn_guid: std::env::var("WOW_BOT_VENDOR_SPAWN_GUID")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_SPAWN_GUID),
        vendor_runtime_counter: std::env::var("WOW_BOT_VENDOR_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        vendor_item_entry: std::env::var("WOW_BOT_VENDOR_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_ITEM_ENTRY),
        vendor_extended_cost: std::env::var("WOW_BOT_VENDOR_EXTENDED_COST")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_EXTENDED_COST),
        vendor_currency_id: std::env::var("WOW_BOT_VENDOR_CURRENCY_ID")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_ID),
        vendor_currency_cost: std::env::var("WOW_BOT_VENDOR_CURRENCY_COST")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_COST),
        vendor_currency_quantity: std::env::var("WOW_BOT_VENDOR_CURRENCY_QUANTITY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_QUANTITY),
        vendor_timeout_secs: std::env::var("WOW_BOT_VENDOR_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        equipment_set_race_smoke: std::env::var("WOW_BOT_EQUIPMENT_SET_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        equipment_set_account_a: std::env::var("WOW_BOT_EQUIPMENT_SET_ACCOUNT_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_A.to_string()),
        equipment_set_account_b: std::env::var("WOW_BOT_EQUIPMENT_SET_ACCOUNT_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_B.to_string()),
        equipment_set_timeout_secs: std::env::var("WOW_BOT_EQUIPMENT_SET_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(10),
        rested_xp_smoke: std::env::var("WOW_BOT_RESTED_XP_SMOKE")
            .ok()
            .map(|value| is_truthy(&value))
            .unwrap_or(false),
        // Deliberately CLI-only: an inherited environment must never acknowledge
        // destructive use of a disposable fixture implicitly.
        ack_disposable_rested_xp: false,
        rested_xp_creature_entry: std::env::var("WOW_BOT_RESTED_XP_CREATURE_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_CREATURE_ENTRY),
        rested_xp_creature_guid: std::env::var("WOW_BOT_RESTED_XP_CREATURE_GUID")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        rested_xp_runtime_counter: std::env::var("WOW_BOT_RESTED_XP_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        rested_xp_offline_secs: std::env::var("WOW_BOT_RESTED_XP_OFFLINE_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_OFFLINE_SECS),
        rested_xp_timeout_secs: std::env::var("WOW_BOT_RESTED_XP_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_TIMEOUT_SECS),
        detour_chase_capture: std::env::var("WOW_BOT_DETOUR_CHASE_CAPTURE")
            .ok()
            .map(|value| is_truthy(&value))
            .unwrap_or(false),
        // Deliberately CLI-only: an inherited environment must never
        // acknowledge mutation of a disposable deterministic fixture.
        ack_disposable_detour_fixture: false,
        detour_fixture_manifest: std::env::var("WOW_BOT_DETOUR_FIXTURE_MANIFEST").ok(),
        detour_chase_timeout_secs: std::env::var("WOW_BOT_DETOUR_CHASE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_DETOUR_CHASE_TIMEOUT_SECS),
        creature_spell_capture: std::env::var("WOW_BOT_CREATURE_SPELL_CAPTURE")
            .ok()
            .map(|value| is_truthy(&value))
            .unwrap_or(false),
        creature_spell_fixture_manifest: std::env::var("WOW_BOT_CREATURE_SPELL_FIXTURE_MANIFEST")
            .ok(),
        creature_spell_capture_timeout_secs: std::env::var("WOW_BOT_CREATURE_SPELL_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_CREATURE_SPELL_CAPTURE_TIMEOUT_SECS),
        cast_lifecycle: std::env::var("WOW_BOT_CAST_LIFECYCLE")
            .ok()
            .is_some_and(|value| is_truthy(&value))
            || std::env::var_os("WOW_BOT_CAST_LIFECYCLE_PLAN").is_some(),
        cast_lifecycle_plan: std::env::var("WOW_BOT_CAST_LIFECYCLE_PLAN").ok(),
        loot_race_smoke: std::env::var("WOW_BOT_LOOT_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        loot_item_capture: std::env::var("WOW_BOT_LOOT_ITEM_CAPTURE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        // Deliberately CLI-only: inherited environment cannot acknowledge
        // disposable loot-fixture mutation on the caller's behalf.
        ack_disposable_overworld_loot_race: false,
        loot_race_account_a: std::env::var("WOW_BOT_LOOT_RACE_ACCOUNT_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_A.to_string()),
        loot_race_account_b: std::env::var("WOW_BOT_LOOT_RACE_ACCOUNT_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_B.to_string()),
        loot_race_creature_entry: std::env::var("WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY")
            .or_else(|_| std::env::var("WOW_BOT_LOOT_RACE_CREATURE_ENTRY"))
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_CREATURE_ENTRY),
        loot_race_creature_spawn_guid: std::env::var("WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID")
            .or_else(|_| std::env::var("WOW_BOT_LOOT_RACE_CREATURE_SPAWN_GUID"))
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_CREATURE_SPAWN_GUID),
        loot_race_runtime_counter: std::env::var("WOW_BOT_LOOT_RACE_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_RUNTIME_COUNTER),
        loot_race_item_entry: std::env::var("WOW_BOT_LOOT_RACE_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_ITEM_ENTRY),
        loot_race_timeout_secs: std::env::var("WOW_BOT_LOOT_RACE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_TIMEOUT_SECS),
        loot_workflow_deadline_secs: std::env::var("WOW_BOT_LOOT_WORKFLOW_DEADLINE_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_WORKFLOW_DEADLINE_SECS),
        recover_loot_fixture: false,
        group_capacity_race_smoke: std::env::var("WOW_BOT_GROUP_CAPACITY_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        group_capacity_leader_account: std::env::var("WOW_BOT_GROUP_CAPACITY_LEADER")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_LEADER.to_string()),
        group_capacity_candidate_a_account: std::env::var("WOW_BOT_GROUP_CAPACITY_CANDIDATE_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_CANDIDATE_A.to_string()),
        group_capacity_candidate_b_account: std::env::var("WOW_BOT_GROUP_CAPACITY_CANDIDATE_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_CANDIDATE_B.to_string()),
        group_capacity_group_id: std::env::var("WOW_BOT_GROUP_CAPACITY_GROUP_ID")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(0),
        group_capacity_timeout_secs: std::env::var("WOW_BOT_GROUP_CAPACITY_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_GROUP_CAPACITY_TIMEOUT_SECS),
        quest_smoke: std::env::var("WOW_BOT_QUEST_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_creature_entry: std::env::var("WOW_BOT_QUEST_CREATURE_ENTRY")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_creature_guid: std::env::var("WOW_BOT_QUEST_CREATURE_GUID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_guid_counter: std::env::var("WOW_BOT_QUEST_RUNTIME_COUNTER")
            .or_else(|_| std::env::var("WOW_BOT_QUEST_GUID_COUNTER"))
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_map_id: std::env::var("WOW_BOT_QUEST_MAP_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_expected_id: std::env::var("WOW_BOT_QUEST_EXPECT_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_forbidden_id: std::env::var("WOW_BOT_QUEST_FORBID_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_forbidden_title: std::env::var("WOW_BOT_QUEST_FORBID_TITLE_CONTAINS").ok(),
        quest_query_details: std::env::var("WOW_BOT_QUEST_QUERY_DETAILS")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(true),
        quest_accept: std::env::var("WOW_BOT_QUEST_ACCEPT")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_reset: std::env::var("WOW_BOT_QUEST_RESET")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_relocate: std::env::var("WOW_BOT_QUEST_RELOCATE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_set_level: std::env::var("WOW_BOT_QUEST_SET_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_set_race: std::env::var("WOW_BOT_QUEST_SET_RACE")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_set_class: std::env::var("WOW_BOT_QUEST_SET_CLASS")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_objective_persist: std::env::var("WOW_BOT_QUEST_OBJECTIVE_PERSIST")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_objectives: match std::env::var("WOW_BOT_QUEST_OBJECTIVES") {
            Ok(value) if !value.trim().is_empty() => parse_quest_objective_rows(&value)?,
            _ => Vec::new(),
        },
        quest_objective_status: std::env::var("WOW_BOT_QUEST_OBJECTIVE_STATUS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3),
        gossip_select_option_id: std::env::var("WOW_BOT_GOSSIP_SELECT_OPTION_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        expect_trainer_list: std::env::var("WOW_BOT_EXPECT_TRAINER_LIST")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        expect_trainer_id: std::env::var("WOW_BOT_EXPECT_TRAINER_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_timeout_secs: std::env::var("WOW_BOT_QUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5),
        report_path: std::env::var("WOW_BOT_REPORT").ok(),
    };

    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if raw_args.len() == 6 && !raw_args[0].starts_with("--") {
        opts.single_account = Some(raw_args[0].clone());
        opts.dungeon_id = Some(raw_args[5].parse()?);
        return Ok(opts);
    }

    let mut args = raw_args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => opts.config_path = next_arg(&mut args, "--config")?,
            "--dungeon" => opts.dungeon_id = Some(next_arg(&mut args, "--dungeon")?.parse()?),
            "--timeout" => opts.timeout_secs = Some(next_arg(&mut args, "--timeout")?.parse()?),
            "--single" => opts.single_account = Some(next_arg(&mut args, "--single")?),
            "--sequential" => opts.sequential = true,
            "--parallel" => opts.sequential = false,
            "--auto-teleport" => opts.auto_teleport = Some(true),
            "--no-auto-teleport" => opts.auto_teleport = Some(false),
            "--cleanup-groups" => opts.cleanup_groups = Some(true),
            "--no-cleanup-groups" => opts.cleanup_groups = Some(false),
            "--require-group" => opts.require_group = true,
            "--ensure-test-accounts" => opts.ensure_test_accounts = true,
            "--login-only" => opts.login_only = true,
            "--stand-state-smoke" => opts.stand_state_smoke = true,
            "--stand-state" => {
                opts.stand_state_smoke = true;
                opts.stand_state = Some(next_arg(&mut args, "--stand-state")?.parse()?);
            }
            "--stand-state-timeout" => {
                opts.stand_state_timeout_secs =
                    next_arg(&mut args, "--stand-state-timeout")?.parse()?;
            }
            "--bank-smoke" => opts.bank_smoke = true,
            "--bank-item-entry" => {
                opts.bank_item_entry = next_arg(&mut args, "--bank-item-entry")?.parse()?;
            }
            "--bank-runtime-counter" => {
                opts.bank_runtime_counter =
                    Some(next_arg(&mut args, "--bank-runtime-counter")?.parse()?);
            }
            "--bank-timeout" => {
                opts.bank_timeout_secs = next_arg(&mut args, "--bank-timeout")?.parse()?;
            }
            "--void-storage-smoke" => opts.void_storage_smoke = true,
            "--void-storage-query-capture" => opts.void_storage_query_capture = true,
            "--void-storage-item-entry" => {
                opts.void_storage_item_entry =
                    next_arg(&mut args, "--void-storage-item-entry")?.parse()?;
            }
            "--void-storage-runtime-counter" => {
                opts.void_storage_runtime_counter =
                    Some(next_arg(&mut args, "--void-storage-runtime-counter")?.parse()?);
            }
            "--void-storage-timeout" => {
                opts.void_storage_timeout_secs =
                    next_arg(&mut args, "--void-storage-timeout")?.parse()?;
            }
            "--homebind-smoke" => opts.homebind_smoke = true,
            "--homebind-runtime-counter" => {
                opts.homebind_runtime_counter =
                    Some(next_arg(&mut args, "--homebind-runtime-counter")?.parse()?);
            }
            "--homebind-timeout" => {
                opts.homebind_timeout_secs = next_arg(&mut args, "--homebind-timeout")?.parse()?;
            }
            "--inventory-swap-smoke" => opts.inventory_swap_smoke = true,
            "--inventory-swap-item-entry-a" => {
                opts.inventory_swap_item_entry_a =
                    next_arg(&mut args, "--inventory-swap-item-entry-a")?.parse()?;
            }
            "--inventory-swap-item-entry-b" => {
                opts.inventory_swap_item_entry_b =
                    next_arg(&mut args, "--inventory-swap-item-entry-b")?.parse()?;
            }
            "--inventory-swap-timeout" => {
                opts.inventory_swap_timeout_secs =
                    next_arg(&mut args, "--inventory-swap-timeout")?.parse()?;
            }
            "--vendor-smoke" => opts.vendor_smoke = true,
            "--vendor-entry" => {
                opts.vendor_entry = next_arg(&mut args, "--vendor-entry")?.parse()?;
            }
            "--vendor-spawn-guid" => {
                opts.vendor_spawn_guid = next_arg(&mut args, "--vendor-spawn-guid")?.parse()?;
            }
            "--vendor-runtime-counter" => {
                opts.vendor_runtime_counter =
                    Some(next_arg(&mut args, "--vendor-runtime-counter")?.parse()?);
            }
            "--vendor-item-entry" => {
                opts.vendor_item_entry = next_arg(&mut args, "--vendor-item-entry")?.parse()?;
            }
            "--vendor-extended-cost" => {
                opts.vendor_extended_cost =
                    next_arg(&mut args, "--vendor-extended-cost")?.parse()?;
            }
            "--vendor-currency-id" => {
                opts.vendor_currency_id = next_arg(&mut args, "--vendor-currency-id")?.parse()?;
            }
            "--vendor-currency-cost" => {
                opts.vendor_currency_cost =
                    next_arg(&mut args, "--vendor-currency-cost")?.parse()?;
            }
            "--vendor-currency-quantity" => {
                opts.vendor_currency_quantity =
                    next_arg(&mut args, "--vendor-currency-quantity")?.parse()?;
            }
            "--vendor-timeout" => {
                opts.vendor_timeout_secs = next_arg(&mut args, "--vendor-timeout")?.parse()?;
            }
            "--equipment-set-race-smoke" => opts.equipment_set_race_smoke = true,
            "--equipment-set-account-a" => {
                opts.equipment_set_account_a = next_arg(&mut args, "--equipment-set-account-a")?;
            }
            "--equipment-set-account-b" => {
                opts.equipment_set_account_b = next_arg(&mut args, "--equipment-set-account-b")?;
            }
            "--equipment-set-timeout" => {
                opts.equipment_set_timeout_secs =
                    next_arg(&mut args, "--equipment-set-timeout")?.parse()?;
            }
            "--rested-xp-smoke" => opts.rested_xp_smoke = true,
            arg if parse_ack_disposable_rested_xp_arg(arg, &mut opts.ack_disposable_rested_xp) => {}
            "--rested-xp-creature-entry" => {
                opts.rested_xp_creature_entry =
                    next_arg(&mut args, "--rested-xp-creature-entry")?.parse()?;
            }
            "--rested-xp-creature-guid" => {
                opts.rested_xp_creature_guid =
                    Some(next_arg(&mut args, "--rested-xp-creature-guid")?.parse()?);
            }
            "--rested-xp-runtime-counter" => {
                opts.rested_xp_runtime_counter =
                    Some(next_arg(&mut args, "--rested-xp-runtime-counter")?.parse()?);
            }
            "--rested-xp-offline-secs" => {
                opts.rested_xp_offline_secs =
                    next_arg(&mut args, "--rested-xp-offline-secs")?.parse()?;
            }
            "--rested-xp-timeout" => {
                opts.rested_xp_timeout_secs =
                    next_arg(&mut args, "--rested-xp-timeout")?.parse()?;
            }
            "--detour-chase-capture" => opts.detour_chase_capture = true,
            arg if parse_ack_disposable_detour_fixture_arg(
                arg,
                &mut opts.ack_disposable_detour_fixture,
            ) => {}
            "--detour-fixture-manifest" => {
                opts.detour_fixture_manifest =
                    Some(next_arg(&mut args, "--detour-fixture-manifest")?);
            }
            "--detour-chase-timeout" => {
                opts.detour_chase_timeout_secs =
                    next_arg(&mut args, "--detour-chase-timeout")?.parse()?;
            }
            "--creature-spell-capture" => opts.creature_spell_capture = true,
            "--creature-spell-fixture-manifest" => {
                opts.creature_spell_fixture_manifest =
                    Some(next_arg(&mut args, "--creature-spell-fixture-manifest")?);
            }
            "--creature-spell-timeout" => {
                opts.creature_spell_capture_timeout_secs =
                    next_arg(&mut args, "--creature-spell-timeout")?.parse()?;
            }
            "--cast-lifecycle" => opts.cast_lifecycle = true,
            "--cast-lifecycle-plan" => {
                opts.cast_lifecycle = true;
                opts.cast_lifecycle_plan = Some(next_arg(&mut args, "--cast-lifecycle-plan")?);
            }
            "--loot-race-smoke" => opts.loot_race_smoke = true,
            "--loot-item-capture" => opts.loot_item_capture = true,
            arg if arg == loot_race::ACK_FLAG => opts.ack_disposable_overworld_loot_race = true,
            "--loot-race-account-a" => {
                opts.loot_race_account_a = next_arg(&mut args, "--loot-race-account-a")?;
            }
            "--loot-race-account-b" => {
                opts.loot_race_account_b = next_arg(&mut args, "--loot-race-account-b")?;
            }
            flag @ ("--loot-race-gameobject-entry" | "--loot-race-creature-entry") => {
                opts.loot_race_creature_entry = next_arg(&mut args, flag)?.parse()?;
            }
            flag @ ("--loot-race-gameobject-spawn-guid" | "--loot-race-creature-spawn-guid") => {
                opts.loot_race_creature_spawn_guid = next_arg(&mut args, flag)?.parse()?;
            }
            "--loot-race-runtime-counter" => {
                opts.loot_race_runtime_counter =
                    next_arg(&mut args, "--loot-race-runtime-counter")?.parse()?;
            }
            "--loot-race-item-entry" => {
                opts.loot_race_item_entry =
                    next_arg(&mut args, "--loot-race-item-entry")?.parse()?;
            }
            "--loot-race-timeout" => {
                opts.loot_race_timeout_secs =
                    next_arg(&mut args, "--loot-race-timeout")?.parse()?;
            }
            "--loot-workflow-deadline" => {
                opts.loot_workflow_deadline_secs =
                    next_arg(&mut args, "--loot-workflow-deadline")?.parse()?;
            }
            "--recover-loot-fixture" => opts.recover_loot_fixture = true,
            "--group-capacity-race-smoke" => opts.group_capacity_race_smoke = true,
            "--group-capacity-leader" => {
                opts.group_capacity_leader_account =
                    next_arg(&mut args, "--group-capacity-leader")?;
            }
            "--group-capacity-candidate-a" => {
                opts.group_capacity_candidate_a_account =
                    next_arg(&mut args, "--group-capacity-candidate-a")?;
            }
            "--group-capacity-candidate-b" => {
                opts.group_capacity_candidate_b_account =
                    next_arg(&mut args, "--group-capacity-candidate-b")?;
            }
            "--group-capacity-group-id" => {
                opts.group_capacity_group_id =
                    next_arg(&mut args, "--group-capacity-group-id")?.parse()?;
            }
            "--group-capacity-timeout" => {
                opts.group_capacity_timeout_secs =
                    next_arg(&mut args, "--group-capacity-timeout")?.parse()?;
            }
            "--quest-smoke" => opts.quest_smoke = true,
            "--quest-creature-entry" => {
                opts.quest_creature_entry =
                    Some(next_arg(&mut args, "--quest-creature-entry")?.parse()?);
            }
            "--quest-creature-guid" => {
                opts.quest_creature_guid =
                    Some(next_arg(&mut args, "--quest-creature-guid")?.parse()?);
            }
            "--quest-guid-counter" => {
                opts.quest_guid_counter =
                    Some(next_arg(&mut args, "--quest-guid-counter")?.parse()?);
            }
            "--quest-runtime-counter" => {
                opts.quest_guid_counter =
                    Some(next_arg(&mut args, "--quest-runtime-counter")?.parse()?);
            }
            "--quest-map" => opts.quest_map_id = Some(next_arg(&mut args, "--quest-map")?.parse()?),
            "--expect-quest" => {
                opts.quest_expected_id = Some(next_arg(&mut args, "--expect-quest")?.parse()?);
            }
            "--forbid-quest" => {
                opts.quest_forbidden_id = Some(next_arg(&mut args, "--forbid-quest")?.parse()?);
            }
            "--forbid-title" => {
                opts.quest_forbidden_title = Some(next_arg(&mut args, "--forbid-title")?)
            }
            "--quest-query-details" => opts.quest_query_details = true,
            "--no-quest-query-details" => opts.quest_query_details = false,
            "--quest-accept" => opts.quest_accept = true,
            "--quest-no-accept" => opts.quest_accept = false,
            "--quest-reset" => opts.quest_reset = true,
            "--quest-relocate" => opts.quest_relocate = true,
            "--quest-set-level" => {
                opts.quest_set_level = Some(next_arg(&mut args, "--quest-set-level")?.parse()?);
            }
            "--quest-set-race" => {
                opts.quest_set_race = Some(next_arg(&mut args, "--quest-set-race")?.parse()?);
            }
            "--quest-set-class" => {
                opts.quest_set_class = Some(next_arg(&mut args, "--quest-set-class")?.parse()?);
            }
            "--quest-objective-persist" => opts.quest_objective_persist = true,
            "--quest-objectives" => {
                opts.quest_objectives =
                    parse_quest_objective_rows(&next_arg(&mut args, "--quest-objectives")?)?;
            }
            "--quest-objective-status" => {
                opts.quest_objective_status =
                    next_arg(&mut args, "--quest-objective-status")?.parse()?;
            }
            "--gossip-select-option-id" => {
                opts.gossip_select_option_id =
                    Some(next_arg(&mut args, "--gossip-select-option-id")?.parse()?);
            }
            "--expect-trainer-list" => opts.expect_trainer_list = true,
            "--expect-trainer-id" => {
                opts.expect_trainer_id = Some(next_arg(&mut args, "--expect-trainer-id")?.parse()?);
            }
            "--quest-timeout" => {
                opts.quest_timeout_secs = next_arg(&mut args, "--quest-timeout")?.parse()?;
            }
            "--report" => opts.report_path = Some(next_arg(&mut args, "--report")?),
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("Unknown argument `{}`. Use --help.", arg),
        }
    }
    if opts.loot_item_capture
        && opts.loot_race_creature_entry == loot_race::DEFAULT_CREATURE_ENTRY
        && opts.loot_race_creature_spawn_guid == loot_race::DEFAULT_CREATURE_SPAWN_GUID
        && opts.loot_race_runtime_counter == loot_race::DEFAULT_RUNTIME_COUNTER
        && opts.loot_race_item_entry == loot_race::DEFAULT_ITEM_ENTRY
    {
        opts.loot_race_creature_entry = loot_race::DEFAULT_CAPTURE_CREATURE_ENTRY;
        opts.loot_race_creature_spawn_guid = loot_race::DEFAULT_CAPTURE_CREATURE_SPAWN_GUID;
        opts.loot_race_runtime_counter = loot_race::DEFAULT_CAPTURE_RUNTIME_COUNTER;
        opts.loot_race_item_entry = loot_race::DEFAULT_CAPTURE_ITEM_ENTRY;
    }
    Ok(opts)
}
