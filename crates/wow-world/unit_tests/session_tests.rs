#![cfg(test)]

#[path = "session/tests/active_cast_owner.rs"]
mod active_cast_owner;
#[path = "session/tests/admission.rs"]
mod admission;
#[path = "session/tests/cinematic_catalog.rs"]
mod cinematic_catalog;
#[path = "session/tests/connection.rs"]
mod connection;
#[path = "session/tests/difficulty_owner.rs"]
mod difficulty_owner;
#[path = "session/tests/dispatch.rs"]
mod dispatch;
#[path = "session/tests/driver.rs"]
mod driver;
#[path = "session/effect_learning_tests.rs"]
mod effect_learning_tests;
#[path = "session/tests/fixtures/mod.rs"]
mod fixtures;
#[path = "session/tests/glyph_catalog.rs"]
mod glyph_catalog;
#[path = "session/tests/instance_occupancy.rs"]
mod instance_occupancy;
#[path = "session/tests/lifecycle.rs"]
mod lifecycle;
#[path = "session/tests/lifecycle_persistence.rs"]
mod lifecycle_persistence;
#[path = "session/tests/login_auxiliary_persistence.rs"]
mod login_auxiliary_persistence;
#[path = "session/tests/mailbox_pump.rs"]
mod mailbox_pump;
#[path = "session/tests/map_entry.rs"]
mod map_entry;
#[path = "session/tests/map_phase_pass.rs"]
mod map_phase_pass;
#[path = "session/tests/pending_cast_owner.rs"]
mod pending_cast_owner;
#[path = "session/tests/player_cast_lifecycle.rs"]
mod player_cast_lifecycle;
#[path = "session/tests/player_detach.rs"]
mod player_detach;
#[path = "session/tests/player_spell_hit_source.rs"]
mod player_spell_hit_source;
#[path = "session/tests/rest_owner.rs"]
mod rest_owner;
#[path = "session/tests/routing.rs"]
mod routing;
#[path = "session/tests/save_plan_order.rs"]
mod save_plan_order;
#[path = "session/tests/save_snapshot_owner.rs"]
mod save_snapshot_owner;
#[path = "session/tests/scenarios_chat.rs"]
mod scenarios_chat;
#[path = "session/tests/scenarios_combat_1.rs"]
mod scenarios_combat_1;
#[path = "session/tests/scenarios_combat_2.rs"]
mod scenarios_combat_2;
#[path = "session/tests/scenarios_combat_3.rs"]
mod scenarios_combat_3;
#[path = "session/tests/scenarios_combat_4.rs"]
mod scenarios_combat_4;
#[path = "session/tests/scenarios_combat_5.rs"]
mod scenarios_combat_5;
#[path = "session/tests/scenarios_instances_1.rs"]
mod scenarios_instances_1;
#[path = "session/tests/scenarios_instances_2.rs"]
mod scenarios_instances_2;
#[path = "session/tests/scenarios_instances_3.rs"]
mod scenarios_instances_3;
#[path = "session/tests/scenarios_instances_4.rs"]
mod scenarios_instances_4;
#[path = "session/tests/scenarios_login_1.rs"]
mod scenarios_login_1;
#[path = "session/tests/scenarios_login_2.rs"]
mod scenarios_login_2;
#[path = "session/tests/scenarios_loot.rs"]
mod scenarios_loot;
#[path = "session/tests/scenarios_misc_1.rs"]
mod scenarios_misc_1;
#[path = "session/tests/scenarios_misc_10.rs"]
mod scenarios_misc_10;
#[path = "session/tests/scenarios_misc_11.rs"]
mod scenarios_misc_11;
#[path = "session/tests/scenarios_misc_12.rs"]
mod scenarios_misc_12;
#[path = "session/tests/scenarios_misc_2.rs"]
mod scenarios_misc_2;
#[path = "session/tests/scenarios_misc_3.rs"]
mod scenarios_misc_3;
#[path = "session/tests/scenarios_misc_4.rs"]
mod scenarios_misc_4;
#[path = "session/tests/scenarios_misc_5.rs"]
mod scenarios_misc_5;
#[path = "session/tests/scenarios_misc_6.rs"]
mod scenarios_misc_6;
#[path = "session/tests/scenarios_misc_7.rs"]
mod scenarios_misc_7;
#[path = "session/tests/scenarios_misc_8.rs"]
mod scenarios_misc_8;
#[path = "session/tests/scenarios_misc_9.rs"]
mod scenarios_misc_9;
#[path = "session/tests/scenarios_money.rs"]
mod scenarios_money;
#[path = "session/tests/scenarios_movement_1.rs"]
mod scenarios_movement_1;
#[path = "session/tests/scenarios_movement_2.rs"]
mod scenarios_movement_2;
#[path = "session/tests/scenarios_movement_3.rs"]
mod scenarios_movement_3;
#[path = "session/tests/scenarios_movement_4.rs"]
mod scenarios_movement_4;
#[path = "session/tests/scenarios_movement_5.rs"]
mod scenarios_movement_5;
#[path = "session/tests/scenarios_movement_6.rs"]
mod scenarios_movement_6;
#[path = "session/tests/scenarios_movement_7.rs"]
mod scenarios_movement_7;
#[path = "session/tests/scenarios_movement_8.rs"]
mod scenarios_movement_8;
#[path = "session/tests/scenarios_persistence_1.rs"]
mod scenarios_persistence_1;
#[path = "session/tests/scenarios_persistence_2.rs"]
mod scenarios_persistence_2;
#[path = "session/tests/scenarios_persistence_3.rs"]
mod scenarios_persistence_3;
#[path = "session/tests/scenarios_persistence_4.rs"]
mod scenarios_persistence_4;
#[path = "session/tests/scenarios_pets_1.rs"]
mod scenarios_pets_1;
#[path = "session/tests/scenarios_pets_2.rs"]
mod scenarios_pets_2;
#[path = "session/tests/scenarios_pets_3.rs"]
mod scenarios_pets_3;
#[path = "session/tests/scenarios_player_items_1.rs"]
mod scenarios_player_items_1;
#[path = "session/tests/scenarios_player_items_10.rs"]
mod scenarios_player_items_10;
#[path = "session/tests/scenarios_player_items_11.rs"]
mod scenarios_player_items_11;
#[path = "session/tests/scenarios_player_items_12.rs"]
mod scenarios_player_items_12;
#[path = "session/tests/scenarios_player_items_2.rs"]
mod scenarios_player_items_2;
#[path = "session/tests/scenarios_player_items_3.rs"]
mod scenarios_player_items_3;
#[path = "session/tests/scenarios_player_items_4.rs"]
mod scenarios_player_items_4;
#[path = "session/tests/scenarios_player_items_5.rs"]
mod scenarios_player_items_5;
#[path = "session/tests/scenarios_player_items_6.rs"]
mod scenarios_player_items_6;
#[path = "session/tests/scenarios_player_items_7.rs"]
mod scenarios_player_items_7;
#[path = "session/tests/scenarios_player_items_8.rs"]
mod scenarios_player_items_8;
#[path = "session/tests/scenarios_player_items_9.rs"]
mod scenarios_player_items_9;
#[path = "session/tests/scenarios_progression.rs"]
mod scenarios_progression;
#[path = "session/tests/scenarios_quest_1.rs"]
mod scenarios_quest_1;
#[path = "session/tests/scenarios_quest_2.rs"]
mod scenarios_quest_2;
#[path = "session/tests/scenarios_social_1.rs"]
mod scenarios_social_1;
#[path = "session/tests/scenarios_social_2.rs"]
mod scenarios_social_2;
#[path = "session/tests/scenarios_spell_state_1.rs"]
mod scenarios_spell_state_1;
#[path = "session/tests/scenarios_spell_state_10.rs"]
mod scenarios_spell_state_10;
#[path = "session/tests/scenarios_spell_state_11.rs"]
mod scenarios_spell_state_11;
#[path = "session/tests/scenarios_spell_state_12.rs"]
mod scenarios_spell_state_12;
#[path = "session/tests/scenarios_spell_state_13.rs"]
mod scenarios_spell_state_13;
#[path = "session/tests/scenarios_spell_state_14.rs"]
mod scenarios_spell_state_14;
#[path = "session/tests/scenarios_spell_state_15.rs"]
mod scenarios_spell_state_15;
#[path = "session/tests/scenarios_spell_state_16.rs"]
mod scenarios_spell_state_16;
#[path = "session/tests/scenarios_spell_state_17.rs"]
mod scenarios_spell_state_17;
#[path = "session/tests/scenarios_spell_state_18.rs"]
mod scenarios_spell_state_18;
#[path = "session/tests/scenarios_spell_state_19.rs"]
mod scenarios_spell_state_19;
#[path = "session/tests/scenarios_spell_state_2.rs"]
mod scenarios_spell_state_2;
#[path = "session/tests/scenarios_spell_state_20.rs"]
mod scenarios_spell_state_20;
#[path = "session/tests/scenarios_spell_state_21.rs"]
mod scenarios_spell_state_21;
#[path = "session/tests/scenarios_spell_state_22.rs"]
mod scenarios_spell_state_22;
#[path = "session/tests/scenarios_spell_state_23.rs"]
mod scenarios_spell_state_23;
#[path = "session/tests/scenarios_spell_state_24.rs"]
mod scenarios_spell_state_24;
#[path = "session/tests/scenarios_spell_state_25.rs"]
mod scenarios_spell_state_25;
#[path = "session/tests/scenarios_spell_state_26.rs"]
mod scenarios_spell_state_26;
#[path = "session/tests/scenarios_spell_state_3.rs"]
mod scenarios_spell_state_3;
#[path = "session/tests/scenarios_spell_state_4.rs"]
mod scenarios_spell_state_4;
#[path = "session/tests/scenarios_spell_state_5.rs"]
mod scenarios_spell_state_5;
#[path = "session/tests/scenarios_spell_state_6.rs"]
mod scenarios_spell_state_6;
#[path = "session/tests/scenarios_spell_state_7.rs"]
mod scenarios_spell_state_7;
#[path = "session/tests/scenarios_spell_state_8.rs"]
mod scenarios_spell_state_8;
#[path = "session/tests/scenarios_spell_state_9.rs"]
mod scenarios_spell_state_9;
#[path = "session/tests/scenarios_visibility_1.rs"]
mod scenarios_visibility_1;
#[path = "session/tests/scenarios_visibility_2.rs"]
mod scenarios_visibility_2;
#[path = "session/tests/scenarios_visibility_3.rs"]
mod scenarios_visibility_3;
#[path = "session/tests/scenarios_world_entities_1.rs"]
mod scenarios_world_entities_1;
#[path = "session/tests/scenarios_world_entities_10.rs"]
mod scenarios_world_entities_10;
#[path = "session/tests/scenarios_world_entities_11.rs"]
mod scenarios_world_entities_11;
#[path = "session/tests/scenarios_world_entities_12.rs"]
mod scenarios_world_entities_12;
#[path = "session/tests/scenarios_world_entities_13.rs"]
mod scenarios_world_entities_13;
#[path = "session/tests/scenarios_world_entities_14.rs"]
mod scenarios_world_entities_14;
#[path = "session/tests/scenarios_world_entities_15.rs"]
mod scenarios_world_entities_15;
#[path = "session/tests/scenarios_world_entities_16.rs"]
mod scenarios_world_entities_16;
#[path = "session/tests/scenarios_world_entities_17.rs"]
mod scenarios_world_entities_17;
#[path = "session/tests/scenarios_world_entities_18.rs"]
mod scenarios_world_entities_18;
#[path = "session/tests/scenarios_world_entities_19.rs"]
mod scenarios_world_entities_19;
#[path = "session/tests/scenarios_world_entities_2.rs"]
mod scenarios_world_entities_2;
#[path = "session/tests/scenarios_world_entities_20.rs"]
mod scenarios_world_entities_20;
#[path = "session/tests/scenarios_world_entities_21.rs"]
mod scenarios_world_entities_21;
#[path = "session/tests/scenarios_world_entities_22.rs"]
mod scenarios_world_entities_22;
#[path = "session/tests/scenarios_world_entities_23.rs"]
mod scenarios_world_entities_23;
#[path = "session/tests/scenarios_world_entities_24.rs"]
mod scenarios_world_entities_24;
#[path = "session/tests/scenarios_world_entities_25.rs"]
mod scenarios_world_entities_25;
#[path = "session/tests/scenarios_world_entities_26.rs"]
mod scenarios_world_entities_26;
#[path = "session/tests/scenarios_world_entities_27.rs"]
mod scenarios_world_entities_27;
#[path = "session/tests/scenarios_world_entities_28.rs"]
mod scenarios_world_entities_28;
#[path = "session/tests/scenarios_world_entities_29.rs"]
mod scenarios_world_entities_29;
#[path = "session/tests/scenarios_world_entities_3.rs"]
mod scenarios_world_entities_3;
#[path = "session/tests/scenarios_world_entities_30.rs"]
mod scenarios_world_entities_30;
#[path = "session/tests/scenarios_world_entities_31.rs"]
mod scenarios_world_entities_31;
#[path = "session/tests/scenarios_world_entities_32.rs"]
mod scenarios_world_entities_32;
#[path = "session/tests/scenarios_world_entities_33.rs"]
mod scenarios_world_entities_33;
#[path = "session/tests/scenarios_world_entities_34.rs"]
mod scenarios_world_entities_34;
#[path = "session/tests/scenarios_world_entities_4.rs"]
mod scenarios_world_entities_4;
#[path = "session/tests/scenarios_world_entities_5.rs"]
mod scenarios_world_entities_5;
#[path = "session/tests/scenarios_world_entities_6.rs"]
mod scenarios_world_entities_6;
#[path = "session/tests/scenarios_world_entities_7.rs"]
mod scenarios_world_entities_7;
#[path = "session/tests/scenarios_world_entities_8.rs"]
mod scenarios_world_entities_8;
#[path = "session/tests/scenarios_world_entities_9.rs"]
mod scenarios_world_entities_9;
#[path = "session/tests/session_account_state.rs"]
mod session_account_state;
#[path = "session/tests/skill_owner.rs"]
mod skill_owner;
#[path = "session/tests/spell_history_owner.rs"]
mod spell_history_owner;
#[path = "session/tests/spellbook_owner.rs"]
mod spellbook_owner;
#[path = "session/tests/talent_catalog.rs"]
mod talent_catalog;
#[path = "session/tests/talent_owner.rs"]
mod talent_owner;
#[path = "session/tests/taxi_owner.rs"]
mod taxi_owner;

