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
mod gossip;
mod items;
mod lifecycle;
mod query;
mod session_state;
mod vendor;
mod visibility;
mod world_entry;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;

use rand::Rng;
use tracing::{debug, info, trace, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::unit::{
    NPCFlags1, SheathState, UNIT_FLAGS_ALLOWED_LIKE_CPP, UNIT_FLAGS2_ALLOWED_LIKE_CPP,
    UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags,
};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, CreatureFlagsExtra, CreatureRandomMovementType,
    EnchantmentSlot, InventoryResult, InventoryType, ItemBondingType, ItemContext,
    ItemExtendedCostFlags, ItemFieldFlags, ItemFlags, ItemFlags2, ItemModifier, ItemUpdateState,
    ItemVendorType, PowerType, Team, TypeId, TypeMask, UnitStandStateType,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_crypto::rsa_sign::rsa_sign_connect_to;
#[cfg(test)]
use wow_data::PlayerCreatePositionLikeCpp;
use wow_data::{
    ConditionEntriesByTypeStore, ConditionId, CurrencyTypesStore, HotfixRecordStatus,
    ItemExtendedCostStore, PlayerConditionContextLikeCpp, PlayerConditionStore,
    PlayerCreateInfoLikeCpp, PlayerStatSystemInputLikeCpp, PlayerStatSystemProjectionLikeCpp,
    TaxiPathNodeEntry, TaxiPathNodeStore, calculate_player_stat_system_like_cpp,
    hotfix_locale_mask, is_player_meeting_condition_like_cpp,
};
use wow_database::{
    CharStatements, CharacterDatabase, LoginStatements, PreparedStatement, SqlResult,
    SqlTransaction, StatementDef, WorldDatabase, WorldStatements,
};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BUYBACK_SLOT_START, Corpse, CorpseCustomizationChoice,
    CorpseType, CreatureAddonLifecycleRecordLikeCpp, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_QUESTGIVER, GameObjectTemplateData, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START, MAX_BAG_SIZE, MAX_GAMEOBJECT_DATA, MovementGeneratorType, NULL_BAG,
    NULL_SLOT, REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SendNewItemDelivery,
    SendNewItemDisplayText, SendNewItemInstancePlan, SendNewItemModifier, SendNewItemPlan,
    SocketedGem, SwapItemPreflightResult, WorldObject, is_bank_pos, is_child_equipment_pos,
    is_equipment_pos, is_inventory_pos, item_can_go_into_bag,
    normalize_creature_chase_movement_type_like_cpp,
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
use wow_packet::{ClientPacket, WorldPacket};

use crate::handlers::quest::RepresentedQuestGiverStatusSourceLikeCpp;
use crate::map_manager::{
    terrain_grid_area_id_for_position_like_cpp, zone_and_area_for_position_like_cpp,
};
use crate::reputation::mgr::CharacterReputationRowLikeCpp;
use crate::session::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, CharacterPetAuraEffectRowLikeCpp,
    CharacterPetAuraRowLikeCpp, CharacterPetDeclinedNamesRowLikeCpp,
    CharacterPetSpellChargeRowLikeCpp, CharacterPetSpellCooldownRowLikeCpp,
    CharacterPetSpellRowLikeCpp, CharacterPetStableRowLikeCpp, GLOBAL_CACHE_MASK_LIKE_CPP,
    REST_STATE_NORMAL_LIKE_CPP, REST_STATE_RAF_LINKED_LIKE_CPP, RepresentedAlterAppearanceLikeCpp,
    RepresentedAutoUnequipOffhandLikeCpp, RepresentedBankItemMoveLikeCpp,
    RepresentedConfirmBarbersChoiceLikeCpp, RepresentedGameObjectUseState,
    RepresentedHomebindLikeCpp, RepresentedQuestObjectiveProgressEventLikeCpp,
    RepresentedVoidStorageItemLikeCpp, SpellCastMetadata,
};
#[cfg(test)]
use wow_entities::GAMEOBJECT_TYPE_GOOBER;

// ── Handler registration ────────────────────────────────────────────

const GO_SPAWN_TEMPLATE_DATA_START: usize = 16;
const GO_SPAWN_PHASE_USE_FLAGS_COLUMN: usize = GO_SPAWN_TEMPLATE_DATA_START + MAX_GAMEOBJECT_DATA;
const GO_SPAWN_PHASE_ID_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 1;
const GO_SPAWN_PHASE_GROUP_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 2;
const GO_SPAWN_TERRAIN_SWAP_MAP_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 3;
const GO_SPAWN_EFFECTIVE_FLAGS_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 4;
const GO_SPAWN_EFFECTIVE_FACTION_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 5;
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
const GO_SPAWN_OVERRIDE_SOURCE_KNOWN_COLUMN: usize = GO_SPAWN_PHASE_USE_FLAGS_COLUMN + 6;
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

#[derive(Debug, Clone, Default)]
struct RepresentedPlayerGearStatsLikeCpp {
    stats: [i32; 5],
    attack_power: i32,
    ranged_attack_power: i32,
    health: i32,
    mana: i32,
    combat_ratings: [i32; 32],
    spell_power: i32,
    armor: i32,
    mana_regen_bonus: i32,
    shield_block_base_mod: i32,
    shield_block_value: u32,
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

#[derive(Debug, Clone)]
struct ExistingStorageStackUpdateLikeCpp {
    item: InventoryItem,
    bag: u8,
    slot: u8,
    new_count: u32,
}

#[derive(Debug, Clone)]
struct InventoryStorageMovePlanLikeCpp {
    source_bag: u8,
    source_slot: u8,
    source: InventoryItem,
    source_count: u32,
    existing_updates: Vec<ExistingStorageStackUpdateLikeCpp>,
    moved_destination: Option<(u8, u8, u32)>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemStorageMutablePersistenceLikeCpp {
    db_guid: u64,
    count: u32,
    expiration: u32,
    charges: String,
    flags: u32,
    enchantments: String,
    durability: u32,
    played_time: u32,
}

fn loaded_item_random_properties_like_cpp(
    random_properties_id: i32,
    random_properties_seed: i32,
    item_random_properties_store: Option<&wow_data::ItemRandomPropertiesStore>,
    item_random_suffix_store: Option<&wow_data::ItemRandomSuffixStore>,
) -> Option<LoadedItemRandomPropertiesLikeCpp> {
    if random_properties_id > 0 {
        item_random_properties_store?
            .get(random_properties_id as u32)
            .map(|_| LoadedItemRandomPropertiesLikeCpp {
                id: random_properties_id,
                seed: 0,
            })
    } else if random_properties_id < 0 {
        item_random_suffix_store?
            .get(random_properties_id.unsigned_abs())
            .map(|_| LoadedItemRandomPropertiesLikeCpp {
                id: random_properties_id,
                seed: random_properties_seed,
            })
    } else {
        None
    }
}

fn apply_loaded_item_instance_fields_like_cpp(
    item: &mut wow_entities::Item,
    enchantments: &[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT],
    random_properties: Option<LoadedItemRandomPropertiesLikeCpp>,
) {
    if let Some(random_properties) = random_properties {
        item.set_random_properties_id(random_properties.id);
        item.set_property_seed(random_properties.seed);
    }

    for (slot_index, enchantment) in enchantments.iter().enumerate() {
        let Some(slot) = <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(slot_index)
        else {
            continue;
        };
        item.set_enchantment(
            slot,
            enchantment.id,
            enchantment.duration,
            enchantment.charges,
        );
    }
}

fn loaded_item_spell_charges_like_cpp(
    charges: &str,
    effect_count: usize,
) -> [i32; wow_entities::MAX_ITEM_SPELLS] {
    let mut values = [0; wow_entities::MAX_ITEM_SPELLS];
    for (target, token) in values
        .iter_mut()
        .take(effect_count.min(wow_entities::MAX_ITEM_SPELLS))
        .zip(charges.split_whitespace())
    {
        *target = token.parse::<i32>().unwrap_or(0);
    }
    values
}

fn apply_loaded_item_storage_mutable_fields_like_cpp(
    item: &mut wow_entities::Item,
    stored_expiration: u32,
    template_expiration: u32,
    charges: &str,
    effect_count: usize,
) -> bool {
    let expiration_needs_save = (template_expiration == 0) != (stored_expiration == 0);
    item.set_expiration(if expiration_needs_save {
        template_expiration
    } else {
        stored_expiration
    });
    for (index, charge) in loaded_item_spell_charges_like_cpp(charges, effect_count)
        .into_iter()
        .enumerate()
    {
        item.set_spell_charges(index, charge);
    }
    expiration_needs_save
}

fn loaded_item_slot_applies_equipped_enchantments_like_cpp(slot: u8) -> bool {
    slot < INVENTORY_SLOT_BAG_END
}

fn loaded_socketed_gems_like_cpp(fields: [(i32, String, u8); 3]) -> Vec<SocketedGem> {
    let Some(last_populated_socket) = fields.iter().rposition(|(item_id, _, _)| *item_id > 0)
    else {
        return Vec::new();
    };
    fields
        .into_iter()
        .take(last_populated_socket + 1)
        .map(|(item_id, bonuses, context)| {
            if item_id <= 0 {
                return SocketedGem::default();
            }
            SocketedGem {
                item_id,
                context,
                bonus_list_ids: bonuses
                    .split_whitespace()
                    .filter_map(|bonus| bonus.parse::<u16>().ok())
                    .take(16)
                    .collect(),
            }
        })
        .collect()
}

fn loaded_socketed_gem_create_updates_like_cpp(
    gems: &[SocketedGem],
) -> Vec<wow_packet::packets::update::SocketedGemValuesUpdate> {
    gems.iter()
        .map(|gem| {
            let mut bonus_list_ids = [0; 16];
            for (target, source) in bonus_list_ids.iter_mut().zip(&gem.bonus_list_ids) {
                *target = *source;
            }
            wow_packet::packets::update::SocketedGemValuesUpdate {
                socketed_gem_mask: 0x000F_FFFF,
                item_id: gem.item_id,
                context: gem.context,
                bonus_list_ids,
            }
        })
        .collect()
}

fn loaded_item_effective_enchantments_like_cpp(
    loaded_enchantments: Option<&[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]>,
    random_properties_id: i32,
    item_random_properties_store: Option<&wow_data::ItemRandomPropertiesStore>,
    item_random_suffix_store: Option<&wow_data::ItemRandomSuffixStore>,
) -> [ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT] {
    let mut values = [ItemEnchantmentValuesUpdate::default(); wow_entities::MAX_ENCHANTMENT_SLOT];

    if random_properties_id > 0 {
        if let Some(entry) =
            item_random_properties_store.and_then(|store| store.get(random_properties_id as u32))
        {
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                values[EnchantmentSlot::Property2 as usize + offset].id =
                    i32::from(*enchantment_id);
            }
        }
    } else if random_properties_id < 0 {
        if let Some(entry) = item_random_suffix_store
            .and_then(|store| store.get(random_properties_id.unsigned_abs()))
        {
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                values[EnchantmentSlot::Property0 as usize + offset].id =
                    i32::from(*enchantment_id);
            }
        }
    }

