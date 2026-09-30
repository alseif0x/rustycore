//! Behaviour tests for [`super`].
//!
//! Extracted from `character.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

#[path = "character_tests/item_fixture_rows.rs"]
mod item_fixture_rows;

#[path = "character_tests/fixtures_1.rs"]
mod fixtures_1;
#[path = "character_tests/fixtures_2.rs"]
mod fixtures_2;
#[path = "character_tests/fixtures_3.rs"]
mod fixtures_3;
#[allow(unused_imports)]
use fixtures_1::*;
#[allow(unused_imports)]
use fixtures_2::*;
#[allow(unused_imports)]
use fixtures_3::*;

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;

use wow_world::session::{
    AuraApplication, InventoryItem, RepresentedAuraEffectLikeCpp,
};
use wow_entities::PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp;
use wow_entities::PlayerTaxiFlightNodeLikeCpp as RepresentedTaxiFlightNodeLikeCpp;
use wow_constants::unit::{
    NPCFlags1, SheathState, UNIT_FLAGS_ALLOWED_LIKE_CPP, UNIT_FLAGS2_ALLOWED_LIKE_CPP,
    UNIT_FLAGS3_ALLOWED_LIKE_CPP, UnitFlags,
};
use wow_constants::{
    ClientOpcodes, ConditionSourceType, CreatureFlagsExtra, EnchantmentSlot, InventoryResult,
    InventoryType, ItemBondingType, ItemContext, ItemExtendedCostFlags, ItemFieldFlags, ItemFlags,
    ItemFlags2, ItemModifier, ItemUpdateState, ItemVendorType, PowerType, Team, TypeId, TypeMask,
};
use wow_constants::{ItemClass, ItemSubClassWeapon, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::character_progression::{
    ChrClassesEntry, ChrClassesStore, ChrRacesEntry, ChrRacesStore,
};
use wow_data::item::ItemRecord;
use wow_data::item::stats::{ItemModType, ItemSparseTemplateEntry, ItemStatEntry, ItemStatsStore};
use wow_data::quest::{
    QUEST_ITEM_DROP_COUNT, QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT,
    QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT, QuestObjective, QuestStore,
    QuestTemplate,
};
use wow_data::{
    ItemChildEquipmentEntry, ItemChildEquipmentStore,
    PlayerConditionEntry, PlayerLevelStats, PlayerStatsStore, SpellMiscEntry, SpellMiscStore,
};
use wow_entities::{
    BANK_SLOT_BAG_END, BANK_SLOT_BAG_START, BUYBACK_SLOT_START, CHILD_EQUIPMENT_SLOT_START,
    Corpse, CorpseCustomizationChoice, CorpseType, CreatureAddonLifecycleRecordLikeCpp,
    CreatureLoot,
    EQUIPMENT_SLOT_MAINHAND, GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_GOOBER,
    INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START,
    InventoryStorageMovePlanLikeCpp, LootEntry, LootEntryFlags, MAX_BAG_SIZE, MAX_MONEY_AMOUNT,
    NULL_BAG, NULL_SLOT, PlayerEffectiveCombatStatsLikeCpp, REAGENT_BAG_SLOT_END,
    REAGENT_BAG_SLOT_START, SendNewItemDelivery, SendNewItemDisplayText, SendNewItemInstancePlan,
    SendNewItemModifier, SendNewItemPlan, SocketedGem, SwapItemPreflightResult, WorldObject,
    is_bank_pos, is_child_equipment_pos, is_equipment_pos, is_inventory_pos,
    item_can_go_into_bag, normalize_creature_chase_movement_type_like_cpp,
    normalize_creature_random_movement_type_like_cpp,
};
use wow_handler::{PacketProcessing, SessionStatus};
use wow_packet::packets::auth::*;
use wow_packet::packets::character::*;
use wow_packet::packets::chat::*;
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::item::*;
use wow_packet::packets::loot::LOOT_TYPE_CORPSE_LIKE_CPP;
use wow_packet::packets::misc::*;
use wow_packet::packets::quest::quest_giver_status;
use wow_packet::packets::update::*;
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_progression::mgr::CharacterReputationRowLikeCpp;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountLoadRowLikeCpp,
    AccountToyLoadRowLikeCpp, GossipBroadcastTextLocaleRequestLikeCpp,
    GossipCatalogPersistencePortLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipMenuOptionCatalogRowLikeCpp, GossipNpcTextCatalogRequestLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStateTemplateRowLikeCpp,
    PlayerInitialWorldStateValueRowLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp, PlayerLoginItemRepairRequestLikeCpp,
    PlayerLoginPetTalentResetOutcomeLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerOfflineMarkLikeCpp,
    PlayerOnlineMarkRequestLikeCpp,
};
use wow_world::handlers::quest::*;
use wow_world::session::*;
use wow_world::test_fixtures::*;
use wow_world::{
    canonical_player_access, conditions, entity_update_bridge, handlers, map_manager,
    player_directory, session, FinalizationDisposition, PlayerRegenerationRatesLikeCpp,
};

#[path = "character_tests/item_2.rs"]
mod item_2;
#[path = "character_tests/catalog_queries.rs"]
mod catalog_queries;
#[path = "character_tests/inventory_runtime.rs"]
mod inventory_runtime;
#[path = "character_tests/inventory_admission.rs"]
mod inventory_admission;
#[path = "character_tests/inventory_children.rs"]
mod inventory_children;
#[path = "character_tests/inventory_decisions.rs"]
mod inventory_decisions;
#[path = "character_tests/inventory_loaded.rs"]
mod inventory_loaded;
#[path = "character_tests/inventory_child_plans.rs"]
mod inventory_child_plans;
#[path = "character_tests/inventory_handlers.rs"]
mod inventory_handlers;
#[path = "character_tests/inventory_bank.rs"]
mod inventory_bank;
#[path = "character_tests/inventory_authorization.rs"]
mod inventory_authorization;
#[path = "character_tests/inventory_destruction.rs"]
mod inventory_destruction;
#[path = "character_tests/inventory_turnin.rs"]
mod inventory_turnin;
#[path = "character_tests/inventory_fixture_modes.rs"]
mod inventory_fixture_modes;
#[path = "character_tests/item_4.rs"]
mod item_4;
#[path = "character_tests/login.rs"]
mod login;
#[path = "character_tests/misc_1.rs"]
mod misc_1;
#[path = "character_tests/misc_2.rs"]
mod misc_2;
#[path = "character_tests/persistence.rs"]
mod persistence;
#[path = "character_tests/quest.rs"]
mod quest;
#[path = "character_tests/spell.rs"]
mod spell;

#[path = "character_tests/appearances.rs"]
mod appearances;

#[path = "character_tests/gossip.rs"]
mod gossip;
