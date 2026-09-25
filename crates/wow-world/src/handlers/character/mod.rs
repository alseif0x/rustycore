// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handlers, organised by feature.
//!
//! Issue #224 split the former 20,272-line `handlers/character.rs` into
//! private feature modules. The logical owner, every registration, opcode and
//! dispatcher arm are unchanged; this module keeps the shared constants,
//! helper types and free functions the features build on.

mod account;
mod bank;
mod condition_objects;
mod creation_support;
mod entry_zone;
mod enumeration_support;
mod gossip;
mod item_load_support;
mod items;
mod lifecycle;
mod login_support;
mod login_transport_support;
mod pets;
mod query;
mod session_state;
mod spell_rules;
mod stats;
mod stats_queries;
mod stats_update;
mod vendor;
mod vendor_admission;
mod visibility;
mod world_entry;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;

use rand::Rng;
pub(crate) use stats::RepresentedPlayerGearStatsLikeCpp;
use tracing::{debug, info, trace, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::unit::{
    NPCFlags1, SheathState, UNIT_FLAGS_ALLOWED_LIKE_CPP, UNIT_FLAGS2_ALLOWED_LIKE_CPP,
    UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags,
};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, CreatureFlagsExtra, EnchantmentSlot, InventoryResult,
    InventoryType, ItemBondingType, ItemContext, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags,
    ItemFlags2, ItemModifier, ItemUpdateState, ItemVendorType, PowerType, Team, TypeId, TypeMask,
    UnitStandStateType,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
#[cfg(test)]
use wow_data::PlayerCreatePositionLikeCpp;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    PlayerCreateInfoLikeCpp, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
    PlayerStatSystemProjectionLikeCpp, TaxiPathNodeEntry, TaxiPathNodeStore,
    calculate_player_stat_system_like_cpp, hotfix_locale_mask,
    is_player_meeting_condition_like_cpp,
};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BUYBACK_SLOT_START, Corpse, CorpseCustomizationChoice,
    CorpseType, CreatureAddonLifecycleRecordLikeCpp, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_QUESTGIVER, GameObjectTemplateData, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START, InventoryStorageMovePlanLikeCpp, MAX_BAG_SIZE, MAX_MONEY_AMOUNT,
    MovementGeneratorType, NULL_BAG, NULL_SLOT, PlayerEffectiveCombatStatsLikeCpp,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SendNewItemDelivery, SendNewItemDisplayText,
    SendNewItemInstancePlan, SendNewItemModifier, SendNewItemPlan, SocketedGem,
    SwapItemPreflightResult, WorldObject, is_bank_pos, is_child_equipment_pos, is_equipment_pos,
    is_inventory_pos, item_can_go_into_bag, normalize_creature_chase_movement_type_like_cpp,
    normalize_creature_random_movement_type_like_cpp,
};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::packets::auth::{
    ConnectTo, ConnectToAddress, ConnectToFailed, ConnectToKey, ConnectToSerial, ResumeComms,
};
use wow_packet::packets::character::*;
use wow_packet::packets::chat::ChatServerMessage;
use wow_packet::packets::item::*;
use wow_packet::packets::loot::LootReleaseAll;
use wow_packet::packets::misc::*;
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::quest::QuestGiverStatusMultiple;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_packet::packets::update::*;
// Explicit provenance for child modules that reach these names through `super::{}`:
// a glob import leaves the ownership checker without a defining source.
use wow_packet::packets::misc::BindPointUpdate;
use wow_packet::packets::update::{
    ItemCreateData, PlayerCombatStats, UpdateBlock, UpdateObject, UpdateType,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerLoginTransportLoadRowLikeCpp,
};

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::map_manager::zone_and_area_for_position_like_cpp;
use crate::reputation::mgr::CharacterReputationRowLikeCpp;
use crate::session::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, CharacterPetAuraEffectRowLikeCpp,
    CharacterPetAuraRowLikeCpp, CharacterPetDeclinedNamesRowLikeCpp,
    CharacterPetSpellChargeRowLikeCpp, CharacterPetSpellCooldownRowLikeCpp,
    CharacterPetSpellRowLikeCpp, CharacterPetStableRowLikeCpp, CreatureSpawnCatalogsLikeCpp,
    GLOBAL_CACHE_MASK_LIKE_CPP, PlayerBootstrapCatalogsLikeCpp, REST_STATE_NORMAL_LIKE_CPP,
    REST_STATE_RAF_LINKED_LIKE_CPP, RepresentedAlterAppearanceLikeCpp,
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedBankItemMoveLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp, RepresentedGameObjectUseState,
    RepresentedHomebindLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp,
    RepresentedVoidStorageItemLikeCpp, SpellCastMetadata, SupportFeaturePolicyLikeCpp,
};
pub(crate) use creation_support::default_display_id;
use creation_support::{
    default_character_power1_like_cpp, default_health_mana, max_health_u32_like_cpp,
    restored_saved_health_like_cpp, start_position, start_zone,
};
use enumeration_support::{
    EnumCharacterFlagsLikeCpp, enum_character_effective_player_flags_like_cpp,
    enum_character_flags_like_cpp, enum_character_pet_data_like_cpp,
};
use item_load_support::*;
use login_support::*;
pub(crate) use login_transport_support::player_visibility_create_update_from_snapshot_like_cpp;
use login_transport_support::{
    InitTransportsPlanLikeCpp, MapTransportCreateLikeCpp, PersistedTransportLoginLikeCpp,
    TransportCreatePositionLikeCpp, compose_init_self_create_blocks_like_cpp,
    map_transport_create_block_like_cpp, map_transport_create_from_load_row_like_cpp,
    object_guid_from_db_binary_like_cpp, transport_position_for_login_like_cpp,
    transport_route_contains_saved_map_like_cpp, validate_persisted_transport_login_like_cpp,
};
#[cfg(test)]
use wow_entities::GAMEOBJECT_TYPE_GOOBER;