    // C++ Item::LoadFromDB synthesizes random-property slots first, then a
    // correctly sized persisted enchantment array overwrites every slot. A
    // valid all-zero array is therefore authoritative; only a missing or
    // malformed array keeps the synthesized fallback.
    if let Some(loaded_enchantments) = loaded_enchantments {
        values = *loaded_enchantments;
    }

    values
}

fn loaded_item_enchantments_like_cpp(
    enchantments: &str,
) -> Option<[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]> {
    let mut values = [ItemEnchantmentValuesUpdate::default(); wow_entities::MAX_ENCHANTMENT_SLOT];
    let tokens: Vec<&str> = enchantments.split_whitespace().collect();
    if tokens.len() != wow_entities::MAX_ENCHANTMENT_SLOT * ITEM_ENCHANTMENT_DB_FIELDS {
        return None;
    }

    for slot_index in 0..wow_entities::MAX_ENCHANTMENT_SLOT {
        let base = slot_index * ITEM_ENCHANTMENT_DB_FIELDS;
        values[slot_index] = ItemEnchantmentValuesUpdate {
            item_enchantment_mask: 0,
            id: tokens[base].parse::<i32>().unwrap_or(0),
            duration: tokens[base + 1].parse::<u32>().unwrap_or(0),
            charges: tokens[base + 2].parse::<i16>().unwrap_or(0),
            field_a: 0,
            field_b: 0,
        };
    }

    Some(values)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoginWorldStateTemplateLikeCpp {
    id: i32,
    default_value: i32,
    map_ids: BTreeSet<i32>,
    area_ids: BTreeSet<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CharacterLoginLocationLikeCpp {
    map_id: u32,
    /// C++ `m_homebindAreaId`, loaded from `character_homebind.zoneId`.
    /// This belongs to the bind packet and is distinct from the current
    /// terrain-derived zone/area. Battleground join positions leave it unset.
    bind_area_id: Option<u32>,
    position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CharacterBattlegroundLoginDataLikeCpp {
    entry_point: CharacterLoginLocationLikeCpp,
}

fn usable_character_login_location_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    map_store: Option<&wow_data::MapStore>,
) -> bool {
    location.map_id != u32::from(u16::MAX)
        && u16::try_from(location.map_id).is_ok()
        && location.position.is_valid_map_coord_like_cpp()
        && map_store.is_some_and(|store| store.get(location.map_id).is_some())
}

fn usable_character_homebind_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    map_store: Option<&wow_data::MapStore>,
    session_expansion: u8,
) -> bool {
    usable_character_login_location_like_cpp(location, map_store)
        && location.bind_area_id.is_some()
        && map_store
            .and_then(|store| store.get(location.map_id))
            .is_some_and(|entry| {
                !entry.is_instanceable_like_cpp() && session_expansion >= entry.expansion_like_cpp()
            })
}

fn default_graveyard_safe_loc_ids_for_race_like_cpp(race: u8) -> [Option<u32>; 2] {
    const RACE_PANDAREN_NEUTRAL_LIKE_CPP: u8 = 24;
    const WANDERING_ISLE_STARTING_GRAVEYARD_LIKE_CPP: u32 = 3295;

    [
        wow_data::GraveyardStore::default_graveyard_safe_loc_id_like_cpp(player_team_for_race_cpp(
            race,
        ) as u32),
        (race == RACE_PANDAREN_NEUTRAL_LIKE_CPP)
            .then_some(WANDERING_ISLE_STARTING_GRAVEYARD_LIKE_CPP),
    ]
}

fn first_login_creation_homebind_like_cpp(
    player_create_info: PlayerCreateInfoLikeCpp,
    create_mode: u8,
) -> Option<CharacterLoginLocationLikeCpp> {
    let create_position = if create_mode == wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP {
        player_create_info
            .create_position_npe
            .unwrap_or(player_create_info.create_position)
    } else {
        player_create_info.create_position
    };

    create_position
        .transport_guid
        .is_none()
        .then_some(CharacterLoginLocationLikeCpp {
            map_id: create_position.map_id,
            bind_area_id: None,
            position: create_position.position,
        })
}

fn zone_and_area_from_area_id_like_cpp(
    area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> (u32, u32) {
    let zone_id = area_store
        .and_then(|store| store.get(area_id))
        .filter(|area| area.parent_area_id != 0 && area.is_subzone_like_cpp())
        .map(|area| u32::from(area.parent_area_id))
        .unwrap_or(area_id);
    (zone_id, area_id)
}

fn login_location_zone_area_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    resolve_terrain: impl FnOnce(u32, Position) -> std::io::Result<(u32, u32)>,
) -> std::io::Result<(u32, u32)> {
    resolve_terrain(location.map_id, location.position)
}

fn login_bind_point_update_like_cpp(homebind: CharacterLoginLocationLikeCpp) -> BindPointUpdate {
    let bind_area_id = homebind
        .bind_area_id
        .expect("validated character homebind must have an area ID");
    BindPointUpdate {
        x: homebind.position.x,
        y: homebind.position.y,
        z: homebind.position.z,
        map_id: homebind.map_id,
        area_id: bind_area_id,
    }
}

fn battleground_login_fallback_location_like_cpp(
    battleground_data: Option<CharacterBattlegroundLoginDataLikeCpp>,
    homebind: Option<CharacterLoginLocationLikeCpp>,
    map_store: Option<&wow_data::MapStore>,
) -> Option<CharacterLoginLocationLikeCpp> {
    battleground_data
        .map(|data| data.entry_point)
        .filter(|location| usable_character_login_location_like_cpp(*location, map_store))
        .or_else(|| {
            homebind
                .filter(|location| usable_character_login_location_like_cpp(*location, map_store))
        })
}

fn parse_login_world_state_map_ids_like_cpp(
    map_ids_csv: &str,
    map_exists: impl Fn(i32) -> bool,
) -> BTreeSet<i32> {
    let mut map_ids = BTreeSet::new();
    for token in map_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(map_id) = token.trim().parse::<i32>() else {
            continue;
        };
        if map_id != WORLDSTATE_ANY_MAP_LIKE_CPP && !map_exists(map_id) {
            continue;
        }
        map_ids.insert(map_id);
    }
    map_ids
}

fn parse_login_world_state_area_ids_like_cpp(
    area_ids_csv: &str,
    map_ids: &BTreeSet<i32>,
    area_store: Option<&wow_data::AreaTableStore>,
) -> BTreeSet<u32> {
    let mut area_ids = BTreeSet::new();
    for token in area_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(area_id) = token.trim().parse::<u32>() else {
            continue;
        };
        let Some(area) = area_store.and_then(|store| store.get(area_id)) else {
            continue;
        };
        if !map_ids.contains(&i32::from(area.continent_id)) {
            continue;
        }
        area_ids.insert(area_id);
    }
    area_ids
}

