//! Behaviour tests for [`super`].
//!
//! Extracted from `character.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

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

#[path = "character_tests/finalization.rs"]
mod finalization;

#[path = "character_tests/post_add_rest.rs"]
mod post_add_rest;
#[path = "character_tests/post_add_scaling.rs"]
mod post_add_scaling;

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).

use super::*;
use crate::player::inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp;
use crate::session::{
    AuraApplication, InventoryItem, RepresentedAuraEffectLikeCpp, RepresentedHomebindLikeCpp,
    RepresentedTaxiFlightNodeLikeCpp,
};
use wow_constants::{ItemClass, ItemSubClassWeapon, ServerOpcodes};
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuidGenerator};
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
    CreatureQueryCatalogLikeCpp, CreatureQueryDisplayLikeCpp, CreatureQueryTemplateLikeCpp,
    GameObjectQueryCatalogLikeCpp, GameObjectQueryTemplateLikeCpp, ItemChildEquipmentEntry,
    ItemChildEquipmentStore, PageTextCatalogLikeCpp, PageTextLikeCpp, PlayerConditionEntry,
    PlayerLevelStats, PlayerStatsStore, SpellMiscEntry, SpellMiscStore,
};
use wow_entities::{CHILD_EQUIPMENT_SLOT_START, EQUIPMENT_SLOT_MAINHAND};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
};
use wow_packet::packets::quest::quest_giver_status;
use wow_packet::{ServerPacket, WorldPacket};
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountLoadRowLikeCpp,
    AccountToyLoadRowLikeCpp, CharacterEnumerationLoadOutcomeLikeCpp,
    CharacterEnumerationPersistencePortLikeCpp, CharacterEnumerationRequestLikeCpp,
    CharacterEnumerationRowLikeCpp, GossipBroadcastTextLocaleRequestLikeCpp,
    GossipCatalogPersistencePortLikeCpp, GossipCatalogReadOutcomeLikeCpp,
    GossipCreatureMenuRequestLikeCpp, GossipMenuCatalogRequestLikeCpp,
    GossipMenuOptionCatalogRowLikeCpp, GossipNpcTextCatalogRequestLikeCpp,
    MapCorpseAuxiliaryLoadOutcomeLikeCpp,
    MapCorpseLoadOutcomeLikeCpp as PersistedMapCorpseLoadOutcomeLikeCpp,
    MapCorpseLoadRequestLikeCpp, MapCorpseLoadRowLikeCpp, MapCorpsePersistencePortLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStateTemplateRowLikeCpp,
    PlayerInitialWorldStateValueRowLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerLoginAuxiliaryLoadOutcomeLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp, PlayerLoginItemRepairRequestLikeCpp,
    PlayerLoginPetTalentResetOutcomeLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerNameQueryOutcomeLikeCpp,
    PlayerNameQueryPersistencePortLikeCpp, PlayerNameQueryRequestLikeCpp,
    PlayerNameQueryRowLikeCpp, PlayerOfflineMarkLikeCpp, PlayerOnlineMarkRequestLikeCpp,
};

#[path = "character_tests/creature.rs"]
mod creature;
#[path = "character_tests/gameobject.rs"]
mod gameobject;
#[path = "character_tests/group.rs"]
mod group;
#[path = "character_tests/instance.rs"]
mod instance;
#[path = "character_tests/item_1.rs"]
mod item_1;
#[path = "character_tests/item_2.rs"]
mod item_2;
#[path = "character_tests/item_3.rs"]
mod item_3;
#[path = "character_tests/item_4.rs"]
mod item_4;
#[path = "character_tests/login.rs"]
mod login;
#[path = "character_tests/loot.rs"]
mod loot;
#[path = "character_tests/misc_1.rs"]
mod misc_1;
#[path = "character_tests/misc_2.rs"]
mod misc_2;
#[path = "character_tests/misc_3.rs"]
mod misc_3;
#[path = "character_tests/misc_4.rs"]
mod misc_4;
#[path = "character_tests/movement.rs"]
mod movement;
#[path = "character_tests/persistence.rs"]
mod persistence;
#[path = "character_tests/pet.rs"]
mod pet;
#[path = "character_tests/quest.rs"]
mod quest;
#[path = "character_tests/skill.rs"]
mod skill;
#[path = "character_tests/spell.rs"]
mod spell;
#[path = "character_tests/visibility.rs"]
mod visibility;