// ── Handler registration ────────────────────────────────────────────

const DEFAULT_MOTD_LIKE_CPP: &str = "Welcome to a Trinity Core Server.";
const DIRECT_VENDOR_MASK_LIKE_CPP: u32 = 0x80 | 0x100 | 0x200 | 0x400 | 0x800;
const DIRECT_TRAINER_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const DIRECT_FLIGHT_MASTER_LIKE_CPP: u32 = 0x2000;
const DIRECT_AUCTIONEER_LIKE_CPP: u32 = 0x200000;
const DIRECT_BANKER_LIKE_CPP: u32 = 0x20000;
const DIRECT_TABARD_DESIGNER_LIKE_CPP: u32 = 0x80000;
const DIRECT_STABLE_MASTER_LIKE_CPP: u32 = 0x400000;
const DIRECT_GUILD_BANKER_LIKE_CPP: u32 = 0x800000;
const DIRECT_INTERACTION_MASK_LIKE_CPP: u32 = DIRECT_VENDOR_MASK_LIKE_CPP
    | DIRECT_TRAINER_MASK_LIKE_CPP
    | DIRECT_FLIGHT_MASTER_LIKE_CPP
    | DIRECT_AUCTIONEER_LIKE_CPP
    | DIRECT_BANKER_LIKE_CPP
    | DIRECT_TABARD_DESIGNER_LIKE_CPP
    | DIRECT_STABLE_MASTER_LIKE_CPP
    | DIRECT_GUILD_BANKER_LIKE_CPP;

fn npc_has_direct_interaction_like_cpp(npc_flags: u32) -> bool {
    npc_flags & DIRECT_INTERACTION_MASK_LIKE_CPP != 0
}
const WORLDSTATE_ANY_MAP_LIKE_CPP: i32 = -1;
const DEFAULT_GOSSIP_MESSAGE_LIKE_CPP: i32 = 0x00FF_FFFF;
const TRAINER_NPC_FLAGS_MASK_LIKE_CPP: u32 = 0x10 | 0x20 | 0x40;
const GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP: i32 = -1;
const GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP: u8 = 3;
const GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP: &str = "I would like to train.";
const ITEM_ENCHANTMENT_DB_FIELDS: usize = 3;