use routing::assert_destroyed_party_update_like_cpp;

use super::*;
use crate::canonical_player_access::{
    configure_canonical_player_party_flags_for_test as set_party_flags,
    configure_canonical_player_vitals_for_test as set_vitals,
    install_canonical_player_owner_for_test, with_canonical_player_at_like_cpp,
    with_canonical_player_at_mut_like_cpp,
};
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupRemovalLikeCppCommand,
    ApplyGroupSubgroupLikeCppCommand, ApplyLootMoneyLikeCppCommand,
    CreatureAttackStartLikeCppCommand, GameEventQuestCompleteClientOutcomeLikeCpp,
    GameEventQuestCompleteResponseLikeCpp, KickLikeCppCommand,
    RefreshVisibleWorldCreaturesLikeCppCommand, ResetSeasonalQuestStatusCommand,
    SendIfVisibleLikeCppCommand, SendPartyUpdateLikeCppCommand, SendRealmPacketLikeCppCommand,
    SendVisibleObjectValuesUpdateCommand, SessionCommand, WorldSessionShutdownFlushLikeCppCommand,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use wow_constants::ItemModType;
use wow_constants::{
    BagFamilyMask, ConditionSourceType, ConditionType, EnchantmentSlot, InventoryResult,
    InventoryType, ItemBondingType, ItemClass, ItemContext, ItemFieldFlags, ItemFlags, ItemFlags2,
    ItemUpdateState, PhaseShiftFlags, ServerOpcodes, SpellCastResult, SpellItemEnchantmentFlags,
    UnitDynFlags, UnitFlags,
};
use wow_core::{Position, guid::HighGuid};
use wow_data::{
    ChrSpecializationEntry, ChrSpecializationStore, Condition, DifficultyEntry, DifficultyStore,
    DurabilityCostsEntry, DurabilityCostsStore, DurabilityQualityEntry, DurabilityQualityStore,
    GemPropertiesEntry, GemPropertiesStore, HeirloomEntry, HeirloomStore, ImportPriceArmorEntry,
    ImportPriceArmorStore, ImportPriceQualityEntry, ImportPriceQualityStore,
    ImportPriceShieldEntry, ImportPriceShieldStore, ImportPriceStores, ImportPriceWeaponEntry,
    ImportPriceWeaponStore, ItemAppearanceEntry, ItemAppearanceStore, ItemBonusDb2Entry,
    ItemBonusDb2Store, ItemClassEntry, ItemClassStore, ItemCurrencyCostEntry,
    ItemCurrencyCostStore, ItemDisenchantLootEntry, ItemDisenchantLootStore, ItemEffectEntry,
    ItemEffectStore, ItemLimitCategoryConditionEntry, ItemLimitCategoryConditionStore,
    ItemLimitCategoryEntry, ItemLimitCategoryStore, ItemModifiedAppearanceEntry,
    ItemModifiedAppearanceStore, ItemPriceBaseEntry, ItemPriceBaseStore,
    ItemRandomPropertyTemplateEntry, ItemRandomSuffixEntry, ItemRandomSuffixStore, ItemRecord,
    ItemSearchNameEntry, ItemSearchNameStore, ItemSetEntry, ItemSetSpellEntry, ItemSetSpellStore,
    ItemSetStore, ItemSocketTemplateEntry, ItemSparseTemplateEntry, ItemSpecOverrideEntry,
    ItemSpecOverrideStore, ItemStatsStore, ItemStore, ItemWeaponTemplateEntry, LockEntry,
    LockStore, MapDifficultyEntry, MapDifficultyStore, PlayerConditionEntry, PlayerConditionStore,
    ShieldBlockRegularEntryLikeCpp, ShieldBlockRegularGameTableLikeCpp, SpellInfo,
    SpellItemEnchantmentConditionEntry, SpellItemEnchantmentConditionStore,
    SpellItemEnchantmentEntry, SpellItemEnchantmentStore, SpellStore, ToyEntry, ToyStore,
    TransmogSetEntry, TransmogSetItemEntry, TransmogSetItemStore,
    progression_rewards::{
        ContentTuningEntry, ContentTuningStore, CurveEntry, CurvePointEntry, CurvePointStore,
        CurveStore, FactionEntry, FactionStore, QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP,
        QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP, QuestPackageItemEntry, QuestPackageItemStore,
        ScalingStatDistributionEntry, ScalingStatDistributionStore, ScalingStatValuesEntry,
        ScalingStatValuesStore,
    },
    reputation::ReputationFlagsLikeCpp,
};
use wow_data::{ItemStatEntry, PvpItemEntry};
use wow_entities::{
    ApplyEnchantmentDurationAction, ApplyEnchantmentResult, ApplyEnchantmentSkipReason,
    BANK_SLOT_BAG_START, BANK_SLOT_ITEM_START, CharmType, EQUIPMENT_SLOT_CHEST,
    EQUIPMENT_SLOT_HANDS, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, ItemBonusKey,
    PlayerEnchantDuration, REAGENT_BAG_SLOT_START, SendNewItemInstancePlan, SendNewItemModifier,
    SocketedGem, TYPEID_UNIT, UNIT_DATA_BITS, UnitDataUpdate, UnitDataValues, UnitValuesUpdate,
    UpdateMask,
};
use wow_packet::ServerPacket;
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags,
};
use wow_social::group::{
    GroupInfo, GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp, GroupRegistry,
    PendingInviteLikeCpp, PendingInvites,
};

use fixtures::*;