fn build_initial_world_states_like_cpp(
    templates: impl IntoIterator<Item = LoginWorldStateTemplateLikeCpp>,
    saved_values: impl IntoIterator<Item = (i32, i32)>,
    map_id: i32,
    player_area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> Vec<(i32, i32)> {
    let mut template_by_id = BTreeMap::new();
    let mut realm_values = BTreeMap::new();
    let mut map_values_by_map: BTreeMap<i32, BTreeMap<i32, i32>> = BTreeMap::new();

    for template in templates {
        if template.map_ids.is_empty() {
            realm_values.insert(template.id, template.default_value);
        } else {
            for &template_map_id in &template.map_ids {
                map_values_by_map
                    .entry(template_map_id)
                    .or_default()
                    .insert(template.id, template.default_value);
            }
        }
        template_by_id.insert(template.id, template);
    }

    for (world_state_id, value) in saved_values {
        let Some(template) = template_by_id.get(&world_state_id) else {
            continue;
        };
        if template.map_ids.is_empty() {
            realm_values.insert(world_state_id, value);
        } else {
            for &template_map_id in &template.map_ids {
                map_values_by_map
                    .entry(template_map_id)
                    .or_default()
                    .insert(world_state_id, value);
            }
        }
    }

    let mut out = Vec::new();
    out.extend(realm_values);

    for lookup_map_id in [WORLDSTATE_ANY_MAP_LIKE_CPP, map_id] {
        let Some(values) = map_values_by_map.get(&lookup_map_id) else {
            continue;
        };
        for (&world_state_id, &value) in values {
            if let Some(template) = template_by_id.get(&world_state_id) {
                if !template.area_ids.is_empty()
                    && !template.area_ids.iter().any(|required_area_id| {
                        area_store.is_some_and(|store| {
                            store.is_in_area_like_cpp(player_area_id, *required_area_id)
                        })
                    })
                {
                    continue;
                }
            }
            out.push((world_state_id, value));
        }
    }

    out
}

/// Apply the realm-wide PvP-season world states the C++ `WorldStateMgr` seeds and
/// `FillInitialWorldStates` always sends (World.cpp:1363-1364, 2300-2301):
/// `WS_CURRENT_PVP_SEASON_ID` (3191) = `in_progress ? season_id : 0`, and
/// `WS_PREVIOUS_PVP_SEASON_ID` (3901) = `season_id - (in_progress ? 1 : 0)`
/// (SharedDefines.h:8081-8082). Overrides the value in place when the id is already
/// present (preserving order), else appends it. Rust previously shipped both as 0.
fn apply_pvp_season_world_states_like_cpp(
    states: &mut Vec<(i32, i32)>,
    arena_season_id: i32,
    arena_season_in_progress: bool,
) {
    const WS_CURRENT_PVP_SEASON_ID: i32 = 3191;
    const WS_PREVIOUS_PVP_SEASON_ID: i32 = 3901;

    let current = if arena_season_in_progress {
        arena_season_id
    } else {
        0
    };
    let previous = arena_season_id - i32::from(arena_season_in_progress);

    for (id, value) in [
        (WS_CURRENT_PVP_SEASON_ID, current),
        (WS_PREVIOUS_PVP_SEASON_ID, previous),
    ] {
        if let Some(entry) = states.iter_mut().find(|(state_id, _)| *state_id == id) {
            entry.1 = value;
        } else {
            states.push((id, value));
        }
    }
}
const CREATURE_SPAWN_ROOTED_COLUMN: usize = 35;
const CREATURE_SPAWN_CHASE_MOVEMENT_TYPE_COLUMN: usize = 36;
const CREATURE_SPAWN_RANDOM_MOVEMENT_TYPE_COLUMN: usize = 37;
const CREATURE_SPAWN_INTERACTION_PAUSE_TIMER_COLUMN: usize = 38;
const CREATURE_SPAWN_WANDER_DISTANCE_COLUMN: usize = 39;
const CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN: usize = 40;
const CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN: usize = 41;
const CREATURE_SPAWN_DISPLAY_SCALE_COLUMN: usize = 42;
const CREATURE_SPAWN_CLASSIFICATION_COLUMN: usize = 43;
const CREATURE_SPAWN_REGEN_HEALTH_COLUMN: usize = 44;
const CREATURE_SPAWN_NPC_FLAGS_OVERRIDE_COLUMN: usize = 45;
const CREATURE_SPAWN_UNIT_FLAGS_OVERRIDE_COLUMN: usize = 46;
const CREATURE_SPAWN_UNIT_FLAGS2_OVERRIDE_COLUMN: usize = 47;
const CREATURE_SPAWN_UNIT_FLAGS3_OVERRIDE_COLUMN: usize = 48;
const CREATURE_SPAWN_EQUIPMENT_ID_COLUMN: usize = 49;
const CREATURE_SPAWN_RESPAWN_DELAY_SECS_COLUMN: usize = 50;
const CREATURE_SPAWN_DIFFICULTIES_COLUMN: usize = 51;
const CREATURE_SPAWN_SCRIPT_NAME_COLUMN: usize = 52;
const CREATURE_SPAWN_STRING_ID_COLUMN: usize = 53;
const CREATURE_SPAWN_VEHICLE_ID_COLUMN: usize = 54;
const WAYPOINT_MOTION_TYPE_LIKE_CPP: u8 = 2;
const TACT_KEY_TABLE_HASH_LIKE_CPP: u32 = 0xDF2F_53CF;
const QUEST_GIVER_STATUS_TRACKED_QUERY_MAX_GUIDS_LIKE_CPP: u32 = 1000;
const MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP: f32 = 20.0;
// C++ ObjectDefines.h: DEFAULT_VISIBILITY_DISTANCE = VISIBILITY_DISTANCE_NORMAL = 100 yards.
// Wider values here make the SQL fallback load whole areas and can crash the 3.4.3 client.
const DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP: f32 = crate::map_manager::VISIBILITY_RADIUS;
const DIFFICULTY_NORMAL_LIKE_CPP: u8 = 1;
const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u8 = 14;
const RESPONSE_SUCCESS_LIKE_CPP: u8 = 0;
const CHAR_CREATE_ERROR_LIKE_CPP: u8 = 25;
const CHAR_CREATE_NAME_IN_USE_LIKE_CPP: u8 = 27;
const CHAR_NAME_NO_NAME_LIKE_CPP: u8 = 92;
const CHAR_NAME_TOO_SHORT_LIKE_CPP: u8 = 93;
const CHAR_NAME_TOO_LONG_LIKE_CPP: u8 = 94;
const CHAR_NAME_INVALID_CHARACTER_LIKE_CPP: u8 = 95;
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
const DIFFICULTY_10_N_LIKE_CPP: u8 = 3;
const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP: u8 = 15;
const TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP: i32 = 0x1;
const TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP: i32 = 0x2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnumCharacterFlagsLikeCpp {
    flags: u32,
    flags2: u32,
    first_login: bool,
}

fn enum_character_effective_player_flags_like_cpp(player_flags: u32, at_login_flags: u16) -> u32 {
    if (at_login_flags & AT_LOGIN_RESURRECT_LIKE_CPP) != 0 {
        player_flags & !PLAYER_FLAGS_GHOST_LIKE_CPP
    } else {
        player_flags
    }
}

/// C++ anchors:
/// - `Server/Packets/CharacterPackets.cpp:118-145`
/// - `Miscellaneous/SharedDefines.h:1019-1061`
fn enum_character_flags_like_cpp(
    player_flags: u32,
    at_login_flags: u16,
    banned_guid: u64,
    declined_genitive: Option<&str>,
    declined_names_used: bool,
) -> EnumCharacterFlagsLikeCpp {
    let player_flags = enum_character_effective_player_flags_like_cpp(player_flags, at_login_flags);
    let mut flags = 0;

    if (player_flags & PLAYER_FLAGS_GHOST_LIKE_CPP) != 0 {
        flags |= CHARACTER_FLAG_GHOST_LIKE_CPP;
    }
    if (at_login_flags & AT_LOGIN_RENAME_LIKE_CPP) != 0 {
        flags |= CHARACTER_FLAG_RENAME_LIKE_CPP;
    }
    if banned_guid != 0 {
        flags |= CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP;
    }
    if declined_names_used && declined_genitive.is_some_and(|name| !name.is_empty()) {
        flags |= CHARACTER_FLAG_DECLINED_LIKE_CPP;
    }

    let flags2 = if (at_login_flags & AT_LOGIN_CUSTOMIZE_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP
    } else if (at_login_flags & AT_LOGIN_CHANGE_FACTION_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP
    } else if (at_login_flags & AT_LOGIN_CHANGE_RACE_LIKE_CPP) != 0 {
        CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP
    } else {
        0
    };

    EnumCharacterFlagsLikeCpp {
        flags,
        flags2,
        first_login: (at_login_flags & AT_LOGIN_FIRST_LIKE_CPP) != 0,
    }
}

/// C++ anchor: `Server/Packets/CharacterPackets.cpp:147-156`.
fn enum_character_pet_data_like_cpp(
    player_flags: u32,
    at_login_flags: u16,
    class_id: u8,
    pet_entry: u32,
    pet_display_id: u32,
    pet_level: u32,
    creature_templates: Option<&wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
) -> (u32, u32, u32) {
    let player_flags = enum_character_effective_player_flags_like_cpp(player_flags, at_login_flags);
    if (player_flags & PLAYER_FLAGS_GHOST_LIKE_CPP) != 0
        || !matches!(
            class_id,
            CLASS_WARLOCK_LIKE_CPP | CLASS_HUNTER_LIKE_CPP | CLASS_DEATH_KNIGHT_LIKE_CPP
        )
    {
        return (0, 0, 0);
    }

    creature_templates
        .and_then(|store| store.get(pet_entry))
        .map(|template| (pet_display_id, pet_level, template.family))
        .unwrap_or((0, 0, 0))
}

fn enum_character_query_statements_like_cpp(
    declined_names_used: bool,
) -> (CharStatements, CharStatements) {
    (
        CharStatements::DEL_EXPIRED_BANS,
        if declined_names_used {
            CharStatements::SEL_ENUM_DECLINED_NAME
        } else {
            CharStatements::SEL_ENUM
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MapTransportCreateLikeCpp {
    guid_low: u32,
    entry: u32,
    display_id: u32,
    scale: f32,
    taxi_path_id: u16,
    move_speed: u32,
    accel_rate: u32,
    allow_stopping: bool,
    phase_use_flags: u8,
    phase_id: u16,
    phase_group_id: u32,
    gameobject_flags: u32,
    faction_template: i32,
}

fn map_transport_create_from_row_like_cpp(result: &SqlResult) -> MapTransportCreateLikeCpp {
    MapTransportCreateLikeCpp {
        guid_low: result
            .try_read::<i64>(0)
            .map(|value| value.max(0) as u32)
            .or_else(|| result.try_read::<u32>(0))
            .unwrap_or(0),
        entry: result
            .try_read::<i32>(1)
            .map(|value| value.max(0) as u32)
            .or_else(|| result.try_read::<u32>(1))
            .unwrap_or(0),
        phase_use_flags: result
            .try_read::<u8>(2)
            .or_else(|| result.try_read::<i16>(2).map(|value| value.max(0) as u8))
            .unwrap_or(0),
        phase_id: result
            .try_read::<u16>(3)
            .or_else(|| result.try_read::<i32>(3).map(|value| value.max(0) as u16))
            .unwrap_or(0),
        phase_group_id: result
            .try_read::<u32>(4)
            .or_else(|| result.try_read::<i32>(4).map(|value| value.max(0) as u32))
            .unwrap_or(0),
        display_id: result
            .try_read::<i32>(5)
            .map(|value| value.max(0) as u32)
            .or_else(|| result.try_read::<u32>(5))
            .unwrap_or(0),
        scale: result.try_read::<f32>(6).unwrap_or(1.0),
        taxi_path_id: result
            .try_read::<i32>(7)
            .map(|value| value.max(0) as u16)
            .or_else(|| result.try_read::<u16>(7))
            .unwrap_or(0),
        move_speed: result
            .try_read::<i32>(8)
            .map(|value| value.max(1) as u32)
            .or_else(|| result.try_read::<u32>(8))
            .unwrap_or(1),
        accel_rate: result
            .try_read::<i32>(9)
            .map(|value| value.max(1) as u32)
            .or_else(|| result.try_read::<u32>(9))
            .unwrap_or(1),
        allow_stopping: result
            .try_read::<i32>(10)
            .map(|value| value != 0)
            .or_else(|| result.try_read::<u8>(10).map(|value| value != 0))
            .unwrap_or(false),
        gameobject_flags: result
            .try_read::<i64>(11)
            .map(|value| value.max(0) as u32)
            .or_else(|| result.try_read::<u32>(11))
            .unwrap_or(0),
        faction_template: result
            .try_read::<i64>(12)
            .map(|value| value as i32)
            .or_else(|| result.try_read::<i32>(12))
            .unwrap_or(0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TransportCreatePositionLikeCpp {
    map_id: u16,
    position: Position,
    timer_ms: u32,
    total_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PersistedTransportLoginLikeCpp {
    guid: ObjectGuid,
    map_id: u16,
    offset: Position,
    world_position: Position,
    transport_position: TransportCreatePositionLikeCpp,
    transport_create: MapTransportCreateLikeCpp,
}

fn validate_persisted_transport_login_like_cpp(
    guid: ObjectGuid,
    offset: Position,
    transport_position: TransportCreatePositionLikeCpp,
    transport_create: MapTransportCreateLikeCpp,
) -> Option<PersistedTransportLoginLikeCpp> {
    // C++ Player::LoadFromDB first converts the saved passenger offset to
    // world coordinates, then rejects invalid world coordinates and transport
    // offsets outside the hard ±250-yard transport-size limit.
    if !offset.x.is_finite()
        || !offset.y.is_finite()
        || !offset.z.is_finite()
        || !offset.orientation.is_finite()
        || offset.x.abs() > 250.0
        || offset.y.abs() > 250.0
        || offset.z.abs() > 250.0
    {
        return None;
    }

    let world_position =
        wow_entities::calculate_passenger_position(offset, transport_position.position);
    world_position
        .is_valid_map_coord_like_cpp()
        .then_some(PersistedTransportLoginLikeCpp {
            guid,
            map_id: transport_position.map_id,
            offset,
            world_position,
            transport_position,
            transport_create,
        })
}

fn transport_route_contains_saved_map_like_cpp(
    route_map_ids: impl IntoIterator<Item = u16>,
    saved_map_id: u16,
) -> bool {
    route_map_ids
        .into_iter()
        .any(|map_id| map_id == saved_map_id)
}

fn map_transport_create_block_like_cpp(
    transport: MapTransportCreateLikeCpp,
    path_position: TransportCreatePositionLikeCpp,
    now_ms: u32,
) -> UpdateBlock {
    let transport_guid =
        ObjectGuid::create_transport(HighGuid::Transport, transport.guid_low as i64);
    let path_progress =
        ((path_position.timer_ms as f32 / path_position.total_time_ms as f32) * 65535.0) as u32;
    let create_data = GameObjectCreateData {
        guid: transport_guid,
        entry: transport.entry,
        dynamic_flags: path_progress << 16,
        display_id: transport.display_id,
        go_type: GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
        position: path_position.position,
        rotation: [0.0, 0.0, 0.0, 1.0],
        anim_progress: 255,
        state: if transport.allow_stopping {
            wow_entities::GoState::Active as i8
        } else {
            wow_entities::GoState::Ready as i8
        },
        art_kit: 0,
        created_by: ObjectGuid::EMPTY,
        faction_template: transport.faction_template,
        // Transport.cpp + GameObject flags:
        // GO_FLAG_TRANSPORT | GO_FLAG_NODESPAWN | GO_FLAG_MAP_OBJECT.
        gameobject_flags: transport.gameobject_flags | 0x0010_0028,
        world_effect_id: 0,
        scale: transport.scale,
        level: path_position.total_time_ms,
        parent_rotation: [0.0, 0.0, 0.0, 1.0],
    };
    UpdateObject::create_transport_block(create_data, now_ms)
}

#[derive(Default)]
struct InitTransportsPlanLikeCpp {
    own_transport: Option<(ObjectGuid, UpdateBlock)>,
    other_blocks: Vec<UpdateBlock>,
    other_visible_guids: Vec<ObjectGuid>,
    considered: usize,
    skipped_other_map: usize,
    skipped_missing_path: usize,
    skipped_phase: usize,
}

pub(crate) fn player_visibility_create_update_from_snapshot_like_cpp(
    player: &crate::session::directory::PlayerVisibilityCreateSnapshot,
    map_id: u16,
) -> UpdateObject {
    let max_mana = if player.power_type == PowerType::Mana as u8 {
        i64::from(player.max_power)
    } else {
        0
    };
    let combat = PlayerCombatStats {
        health: i64::from(player.current_health),
        max_health: i64::from(player.max_health),
        base_mana: player.base_mana,
        max_mana,
        ..PlayerCombatStats::default()
    };
    let mut update = UpdateObject::create_player_with_party_type(
        player.guid,
        player.race,
        player.class,
        player.sex,
        player.level,
        player.display_id,
        &player.position,
        map_id,
        player.zone_id,
        false,
        *player.visible_items,
        [ObjectGuid::EMPTY; 141],
        combat,
        Vec::new(),
        0,
        Vec::new(),
        player.party_member_party_type,
    );
    update.set_player_current_power0_like_cpp(i32::from(player.current_power));
    update.set_player_customizations_like_cpp(player.customizations.as_ref().clone());
    if let Some(transport) = player.transport.clone() {
        update.set_player_movement_transport_like_cpp(transport);
    }
    update
}

fn compose_init_self_create_blocks_like_cpp(
    player_update: &mut UpdateObject,
    item_creates: Vec<ItemCreateData>,
    own_transport: Option<(ObjectGuid, UpdateBlock)>,
    fellow_passenger_blocks: Vec<UpdateBlock>,
) -> Option<ObjectGuid> {
    if item_creates.is_empty() && own_transport.is_none() && fellow_passenger_blocks.is_empty() {
        return None;
    }

    let mut blocks = Vec::with_capacity(
        usize::from(own_transport.is_some())
            + item_creates.len()
            + player_update.blocks.len()
            + fellow_passenger_blocks.len(),
    );
    let own_transport_guid = own_transport.map(|(guid, block)| {
        blocks.push(block);
        guid
    });
    blocks.extend(
        item_creates
            .into_iter()
            .map(|create_data| UpdateBlock::CreateItem {
                update_type: UpdateType::CreateObject,
                guid: create_data.item_guid,
                create_data,
            }),
    );
    blocks.append(&mut player_update.blocks);
    blocks.extend(fellow_passenger_blocks);
    player_update.blocks = blocks;
    player_update.num_updates = player_update.blocks.len() as u32;
    own_transport_guid
}

fn object_guid_from_db_binary_like_cpp(raw: Vec<u8>) -> ObjectGuid {
    let Ok(bytes) = <[u8; 16]>::try_from(raw.as_slice()) else {
        return ObjectGuid::EMPTY;
    };
    ObjectGuid::from_raw_bytes(&bytes)
}

fn distance_3d_like_cpp(a: &TaxiPathNodeEntry, b: &TaxiPathNodeEntry) -> f32 {
    let dx = b.loc.x - a.loc.x;
    let dy = b.loc.y - a.loc.y;
    let dz = b.loc.z - a.loc.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn movement_time_ms_for_transport_segment_like_cpp(
    distance: f32,
    speed: u32,
    accel_rate: u32,
    accel_from_pause: bool,
) -> u32 {
    if distance <= 0.0 {
        return 0;
    }

    let speed = speed.max(1) as f32;
    let accel = accel_rate.max(1) as f32;
    if accel_from_pause {
        let accel_dist = 0.5 * speed * speed / accel;
        if accel_dist >= distance {
            ((distance * 2.0 / accel).sqrt() * 1000.0) as u32
        } else {
            (((distance - accel_dist) / speed + speed / accel) * 1000.0) as u32
        }
    } else {
        (distance / speed * 1000.0) as u32
    }
}

fn transport_position_for_login_like_cpp(
    nodes: &[TaxiPathNodeEntry],
    move_speed: u32,
    accel_rate: u32,
    now_ms: u32,
) -> Option<TransportCreatePositionLikeCpp> {
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort_by_key(|node| node.node_index);
    if sorted_nodes.len() < 2 {
        return None;
    }

    let mut legs: Vec<Vec<TaxiPathNodeEntry>> = Vec::new();
    let mut current_leg: Vec<TaxiPathNodeEntry> = Vec::new();
    let mut current_map = sorted_nodes[0].continent_id;
    let mut prev_node_was_teleport = false;

    for node in sorted_nodes {
        if !current_leg.is_empty() && (node.continent_id != current_map || prev_node_was_teleport) {
            legs.push(std::mem::take(&mut current_leg));
            current_map = node.continent_id;
        }
        prev_node_was_teleport = (node.flags & TAXI_PATH_NODE_FLAG_TELEPORT_LIKE_CPP) != 0;
        current_leg.push(node);
    }
    if !current_leg.is_empty() {
        legs.push(current_leg);
    }

    let mut leg_durations: Vec<u32> = Vec::with_capacity(legs.len());
    let mut total_time_ms = 0u32;
    for leg in &legs {
        let mut duration = 0u32;
        let mut accel_from_pause = false;
        for segment in leg.windows(2) {
            let distance = distance_3d_like_cpp(&segment[0], &segment[1]);
            duration = duration.saturating_add(movement_time_ms_for_transport_segment_like_cpp(
                distance,
                move_speed,
                accel_rate,
                accel_from_pause,
            ));
            let stop_delay = if (segment[1].flags & TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP) != 0 {
                segment[1].delay.saturating_mul(1000)
            } else {
                0
            };
            if stop_delay > 0 {
                duration = duration.saturating_add(stop_delay);
                accel_from_pause = true;
            } else {
                accel_from_pause = false;
            }
        }
        leg_durations.push(duration);
        total_time_ms = total_time_ms.saturating_add(duration);
    }

    if total_time_ms == 0 {
        return None;
    }

    let timer_ms = now_ms % total_time_ms;
    let mut leg_start_ms = 0u32;
    for (leg, leg_duration) in legs.iter().zip(leg_durations.iter().copied()) {
        let leg_end_ms = leg_start_ms.saturating_add(leg_duration);
        if timer_ms >= leg_end_ms {
            leg_start_ms = leg_end_ms;
            continue;
        }

        let mut leg_elapsed_ms = timer_ms.saturating_sub(leg_start_ms);
        let mut accel_from_pause = false;
        for segment in leg.windows(2) {
            let from = &segment[0];
            let to = &segment[1];
            let distance = distance_3d_like_cpp(from, to);
            let move_time = movement_time_ms_for_transport_segment_like_cpp(
                distance,
                move_speed,
                accel_rate,
                accel_from_pause,
            )
            .max(1);

            if leg_elapsed_ms <= move_time {
                let pct = (leg_elapsed_ms as f32 / move_time as f32).clamp(0.0, 1.0);
                let x = from.loc.x + (to.loc.x - from.loc.x) * pct;
                let y = from.loc.y + (to.loc.y - from.loc.y) * pct;
                let z = from.loc.z + (to.loc.z - from.loc.z) * pct;
                let orientation = (to.loc.y - from.loc.y).atan2(to.loc.x - from.loc.x) + PI;
                return Some(TransportCreatePositionLikeCpp {
                    map_id: from.continent_id,
                    position: Position::new(x, y, z, orientation),
                    timer_ms,
                    total_time_ms,
                });
            }
            leg_elapsed_ms = leg_elapsed_ms.saturating_sub(move_time);

            let stop_delay = if (to.flags & TAXI_PATH_NODE_FLAG_STOP_LIKE_CPP) != 0 {
                to.delay.saturating_mul(1000)
            } else {
                0
            };
            if stop_delay > 0 {
                if leg_elapsed_ms <= stop_delay {
                    let orientation = (to.loc.y - from.loc.y).atan2(to.loc.x - from.loc.x) + PI;
                    return Some(TransportCreatePositionLikeCpp {
                        map_id: to.continent_id,
                        position: Position::new(to.loc.x, to.loc.y, to.loc.z, orientation),
                        timer_ms,
                        total_time_ms,
                    });
                }
                leg_elapsed_ms = leg_elapsed_ms.saturating_sub(stop_delay);
                accel_from_pause = true;
            } else {
                accel_from_pause = false;
            }
        }

        if let Some(last) = leg.last() {
            return Some(TransportCreatePositionLikeCpp {
                map_id: last.continent_id,
                position: Position::new(last.loc.x, last.loc.y, last.loc.z, 0.0),
                timer_ms,
                total_time_ms,
            });
        }
    }

    None
}

fn bind_create_character_difficulties_like_cpp(stmt: &mut PreparedStatement) {
    stmt.set_u8(16, DIFFICULTY_NORMAL_LIKE_CPP);
    stmt.set_u8(17, DIFFICULTY_NORMAL_RAID_LIKE_CPP);
    stmt.set_u8(18, DIFFICULTY_10_N_LIKE_CPP);
}

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

fn optional_u64_column_like_cpp(row: &SqlResult, column: usize) -> Option<u64> {
    row.try_read::<Option<i64>>(column)
        .flatten()
        .map(|value| value as u64)
        .or_else(|| row.try_read::<Option<u64>>(column).flatten())
        .or_else(|| row.try_read::<i64>(column).map(|value| value as u64))
        .or_else(|| row.try_read::<u64>(column))
}

fn optional_u32_column_like_cpp(row: &SqlResult, column: usize) -> Option<u32> {
    row.try_read::<Option<u32>>(column)
        .flatten()
        .or_else(|| {
            row.try_read::<Option<i64>>(column)
                .flatten()
                .map(|value| value.max(0) as u32)
        })
        .or_else(|| row.try_read::<u32>(column))
        .or_else(|| row.try_read::<i64>(column).map(|value| value.max(0) as u32))
}

fn nonnegative_i64_to_u64_like_cpp(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn nonnegative_i32_to_u32_like_cpp(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
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

/// Default start position for a race.
/// Returns (map_id, x, y, z, orientation).
fn start_position(race: u8) -> (i32, f32, f32, f32, f32) {
    match race {
        1 => (0, -8949.95, -132.493, 83.5312, 0.0),       // Human
        2 => (1, -618.518, -4251.67, 38.718, 0.0),        // Orc
        3 => (0, -6240.32, 331.033, 382.758, 6.17716),    // Dwarf
        4 => (1, 10311.3, 832.463, 1326.41, 5.69632),     // NightElf
        5 => (0, 1676.71, 1678.31, 121.67, 2.70526),      // Undead
        6 => (1, -2917.58, -257.98, 52.9968, 0.0),        // Tauren
        7 => (0, -6240.32, 331.033, 382.758, 0.0),        // Gnome
        8 => (1, -618.518, -4251.67, 38.718, 0.0),        // Troll
        10 => (530, 10349.6, -6357.29, 33.4026, 5.31605), // BloodElf
        11 => (530, -3961.64, -13931.2, 100.615, 2.08364), // Draenei
        22 => (0, -8949.95, -132.493, 83.5312, 0.0),      // Worgen → Human
        _ => (0, -8949.95, -132.493, 83.5312, 0.0),       // Default: Human
    }
}

/// Default display ID for a race/sex combination.
pub(crate) fn default_display_id(race: u8, sex: u8) -> u32 {
    match (race, sex) {
        (1, 0) => 49,
        (1, 1) => 50, // Human M/F
        (2, 0) => 51,
        (2, 1) => 52, // Orc
        (3, 0) => 53,
        (3, 1) => 54, // Dwarf
        (4, 0) => 55,
        (4, 1) => 56, // NightElf
        (5, 0) => 57,
        (5, 1) => 58, // Undead
        (6, 0) => 59,
        (6, 1) => 60, // Tauren
        (7, 0) => 1563,
        (7, 1) => 1564, // Gnome
        (8, 0) => 1478,
        (8, 1) => 1479, // Troll
        (10, 0) => 15476,
        (10, 1) => 15475, // BloodElf
        (11, 0) => 16125,
        (11, 1) => 16126, // Draenei
        _ => 49,          // Default: Human Male
    }
}

/// Default zone ID for a starting position.
#[cfg_attr(not(test), allow(dead_code))]
fn start_zone(race: u8) -> i32 {
    match race {
        1 | 22 => 12, // Human / Worgen: Elwynn Forest
        2 | 8 => 14,  // Orc / Troll: Durotar
        3 | 7 => 1,   // Dwarf / Gnome: Dun Morogh
        4 => 141,     // NightElf: Teldrassil
        5 => 85,      // Undead: Tirisfal Glades
        6 => 215,     // Tauren: Mulgore
        10 => 3430,   // BloodElf: Eversong Woods
        11 => 3524,   // Draenei: Azuremyst Isle
        _ => 12,
    }
}

/// Default starting health and mana for a level 1 character by class.
fn default_health_mana(class: u8) -> (u32, u32) {
    match class {
        1 => (50, 0),   // Warrior — no mana
        2 => (52, 79),  // Paladin
        3 => (46, 85),  // Hunter (uses focus at high level, mana at 1)
        4 => (45, 0),   // Rogue — no mana
        5 => (52, 160), // Priest
        6 => (130, 0),  // Death Knight — no mana (runic power)
        7 => (47, 73),  // Shaman
        8 => (42, 200), // Mage
        9 => (43, 200), // Warlock
        11 => (54, 60), // Druid
        _ => (50, 100), // Default
    }
}

fn max_health_u32_like_cpp(max_health: i64) -> u32 {
    max_health.max(1).min(i64::from(u32::MAX)) as u32
}

fn restored_saved_health_like_cpp(saved_health: Option<u32>, max_health: i64) -> i64 {
    let max_health = max_health_u32_like_cpp(max_health);
    saved_health
        .map(|health| i64::from(health.min(max_health)))
        .unwrap_or(i64::from(max_health))
}

fn default_character_power1_like_cpp(class: u8, mana: u32) -> u32 {
    match primary_power_type_for_class_like_cpp(class) {
        PowerType::Energy => 100,
        _ => mana,
    }
}

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

const MAX_MONEY_AMOUNT: u64 = 99_999_999_999;
const MAX_VENDOR_ITEMS_CPP: usize = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VendorBuyItem {
    item_id: u32,
    item_type: i32,
    max_count: u32,
    incr_time: u32,
    player_condition_id: u32,
    has_vendor_conditions: bool,
    extended_cost: u32,
    buy_price: u64,
    max_durability: u32,
    buy_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VendorBuyTemplateBlock {
    BuyError(BuyResult),
    Silent,
}

fn vendor_buy_quantity_and_price(buy_price: u64, buy_count: u32, quantity: u32) -> (u32, u64) {
    if buy_price == 0 || quantity == 0 {
        return (quantity, 0);
    }

    let buy_price_per_item = buy_price as f64 / buy_count.max(1) as f64;
    let max_count = (MAX_MONEY_AMOUNT as f64 / buy_price_per_item) as u32;
    let quantity = quantity.min(max_count);
    let price = ((buy_price_per_item * quantity as f64) as u64).max(1);

    (quantity, price)
}

fn vendor_buy_coinage_update_like_cpp(buy_price: u64, remaining_gold: u64) -> Option<u64> {
    // C++ `_StoreOrEquipNewItem` calls `ModifyMoney(-price)`, whose first
    // branch returns without dirtying `ActivePlayerData::Coinage` when the
    // amount is zero (`Player.cpp::ModifyMoney`).
    (buy_price != 0).then_some(remaining_gold)
}

fn vendor_stored_new_item_flags_like_cpp(
    template: Option<&wow_entities::ItemStorageTemplate>,
    bag: u8,
    slot: u8,
) -> u32 {
    // C++ `StoreNewItem` marks the object new before `_StoreItem`, then the
    // store path applies the template bonding rule at its destination.
    let mut item = wow_entities::Item::new(0);
    if let Some(template) = template {
        item.set_bonding(template.bonding);
    }
    item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
        bag, slot,
    )));
    item.item_flags_bits()
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

fn relocate_bag_exchange_child_like_cpp(
    item: &mut wow_entities::Item,
    destination_bag_guid: ObjectGuid,
    destination_slot: u8,
) {
    item.set_container_guid(destination_bag_guid);
    item.set_contained_in(destination_bag_guid);
    item.set_slot(destination_slot);
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

const UPD_CHARACTER_MONEY_AND_BANK_SLOTS_LIKE_CPP: &str =
    "UPDATE characters SET money = ?, bankSlots = ? WHERE guid = ?";

fn bank_slot_purchase_update_statement_like_cpp(
    player_guid: ObjectGuid,
    new_money: u64,
    new_bank_slot_count: u8,
) -> PreparedStatement {
    let mut statement = PreparedStatement::new(UPD_CHARACTER_MONEY_AND_BANK_SLOTS_LIKE_CPP);
    statement.set_u64(0, new_money);
    statement.set_u8(1, new_bank_slot_count);
    statement.set_u64(2, player_guid.counter() as u64);
    statement
}

fn active_known_spell_for_send_like_cpp(spell_id: u32, active: u8, disabled: u8) -> Option<i32> {
    if spell_id > 0 && active != 0 && disabled == 0 {
        i32::try_from(spell_id).ok()
    } else {
        None
    }
}

fn loaded_spell_for_add_spell_side_effects_like_cpp(spell_id: u32, disabled: u8) -> Option<i32> {
    if spell_id > 0 && disabled == 0 {
        i32::try_from(spell_id).ok()
    } else {
        None
    }
}

fn apply_skill_rewarded_spell_changes_to_login_like_cpp(
    known_spells: &mut Vec<i32>,
    loaded_spell_side_effect_spells: &mut Vec<i32>,
    dependent_spells: &mut HashSet<i32>,
    removed_spells: &mut HashSet<i32>,
    changes: wow_data::SkillRewardedSpellChangesLikeCpp,
) {
    for spell_id in changes.remove {
        known_spells.retain(|known_spell| *known_spell != spell_id);
        loaded_spell_side_effect_spells.retain(|known_spell| *known_spell != spell_id);
        dependent_spells.remove(&spell_id);
        removed_spells.insert(spell_id);
    }
    for spell_id in changes.learn {
        removed_spells.remove(&spell_id);
        if !known_spells.contains(&spell_id) {
            known_spells.push(spell_id);
        }
        if !loaded_spell_side_effect_spells.contains(&spell_id) {
            loaded_spell_side_effect_spells.push(spell_id);
        }
        dependent_spells.insert(spell_id);
    }
}

const SKILL_UNARMED_LIKE_CPP: u16 = 162;
const SKILL_FIST_WEAPONS_LIKE_CPP: u16 = 473;

/// Pinned 3.4.3 C++ `Player::_LoadSkills` final Fist Weapons fixup.
///
/// `HasSkill(SKILL_FIST_WEAPONS)` is false for a zero-rank loaded row, so only
/// an active persisted Fist Weapons skill is synchronized. `SetSkill` removes
/// it when Unarmed is absent/zero; otherwise it copies the current Unarmed
/// value and the level-dependent maximum.
fn sync_loaded_fist_weapons_with_unarmed_like_cpp(
    skill_records: &mut HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
    skill_info_by_id: &mut BTreeMap<u16, wow_data::SkillInfoEntry>,
    level: u8,
) {
    let Some(mut fist_weapons) = skill_info_by_id
        .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
        .copied()
        .filter(|entry| entry.rank > 0)
    else {
        return;
    };

    let unarmed_rank = skill_info_by_id
        .get(&SKILL_UNARMED_LIKE_CPP)
        .map(|entry| entry.rank)
        .unwrap_or(0);
    fist_weapons.step = 0;
    fist_weapons.rank = unarmed_rank;
    fist_weapons.max_rank = if unarmed_rank == 0 {
        0
    } else {
        u16::from(level).saturating_mul(5)
    };
    fist_weapons.temp_bonus = 0;
    fist_weapons.perm_bonus = 0;
    skill_info_by_id.insert(SKILL_FIST_WEAPONS_LIKE_CPP, fist_weapons);

    if unarmed_rank == 0 {
        // C++ `SetSkill(..., newVal = 0, ...)` marks the persisted skill
        // deleted while leaving its cleared initial update-field slot.
        skill_records.remove(&SKILL_FIST_WEAPONS_LIKE_CPP);
    } else if let Some(skill_record) = skill_records.get_mut(&SKILL_FIST_WEAPONS_LIKE_CPP) {
        skill_record.value = fist_weapons.rank;
        skill_record.max = fist_weapons.max_rank;
    }
}

pub(crate) fn favorite_known_spells_for_send_like_cpp(
    known_spells: &[i32],
    favorite_spells: &HashSet<i32>,
) -> Vec<i32> {
    known_spells
        .iter()
        .copied()
        .filter(|spell_id| favorite_spells.contains(spell_id))
        .collect()
}

fn unix_now_secs_like_cpp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn remaining_ms_from_unix_secs_like_cpp(end_unix_secs: i64, now_unix_secs: i64) -> Option<u32> {
    let remaining_secs = end_unix_secs.checked_sub(now_unix_secs)?;
    if remaining_secs <= 0 {
        return None;
    }

    u32::try_from(remaining_secs.saturating_mul(1000)).ok()
}

fn spell_history_entry_from_db_like_cpp(
    spell_id: u32,
    item_id: u32,
    cooldown_end_unix_secs: i64,
    category_id: u32,
    category_end_unix_secs: i64,
    now_unix_secs: i64,
) -> Option<SpellHistoryEntry> {
    let cooldown_ms = remaining_ms_from_unix_secs_like_cpp(cooldown_end_unix_secs, now_unix_secs)?;
    let category_ms =
        remaining_ms_from_unix_secs_like_cpp(category_end_unix_secs, now_unix_secs).unwrap_or(0);

    Some(SpellHistoryEntry {
        spell_id,
        item_id,
        category: if category_ms > 0 { category_id } else { 0 },
        recovery_time_ms: if cooldown_ms > category_ms {
            cooldown_ms as i32
        } else {
            0
        },
        category_recovery_time_ms: category_ms as i32,
        mod_rate: 1.0,
        on_hold: false,
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CreatureAddonCreateFieldsLikeCpp {
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

fn spell_charge_entry_from_db_like_cpp(
    category_id: u32,
    first_recharge_end_unix_secs: i64,
    consumed_charges: u8,
    now_unix_secs: i64,
) -> Option<SpellChargeEntry> {
    let next_recovery_time_ms =
        remaining_ms_from_unix_secs_like_cpp(first_recharge_end_unix_secs, now_unix_secs)?;
    Some(SpellChargeEntry {
        category: category_id,
        next_recovery_time_ms,
        charge_mod_rate: 1.0,
        consumed_charges,
    })
}

fn vendor_buy_packet_quantity_to_cpp_count(quantity: i32) -> u32 {
    u32::from((quantity as u8).max(1))
}

fn vendor_buy_currency_packet_quantity_to_cpp_count(quantity: i32) -> u32 {
    (quantity as u32).max(1)
}

fn vendor_list_reaches_cpp_item_limit(count: usize) -> bool {
    count >= MAX_VENDOR_ITEMS_CPP
}

fn vendor_list_should_skip_currency_row(
    currency_store: Option<&CurrencyTypesStore>,
    item_id: i32,
    extended_cost: i32,
) -> bool {
    if extended_cost == 0 {
        return true;
    }

    !vendor_currency_type_is_known(currency_store, item_id as u32)
}

fn vendor_currency_type_is_known(
    currency_store: Option<&CurrencyTypesStore>,
    currency_id: u32,
) -> bool {
    currency_store.is_some_and(|store| store.has_record(currency_id))
}

fn vendor_buy_currency_quantity_block_result(
    max_count: u32,
    quantity: u32,
) -> Option<InventoryResult> {
    if max_count == 0 || quantity % max_count != 0 {
        Some(InventoryResult::CantBuyQuantity)
    } else {
        None
    }
}

fn vendor_buy_muid_to_cpp_slot(muid: i32) -> Option<u32> {
    let muid = muid as u32;
    if muid > 0 { Some(muid - 1) } else { None }
}

fn vendor_player_condition_failed_id_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> i32 {
    if player_condition_id == 0 {
        return 0;
    }

    let (Some(store), Some(context)) = (store, context) else {
        return player_condition_id as i32;
    };

    let Some(condition) = store.get(player_condition_id) else {
        return 0;
    };

    if is_player_meeting_condition_like_cpp(condition, &context) {
        0
    } else {
        player_condition_id as i32
    }
}

fn vendor_buy_player_condition_block_result_like_cpp(
    player_condition_id: u32,
    store: Option<&PlayerConditionStore>,
    context: Option<PlayerConditionContextLikeCpp<'_>>,
) -> Option<InventoryResult> {
    if vendor_player_condition_failed_id_like_cpp(player_condition_id, store, context) == 0 {
        None
    } else {
        Some(InventoryResult::ItemLocked)
    }
}

fn vendor_conditions_block_result(has_vendor_conditions: bool) -> Option<BuyResult> {
    if has_vendor_conditions {
        Some(BuyResult::CantFindItem)
    } else {
        None
    }
}

fn vendor_buy_required_reputation_block_result(
    required_reputation_faction: Option<u16>,
    required_reputation_rank: Option<i32>,
    player_reputation_rank: i32,
) -> Option<BuyResult> {
    if required_reputation_faction.unwrap_or(0) != 0
        && player_reputation_rank < required_reputation_rank.unwrap_or(0)
    {
        Some(BuyResult::ReputationRequire)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VendorExtendedCostBlock {
    Equip(InventoryResult),
    Buy(BuyResult),
    Silent,
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

fn vendor_buy_extended_cost_block_result(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    currency_store: Option<&CurrencyTypesStore>,
    has_item_count: impl Fn(u32, u32) -> bool,
    has_currency: impl Fn(u32, u32) -> bool,
    allow_currency_only_success: bool,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Option<VendorExtendedCostBlock> {
    if extended_cost == 0 {
        return None;
    }

    if quantity % buy_count.max(1) != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantBuyQuantity,
        ));
    }

    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Some(VendorExtendedCostBlock::Silent);
    };
    let stacks = quantity / buy_count.max(1);

    for (item_id, item_count) in extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
    {
        if item_id == 0 {
            continue;
        }

        let Ok(item_id) = u32::try_from(item_id) else {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        };
        let amount = u32::from(item_count).wrapping_mul(stacks);
        if !has_item_count(item_id, amount) {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    for (i, currency_id) in extended_cost_entry.currency_id.iter().copied().enumerate() {
        if currency_id == 0 {
            continue;
        }

        let currency_id = u32::from(currency_id);
        if !vendor_currency_type_is_known(currency_store, currency_id) {
            return Some(VendorExtendedCostBlock::Buy(BuyResult::CantFindItem));
        }

        if item_extended_cost_currency_requires_season_earned(extended_cost_entry.flags, i)
            || !has_currency(
                currency_id,
                extended_cost_entry.currency_count[i].wrapping_mul(stacks),
            )
        {
            return Some(VendorExtendedCostBlock::Equip(
                InventoryResult::VendorMissingTurnins,
            ));
        }
    }

    if extended_cost_entry.required_arena_rating != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::CantEquipRank,
        ));
    }

    if extended_cost_entry.min_faction_id != 0 {
        return Some(VendorExtendedCostBlock::Buy(BuyResult::ReputationRequire));
    }

    if extended_cost_entry.requires_guild() || extended_cost_entry.required_achievement != 0 {
        return Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ));
    }

    if allow_currency_only_success {
        None
    } else {
        Some(VendorExtendedCostBlock::Equip(
            InventoryResult::VendorMissingTurnins,
        ))
    }
}

fn vendor_buy_extended_cost_item_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .item_id
        .iter()
        .copied()
        .zip(extended_cost_entry.item_count.iter().copied())
        .filter(|(item_id, _)| *item_id > 0)
        .map(|(item_id, count)| {
            (
                u32::try_from(item_id).unwrap_or(0),
                u32::from(count).wrapping_mul(stacks),
            )
        })
        .collect()
}

fn vendor_buy_extended_cost_currency_costs(
    extended_cost_store: Option<&ItemExtendedCostStore>,
    extended_cost: u32,
    buy_count: u32,
    quantity: u32,
) -> Vec<(u32, u32)> {
    if extended_cost == 0 {
        return Vec::new();
    }
    let Some(extended_cost_entry) = extended_cost_store.and_then(|store| store.get(extended_cost))
    else {
        return Vec::new();
    };
    let stacks = quantity / buy_count.max(1);
    extended_cost_entry
        .currency_id
        .iter()
        .copied()
        .zip(extended_cost_entry.currency_count.iter().copied())
        .filter(|(currency_id, _)| *currency_id != 0)
        .map(|(currency_id, count)| (u32::from(currency_id), count.wrapping_mul(stacks)))
        .collect()
}

fn item_extended_cost_currency_requires_season_earned(
    flags: ItemExtendedCostFlags,
    currency_index: usize,
) -> bool {
    match currency_index {
        0 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
        1 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
        2 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
        3 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
        4 => flags.contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
        _ => false,
    }
}

fn vendor_buy_direct_store_block_result(
    bag: u8,
    slot: u8,
    _quantity: u32,
) -> Option<InventoryResult> {
    if (bag == NULL_BAG && slot == NULL_SLOT) || is_inventory_pos(bag, slot) {
        return None;
    }

    if is_equipment_pos(bag, slot) {
        return Some(InventoryResult::NotEquippable);
    }

    Some(InventoryResult::WrongSlot)
}

fn vendor_buy_stock_refill_count(
    current_count: u32,
    elapsed_secs: u64,
    incr_time: u32,
    buy_count: u32,
    max_count: u32,
) -> (u32, bool) {
    if max_count == 0 || current_count >= max_count || incr_time == 0 {
        // C++ assumes nonzero incrtime for finite stock; keep invalid DB rows from dividing by zero.
        return (current_count.min(max_count), current_count >= max_count);
    }

    let increments = elapsed_secs / u64::from(incr_time);
    if increments == 0 {
        return (current_count, false);
    }

    let restored = increments.saturating_mul(u64::from(buy_count.max(1)));
    let new_count = u64::from(current_count).saturating_add(restored);
    if new_count >= u64::from(max_count) {
        (max_count, true)
    } else {
        (new_count as u32, false)
    }
}

fn vendor_list_should_skip_sold_out(
    max_count: i32,
    current_count: u32,
    is_game_master: bool,
) -> bool {
    max_count > 0 && current_count == 0 && !is_game_master
}

fn vendor_list_item_refundable(
    item_flags: Option<ItemFlags>,
    max_stack_size: Option<u32>,
    extended_cost: i32,
) -> bool {
    extended_cost > 0
        && max_stack_size == Some(1)
        && item_flags.is_some_and(|flags| flags.contains(ItemFlags::ITEM_PURCHASE_RECORD))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadedItemRefundDecision {
    None,
    Valid {
        paid_money: u64,
        paid_extended_cost: u16,
    },
    Clear {
        new_flags: u32,
    },
}

fn loaded_item_refund_decision(
    item_flags: u32,
    played_time: u32,
    paid_money: Option<u64>,
    paid_extended_cost: Option<u16>,
) -> LoadedItemRefundDecision {
    let flags = ItemFieldFlags::from_bits_retain(item_flags);
    if !flags.contains(ItemFieldFlags::REFUNDABLE) {
        return LoadedItemRefundDecision::None;
    }

    let new_flags = (flags & !ItemFieldFlags::REFUNDABLE).bits();
    if played_time > 2 * 60 * 60 {
        return LoadedItemRefundDecision::Clear { new_flags };
    }

    match (paid_money, paid_extended_cost) {
        (Some(paid_money), Some(paid_extended_cost)) => LoadedItemRefundDecision::Valid {
            paid_money,
            paid_extended_cost,
        },
        _ => LoadedItemRefundDecision::Clear { new_flags },
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SellItemAmountAction {
    Invalid,
    FullStack { amount: u32 },
    PartialStack { amount: u32, remaining: u32 },
}

fn sell_item_amount_action(current_count: u32, requested_amount: i32) -> SellItemAmountAction {
    let amount = if requested_amount == 0 {
        current_count
    } else {
        let Ok(amount) = u32::try_from(requested_amount) else {
            return SellItemAmountAction::Invalid;
        };
        amount
    };

    if amount == 0 || amount > current_count {
        return SellItemAmountAction::Invalid;
    }

    if amount < current_count {
        SellItemAmountAction::PartialStack {
            amount,
            remaining: current_count - amount,
        }
    } else {
        SellItemAmountAction::FullStack { amount }
    }
}

fn item_spell_charges_db_string(charges: &[i32], effect_count: usize) -> String {
    let mut out = String::new();
    for charge in charges.iter().take(effect_count) {
        out.push_str(&charge.to_string());
        out.push(' ');
    }
    out
}

fn append_item_storage_mutable_persistence_like_cpp(
    char_db: &CharacterDatabase,
    tx: &mut SqlTransaction,
    update: &ItemStorageMutablePersistenceLikeCpp,
) {
    let mut statement = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_STORAGE_MUTABLE);
    statement.set_u32(0, update.count);
    statement.set_u32(1, update.expiration);
    statement.set_string(2, &update.charges);
    statement.set_u32(3, update.flags);
    statement.set_string(4, &update.enchantments);
    statement.set_u32(5, update.durability);
    statement.set_u32(6, update.played_time);
    statement.set_u64(7, update.db_guid);
    tx.append(statement);
}

fn fully_merged_item_cleanup_statements_like_cpp() -> [CharStatements; 7] {
    [
        CharStatements::DEL_ITEM_REFUND_INSTANCE,
        CharStatements::DEL_ITEM_BOP_TRADE,
        CharStatements::DEL_ITEM_INSTANCE_GEMS,
        CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
        CharStatements::DEL_GIFT,
        CharStatements::DEL_ITEMCONTAINER_ITEMS,
        CharStatements::DEL_ITEMCONTAINER_MONEY,
    ]
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
        db_guid,
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

fn append_item_refund_clear_statements(
    char_db: &CharacterDatabase,
    tx: &mut SqlTransaction,
    item_db_guid: u64,
    new_flags: u32,
) {
    let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
    del_refund.set_u64(0, item_db_guid);
    tx.append(del_refund);

    let mut upd_flags = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
    upd_flags.set_u32(0, new_flags);
    upd_flags.set_u64(1, item_db_guid);
    tx.append(upd_flags);
}

fn append_item_refund_insert_statements(
    char_db: &CharacterDatabase,
    tx: &mut SqlTransaction,
    item_db_guid: u64,
    player_db_guid: u64,
    paid_money: u64,
    paid_extended_cost: u16,
) {
    let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
    del_refund.set_u64(0, item_db_guid);
    tx.append(del_refund);

    let mut ins_refund = char_db.prepare(CharStatements::INS_ITEM_REFUND_INSTANCE);
    ins_refund.set_u64(0, item_db_guid);
    ins_refund.set_u64(1, player_db_guid);
    ins_refund.set_u64(2, paid_money);
    ins_refund.set_u16(3, paid_extended_cost);
    tx.append(ins_refund);
}

fn player_class_mask(player_class: u8) -> u32 {
    player_class
        .checked_sub(1)
        .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
        .unwrap_or(0)
}

fn vendor_list_should_skip_allowed_class(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    player_class: u8,
    is_game_master: bool,
) -> bool {
    if is_game_master || bonding != Some(ItemBondingType::OnAcquire as u8) {
        return false;
    }

    let Some(allowable_class) = allowable_class else {
        return false;
    };
    (i32::from(allowable_class) & player_class_mask(player_class) as i32) == 0
}

fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        // C++ resolves this from ChrRacesEntry::Alliance: 1 = Horde, 0 = Alliance.
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

fn vendor_list_should_skip_faction_flags(
    flags2: Option<u32>,
    team: Team,
    is_game_master: bool,
) -> bool {
    if is_game_master {
        return false;
    }

    let Some(flags2) = flags2 else {
        return false;
    };
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team == Team::Alliance)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team == Team::Horde)
}

fn vendor_buy_template_block_result(
    allowable_class: Option<i16>,
    bonding: Option<u8>,
    flags2: Option<u32>,
    player_class: u8,
    player_race: u8,
    is_game_master: bool,
) -> Option<VendorBuyTemplateBlock> {
    if vendor_list_should_skip_allowed_class(allowable_class, bonding, player_class, is_game_master)
    {
        return Some(VendorBuyTemplateBlock::BuyError(BuyResult::CantFindItem));
    }

    if vendor_list_should_skip_faction_flags(
        flags2,
        player_team_for_race_cpp(player_race),
        is_game_master,
    ) {
        return Some(VendorBuyTemplateBlock::Silent);
    }

    None
}

fn vendor_buy_direct_inventory_destination(
    player_guid: ObjectGuid,
    buy: &BuyItem,
) -> Option<(u8, u8)> {
    let slot = buy.slot as u8;
    if slot as usize > MAX_BAG_SIZE && slot != NULL_SLOT {
        return None;
    }

    let bag = if buy.container_guid == player_guid {
        INVENTORY_SLOT_BAG_0
    } else {
        NULL_BAG
    };

    Some((bag, slot))
}

// ── Handler implementations ─────────────────────────────────────────

fn is_represented_bag_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

#[cfg(test)]
#[path = "../character_vendor_atomicity_tests.rs"]
mod vendor_atomicity_tests;

#[cfg(test)]
#[path = "../character_tests.rs"]
mod tests;