#[derive(Debug, Clone, PartialEq)]
struct LoadedMapCorpseRowLikeCpp {
    position: Position,
    map_id: u16,
    display_id: u32,
    items: [u32; wow_entities::CORPSE_ITEMS],
    race: u8,
    class: u8,
    sex: u8,
    flags: u32,
    dynamic_flags: u32,
    ghost_time: i64,
    corpse_type: CorpseType,
    instance_id: u32,
    owner_db_guid: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MapCorpseLoadOutcomeLikeCpp {
    already_loaded: bool,
    rows_seen: u32,
    corpses_added: u32,
    invalid_type_rows: u32,
    invalid_race_rows: u32,
    invalid_position_rows: u32,
    add_to_map_errors: u32,
}

fn parse_corpse_items_like_cpp(item_cache: &str) -> [u32; wow_entities::CORPSE_ITEMS] {
    let mut items = [0; wow_entities::CORPSE_ITEMS];
    let tokens = item_cache.split_whitespace().collect::<Vec<_>>();
    if tokens.len() == items.len() {
        for (slot, token) in tokens.into_iter().enumerate() {
            items[slot] = token.parse().unwrap_or(0);
        }
    }
    items
}

fn materialize_loaded_map_corpses_like_cpp(
    map: &mut wow_map::Map,
    realm_id: u16,
    rows: Vec<LoadedMapCorpseRowLikeCpp>,
    phases: &HashMap<u64, BTreeSet<u32>>,
    customizations: &HashMap<u64, Vec<CorpseCustomizationChoice>>,
    faction_templates_by_race: &HashMap<u8, i32>,
) -> MapCorpseLoadOutcomeLikeCpp {
    if map.corpse_data_loaded_like_cpp() {
        return MapCorpseLoadOutcomeLikeCpp {
            already_loaded: true,
            ..Default::default()
        };
    }

    let mut outcome = MapCorpseLoadOutcomeLikeCpp::default();
    for row in rows {
        outcome.rows_seen = outcome.rows_seen.saturating_add(1);
        // C++ `Map::LoadCorpseData` consumes the map-local counter when it
        // calls `LoadCorpseFromDB(GenerateLowGuid(), fields)`. The latter only
        // validates map coordinates near the end, so even a rejected position
        // has already advanced the GUID generator.
        let Ok(low_guid) = map.generate_low_guid_like_cpp(HighGuid::Corpse) else {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        };
        if row.map_id != map.map_id() as u16 || row.instance_id != map.instance_id() {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        }
        if !row.position.is_valid_map_coord_like_cpp() {
            outcome.invalid_position_rows = outcome.invalid_position_rows.saturating_add(1);
            continue;
        }
        let Some(faction_template) = faction_templates_by_race.get(&row.race).copied() else {
            outcome.invalid_race_rows = outcome.invalid_race_rows.saturating_add(1);
            continue;
        };

        let mut corpse = Corpse::new_at(row.corpse_type, row.ghost_time);
        let corpse_guid = ObjectGuid::create_world_object(
            HighGuid::Corpse,
            0,
            realm_id,
            row.map_id,
            0,
            0,
            low_guid,
        );
        corpse.world_mut().object_mut().create(corpse_guid);
        if corpse
            .world_mut()
            .set_map(u32::from(row.map_id), row.instance_id)
            .is_err()
        {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        }
        corpse.world_mut().relocate(row.position);
        corpse.set_display_id(row.display_id);
        corpse.set_race(row.race);
        corpse.set_class(row.class);
        corpse.set_sex(row.sex);
        corpse.replace_all_flags(row.flags);
        corpse.replace_all_corpse_dynamic_flags(row.dynamic_flags);
        corpse.set_owner_guid(ObjectGuid::create_player(
            realm_id,
            row.owner_db_guid as i64,
        ));
        corpse.set_faction_template(faction_template);
        for (slot, item) in row.items.into_iter().enumerate() {
            corpse.set_item(slot, item);
        }
        for phase_id in phases
            .get(&row.owner_db_guid)
            .into_iter()
            .flatten()
            .copied()
        {
            corpse.world_mut().phase_shift_mut().insert(phase_id);
        }
        corpse.set_customizations(
            customizations
                .get(&row.owner_db_guid)
                .cloned()
                .unwrap_or_default(),
        );

        // C++ loads these fields before AddCorpse/AddToMap, so they form the
        // clean baseline rather than a later VALUES delta.
        corpse.clear_corpse_data_changes();
        corpse.world_mut().object_mut().clear_update_mask(false);
        match map.register_loaded_corpse_like_cpp(corpse) {
            Ok(_) => outcome.corpses_added = outcome.corpses_added.saturating_add(1),
            Err(_) => {
                outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            }
        }
    }

    map.mark_corpse_data_loaded_like_cpp();
    outcome
}

fn motd_lines_like_cpp(motd: &str) -> Vec<String> {
    // C++ `World::SetMotd` uses `boost::split` on `@` with token compression
    // disabled, so empty and trailing lines remain part of the login burst.
    motd.split('@').map(ToOwned::to_owned).collect()
}

fn void_storage_login_context_like_cpp(
    random_properties_id: i32,
    _selected_context_column: u8,
) -> u8 {
    // Audited 3.4.3 `Player::_LoadVoidStorage` constructs ItemContext from
    // fields[5] even though CHAR_SEL_CHAR_VOID_STORAGE selects `context` as
    // fields[7]. Keep that executable C++ behavior; the unused argument makes
    // the query/implementation mismatch explicit instead of hiding column 7.
    random_properties_id as u8
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectInventoryPositionUpdateLikeCpp {
    slot: u8,
    item_db_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventorySwapTargetLikeCpp {
    Inventory,
    Bank,
    Equipment { dest: u16 },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryStorageTargetLikeCpp {
    Inventory,
    Bank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryStorageQuestChecksLikeCpp {
    None,
    AutoBankItemRemoved,
    AutoStoreBankItemAdded,
}

fn autostore_bank_target_like_cpp(
    source_bag: u8,
    source_slot: u8,
) -> InventoryStorageTargetLikeCpp {
    if is_bank_pos(source_bag, source_slot) {
        InventoryStorageTargetLikeCpp::Inventory
    } else {
        InventoryStorageTargetLikeCpp::Bank
    }
}

fn autostore_bank_quest_checks_like_cpp(
    target: InventoryStorageTargetLikeCpp,
) -> InventoryStorageQuestChecksLikeCpp {
    if target == InventoryStorageTargetLikeCpp::Inventory {
        InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
    } else {
        // C++ HandleAutoStoreBankItemOpcode intentionally does not call
        // ItemRemovedQuestCheck in its inventory-to-bank branch.
        InventoryStorageQuestChecksLikeCpp::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InventoryEquipChildPlanLikeCpp {
    child_guid: ObjectGuid,
    destination_slot: u8,
    displaced_storage: Option<(u8, u8, InventoryStorageTargetLikeCpp)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventorySwapStepLikeCpp {
    Done,
    ChildRedirect {
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    },
}

#[cfg(test)]
fn plan_direct_inventory_swap_persistence_like_cpp(
    src: u8,
    dst: u8,
    src_item: Option<&InventoryItem>,
    dst_item: Option<&InventoryItem>,
) -> Vec<DirectInventoryPositionUpdateLikeCpp> {
    let mut updates = Vec::with_capacity(2);
    if let Some(item) = src_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: dst,
            item_db_guid: item.db_guid,
        });
    }
    if let Some(item) = dst_item {
        updates.push(DirectInventoryPositionUpdateLikeCpp {
            slot: src,
            item_db_guid: item.db_guid,
        });
    }
    updates
}

fn creature_has_trainer_flag_like_cpp(npc_flags: u32) -> bool {
    (npc_flags & TRAINER_NPC_FLAGS_MASK_LIKE_CPP) != 0
}

fn represented_trainer_gossip_option_like_cpp() -> wow_packet::packets::gossip::ClientGossipOption {
    wow_packet::packets::gossip::ClientGossipOption {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        option_flags: 0,
        option_cost: 0,
        option_language: 0,
        flags: 0,
        order_index: 0,
        status: 0,
        text: GOSSIP_OPTION_TRAINER_TEXT_LIKE_CPP.to_string(),
        confirm: String::new(),
        spell_id: None,
        override_icon_id: None,
    }
}

fn represented_trainer_gossip_option_info_like_cpp() -> crate::session::GossipOptionInfo {
    crate::session::GossipOptionInfo {
        gossip_option_id: GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP,
        menu_id: 0,
        order_index: 0,
        option_npc: GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP,
        action_menu_id: 0,
    }
}

fn add_represented_trainer_gossip_option_if_missing_like_cpp(
    gossip_options: &mut Vec<wow_packet::packets::gossip::ClientGossipOption>,
    stored_options: &mut Vec<crate::session::GossipOptionInfo>,
    npc_flags: u32,
) -> bool {
    if !creature_has_trainer_flag_like_cpp(npc_flags) {
        return false;
    }

    if gossip_options
        .iter()
        .any(|option| option.option_npc == GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP)
    {
        return false;
    }

    gossip_options.push(represented_trainer_gossip_option_like_cpp());
    stored_options.push(represented_trainer_gossip_option_info_like_cpp());
    true
}
fn primary_power_type_for_class_like_cpp(class_id: u8) -> PowerType {
    match class_id {
        1 => PowerType::Rage,
        4 => PowerType::Energy,
        6 => PowerType::RunicPower,
        _ => PowerType::Mana,
    }
}

fn primary_max_power_for_class_like_cpp(class_id: u8, max_mana: i64) -> i32 {
    match class_id {
        1 | 6 => 1_000,
        4 => 100,
        _ => max_mana.max(0).min(i64::from(i32::MAX)) as i32,
    }
}

fn loaded_inventory_slot_count_with_legacy_rust_compat(saved_slots: u8) -> u8 {
    // C++ loads the saved value directly, but TrinityCore's schema defaults
    // inventorySlots to the base backpack size. Older RustyCore builds
    // explicitly inserted zero before this field was wired; keep those
    // already-created characters playable without an out-of-band migration.
    if saved_slots == 0 {
        INVENTORY_DEFAULT_SIZE
    } else {
        saved_slots
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoadedItemRandomPropertiesLikeCpp {
    id: i32,
    seed: i32,
}

fn bank_store_item_added_quest_count_like_cpp(plan: &InventoryStorageMovePlanLikeCpp) -> u32 {
    // C++ HandleAutoStoreBankItemOpcode passes storedItem->GetCount() after
    // StoreItem. _StoreItem returns the last destination item, so a full merge
    // reports that destination stack's total and a merge+remainder reports the
    // final remainder stack count. This is deliberately not source_count.
    plan.moved_destination
        .map(|(_, _, count)| count)
        .or_else(|| plan.existing_updates.last().map(|update| update.new_count))
        .unwrap_or(0)
}

fn bank_store_destination_applies_obtain_spells_like_cpp(bag: u8) -> bool {
    // C++ Player::_StoreItem checks only the bag value. INVENTORY_SLOT_BAG_0
    // therefore includes top-level personal-bank slots as well as carried
    // top-level slots; bank-bag containers remain excluded.
    bag == INVENTORY_SLOT_BAG_0
        || (wow_entities::INVENTORY_SLOT_BAG_START..wow_entities::INVENTORY_SLOT_BAG_END)
            .contains(&bag)
}

fn inventory_storage_move_quest_directions_like_cpp(
    source_bag: u8,
    source_slot: u8,
    target: InventoryStorageTargetLikeCpp,
) -> (bool, bool) {
    let moving_to_bank = target == InventoryStorageTargetLikeCpp::Bank;
    let moving_from_bank = !moving_to_bank && is_bank_pos(source_bag, source_slot);
    (moving_to_bank, moving_from_bank)
}

type ItemStorageMutablePersistenceLikeCpp = wow_persistence::InventoryItemMutablePersistenceLikeCpp;

const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;
const QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP: u32 = 1000;
const MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP: f32 = 20.0;
// C++ ObjectDefines.h: DEFAULT_VISIBILITY_DISTANCE = VISIBILITY_DISTANCE_NORMAL = 100 yards.
// Wider values here make the SQL fallback load whole areas and can crash the 3.4.3 client.
const DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP: f32 = crate::map_manager::VISIBILITY_RADIUS;
pub(crate) use wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP;
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;
const CHAR_CREATE_NAME_IN_USE_LIKE_CPP: u8 = 27;
pub(crate) use wow_constants::character::{
    CHAR_NAME_INVALID_CHARACTER_LIKE_CPP, CHAR_NAME_NO_NAME_LIKE_CPP, CHAR_NAME_TOO_LONG_LIKE_CPP,
    CHAR_NAME_TOO_SHORT_LIKE_CPP,
};
const CLASS_HUNTER_LIKE_CPP: u8 = 3;
const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
const CLASS_WARLOCK_LIKE_CPP: u8 = 9;
const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
const AT_LOGIN_CUSTOMIZE_LIKE_CPP: u16 = 0x008;
const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;
const AT_LOGIN_CHANGE_FACTION_LIKE_CPP: u16 = 0x040;
const AT_LOGIN_CHANGE_RACE_LIKE_CPP: u16 = 0x080;
const AT_LOGIN_RESURRECT_LIKE_CPP: u16 = 0x100;
const CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP: u32 = 0x0000_0004;
const CHARACTER_FLAG_GHOST_LIKE_CPP: u32 = 0x0000_2000;
const CHARACTER_FLAG_RENAME_LIKE_CPP: u32 = 0x0000_4000;
const CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP: u32 = 0x0100_0000;
const CHARACTER_FLAG_DECLINED_LIKE_CPP: u32 = 0x0200_0000;
const CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP: u32 = 0x0000_0001;
const CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP: u32 = 0x0001_0000;
const CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP: u32 = 0x0010_0000;
const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP: u8 = 15;
const TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP: i32 = 0x1;
const TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP: i32 = 0x2;

fn initial_character_rest_state_like_cpp(is_a_recruiter: bool, recruiter_id: u32) -> u8 {
    if is_a_recruiter || recruiter_id != 0 {
        REST_STATE_RAF_LINKED_LIKE_CPP
    } else {
        REST_STATE_NORMAL_LIKE_CPP
    }
}

fn creature_movement_generator_type_from_db_like_cpp(
    db_movement_type: u8,
    wander_distance: f32,
) -> MovementGeneratorType {
    const RANDOM_MOTION_TYPE_LIKE_CPP: u8 = 1;
    match db_movement_type {
        WAYPOINT_MOTION_TYPE_LIKE_CPP => MovementGeneratorType::Waypoint,
        RANDOM_MOTION_TYPE_LIKE_CPP if wander_distance > 0.0 => MovementGeneratorType::Random,
        _ => MovementGeneratorType::Idle,
    }
}

fn normalized_creature_wander_distance_like_cpp(
    default_movement_type: MovementGeneratorType,
    wander_distance: f32,
) -> f32 {
    let wander_distance = wander_distance.max(0.0);
    if default_movement_type == MovementGeneratorType::Idle {
        0.0
    } else {
        wander_distance
    }
}

fn normalize_creature_template_speed_walk_like_cpp(speed_walk: f32) -> f32 {
    if speed_walk == 0.0 { 1.0 } else { speed_walk }
}

fn normalize_creature_template_speed_run_like_cpp(speed_run: f32) -> f32 {
    if speed_run == 0.0 { 1.14286 } else { speed_run }
}

fn spawn_difficulties_contains_spawn_mode_like_cpp(
    spawn_difficulties: &str,
    spawn_mode: u8,
) -> bool {
    // C++ ObjectMgr::ParseSpawnDifficulties parses comma-separated Difficulty
    // values and maps invalid tokens to DIFFICULTY_NONE before the map/grid
    // code filters by Map::GetSpawnMode().
    spawn_difficulties
        .split(',')
        .filter(|token| !token.is_empty())
        .map(|token| token.parse::<u8>().unwrap_or(0))
        .any(|difficulty| difficulty == spawn_mode)
}

fn choose_creature_flags_like_cpp(
    template_npc_flags: u64,
    template_unit_flags: u32,
    template_unit_flags2: u32,
    template_unit_flags3: u32,
    spawn_npc_flags: Option<u64>,
    spawn_unit_flags: Option<u32>,
    spawn_unit_flags2: Option<u32>,
    spawn_unit_flags3: Option<u32>,
    flags_extra: u32,
) -> (u64, u32, u32, u32) {
    // C++ ObjectMgr::ChooseCreatureFlags: spawn overrides are optional;
    // missing values fall back to creature_template.
    let npc_flags = spawn_npc_flags.unwrap_or(template_npc_flags);
    let mut unit_flags =
        spawn_unit_flags.unwrap_or(template_unit_flags) & UNIT_FLAGS_ALLOWED_LIKE_CPP;
    let unit_flags2 =
        spawn_unit_flags2.unwrap_or(template_unit_flags2) & UNIT_FLAGS2_ALLOWED_LIKE_CPP;
    let unit_flags3 =
        spawn_unit_flags3.unwrap_or(template_unit_flags3) & UNIT_FLAGS3_ALLOWED_LIKE_CPP;

    // C++ Creature::UpdateEntry clears template combat state on create and
    // only restores it when the creature is already in combat.
    unit_flags &= !UnitFlags::IN_COMBAT.bits();

    // C++ Creature::UpdateEntry calls SetUninteractible(true) for triggers
    // after selecting DB flags.
    if CreatureFlagsExtra::from_bits_truncate(flags_extra).contains(CreatureFlagsExtra::TRIGGER) {
        unit_flags |= UnitFlags::UNINTERACTIBLE.bits();
    }

    (npc_flags, unit_flags, unit_flags2, unit_flags3)
}

fn is_within_2d_visibility_range_like_cpp(
    viewer: &Position,
    object_x: f32,
    object_y: f32,
    range: f32,
) -> bool {
    let dx = viewer.x - object_x;
    let dy = viewer.y - object_y;
    dx * dx + dy * dy <= range * range
}

fn represented_go_state_from_i8_like_cpp(state: i8) -> Option<wow_entities::GoState> {
    match state {
        0 => Some(wow_entities::GoState::Active),
        1 => Some(wow_entities::GoState::Ready),
        2 => Some(wow_entities::GoState::Destroyed),
        24 => Some(wow_entities::GoState::TransportActive),
        25 => Some(wow_entities::GoState::TransportStopped),
        _ => None,
    }
}

use wow_packet::packets::gossip::*;
use wow_packet::packets::query::*;

use crate::session::{InventoryItem, WorldSession};

// ── Hardcoded data ──────────────────────────────────────────────────

/// Maximum characters per account.
const MAX_CHARACTERS_PER_ACCOUNT: u32 = 10;

/// Reverse-map an equipment slot (0-18) to its InventoryType.
///
/// Used as a fallback when Item.db2 store is not available.
fn slot_to_inventory_type(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(1),        // Head
        1 => Some(2),        // Neck
        2 => Some(3),        // Shoulders
        3 => Some(4),        // Body (Shirt)
        4 => Some(5),        // Chest
        5 => Some(6),        // Waist
        6 => Some(7),        // Legs
        7 => Some(8),        // Feet
        8 => Some(9),        // Wrists
        9 => Some(10),       // Hands
        10 | 11 => Some(11), // Finger (Ring)
        12 | 13 => Some(12), // Trinket
        14 => Some(16),      // Cloak
        15 => Some(21),      // MainHand (WeaponMainHand)
        16 => Some(22),      // OffHand (WeaponOffHand)
        17 => Some(15),      // Ranged
        18 => Some(19),      // Tabard
        _ => None,
    }
}

/// Parse a space-separated equipment cache string into VisualItemInfo array.
///
/// C++ `EnumCharactersResult::CharacterInfo` parses `equipmentCache` as five
/// fields per slot: InvType, DisplayID, DisplayEnchantID, Subclass, and
/// SecondaryItemModifiedAppearanceID.
fn parse_equipment_cache(cache: &str) -> [VisualItemInfo; 34] {
    let mut equipment = [VisualItemInfo::default(); 34];
    if cache.is_empty() {
        return equipment;
    }

    let parts: Vec<&str> = cache.split_whitespace().collect();
    let fields_per_slot = 5;

    for slot in 0..34 {
        let base = slot * fields_per_slot;
        if base + fields_per_slot > parts.len() {
            break;
        }
        equipment[slot] = VisualItemInfo {
            inv_type: parts[base].parse().unwrap_or(0),
            display_id: parts[base + 1].parse().unwrap_or(0),
            display_enchant_id: parts[base + 2].parse().unwrap_or(0),
            subclass: parts[base + 3].parse().unwrap_or(0),
            secondary_item_modified_appearance_id: parts[base + 4].parse().unwrap_or(0),
        };
    }

    equipment
}

fn bind_inventory_item_for_destination_like_cpp(item: &mut wow_entities::Item, destination: u16) {
    let [bag, slot] = destination.to_be_bytes();
    if is_equipment_pos(bag, slot) {
        // C++ `Player::EquipItem` calls `VisualizeItem`, which binds
        // BIND_ON_EQUIP as well as the acquire/quest bonding modes.
        item.bind_if_visualized();
    } else {
        // C++ `Player::_StoreItem` has the narrower storage rule: an
        // OnEquip item binds here only when stored in a bag-equipment slot.
        item.bind_if_stored(wow_entities::is_bag_pos(destination));
    }
}

fn item_dynamic_flags_changed_like_cpp(
    before: &wow_entities::Item,
    after: &wow_entities::Item,
) -> bool {
    before.item_flags_bits() != after.item_flags_bits()
}

fn player_money_gain_like_cpp(current_money: u64, amount: u64) -> Option<u64> {
    if amount == 0 {
        return Some(current_money);
    }

    let max_gain = MAX_MONEY_AMOUNT.checked_sub(amount)?;
    if current_money <= max_gain {
        Some(current_money + amount)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CreatureAddonCreateFieldsLikeCpp {
    has_addon: bool,
    mount_display_id: i32,
    stand_state: u8,
    vis_flags: u8,
    anim_tier: u8,
    sheathe_state: u8,
    pvp_flags: u8,
    emote_state: i32,
    ai_anim_kit_id: u16,
    movement_anim_kit_id: u16,
    melee_anim_kit_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CreatureEquipmentCreateFieldsLikeCpp {
    selected_equipment_id: u8,
    original_equipment_id: i8,
    virtual_items: [(i32, u16, u16); 3],
}

#[derive(Debug, Clone)]
struct MaterializedCreatureSpawnLikeCpp {
    guid: ObjectGuid,
    position: Position,
    create_data: CreatureCreateData,
    min_damage: u32,
    max_damage: u32,
    aggro_radius: f32,
    loot_id: u32,
    skin_loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    respawn_delay_secs: u32,
    selected_equipment_id: u8,
    original_equipment_id: i8,
    script_name: String,
    string_id: Option<String>,
    addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
    phase_use_flags: u8,
    phase_id: u16,
    phase_group_id: u32,
    terrain_swap_map: i32,
    flags_extra: u32,
    ground_movement_type: u8,
    swim_allowed: bool,
    flight_movement_type: u8,
    rooted: bool,
    chase_movement_type: u8,
    random_movement_type: u8,
    interaction_pause_timer_ms: u32,
    wander_distance: f32,
    default_movement_type: MovementGeneratorType,
    waypoint_path_id: u32,
}

fn creature_create_movement_flags_like_cpp(ground_movement_type: u8, rooted: bool) -> u32 {
    let mut flags = MovementFlag::empty();
    if ground_movement_type == wow_constants::CreatureGroundMovementType::Hover as u8 {
        // C++ Creature::LoadCreaturesAddon calls AddUnitMovementFlag(MOVEMENTFLAG_HOVER)
        // when CanHover(), and CanHover() is true for ground movement type Hover.
        flags.insert(MovementFlag::HOVER);
    }
    if rooted {
        // C++ Creature::LoadTemplateRoot -> SetTemplateRooted -> SetControlled(... ROOT)
        // ends in Unit::SetRooted, removing moving flags before adding MOVEMENTFLAG_ROOT.
        flags.remove(MovementFlag::MASK_MOVING);
        flags.insert(MovementFlag::ROOT);
    }
    flags.bits()
}

fn creature_create_position_after_hover_offset_like_cpp(
    mut position: Position,
    movement_flags: u32,
    hover_height: f32,
) -> Position {
    // C++ `Creature::Create` calls `LoadCreaturesAddon()` and then
    // `m_positionZ += GetHoverOffset()`. `GetHoverOffset()` is
    // MOVEMENTFLAG_HOVER ? UnitData::HoverHeight : 0.
    if MovementFlag::from_bits_retain(movement_flags).contains(MovementFlag::HOVER) {
        position.z += hover_height;
    }
    position
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtendedCostItemTurninChange {
    Update {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
        new_count: u32,
    },
    Delete {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestroyItemCountAction {
    FullStack,
    PartialStack { new_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DestroyQuestItemLikeCpp {
    bag: u8,
    slot: u8,
    entry_id: u32,
    count: u32,
}

fn destroy_item_count_action(current_count: u32, requested_count: u32) -> DestroyItemCountAction {
    if requested_count != 0 && current_count > requested_count {
        return DestroyItemCountAction::PartialStack {
            new_count: current_count - requested_count,
        };
    }

    DestroyItemCountAction::FullStack
}

fn item_spell_charges_db_string(charges: &[i32], effect_count: usize) -> String {
    let mut out = String::new();
    for charge in charges.iter().take(effect_count) {
        out.push_str(&charge.to_string());
        out.push(' ');
    }
    out
}

fn item_storage_mutable_persistence_like_cpp(
    db_guid: u64,
    item: &wow_entities::Item,
    count: u32,
    flags: u32,
    enchantments: String,
    effect_count: usize,
) -> ItemStorageMutablePersistenceLikeCpp {
    let data = item.data();
    ItemStorageMutablePersistenceLikeCpp {
        item_guid: db_guid,
        count,
        expiration: data.expiration,
        charges: item_spell_charges_db_string(&data.spell_charges, effect_count),
        flags,
        enchantments,
        durability: data.durability,
        played_time: data.create_played_time,
    }
}

fn item_is_currently_looted_like_cpp(item: &wow_entities::Item) -> bool {
    item.loot_generated()
}

fn item_is_not_empty_bag_like_cpp(
    inventory_type: Option<InventoryType>,
    contains_items: bool,
) -> bool {
    matches!(inventory_type, Some(InventoryType::Bag)) && contains_items
}

fn player_class_mask(player_class: u8) -> u32 {
    player_class
        .checked_sub(1)
        .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
        .unwrap_or(0)
}

fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        // C++ resolves this from ChrRacesEntry::Alliance: 1 = Horde, 0 = Alliance.
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

#[cfg(test)]
#[path = "../character_vendor_atomicity_tests.rs"]
mod vendor_atomicity_tests;

#[cfg(test)]
#[path = "../character_tests.rs"]
pub(crate) mod tests;
