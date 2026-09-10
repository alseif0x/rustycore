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

const BATTLEGROUND_AB_LIKE_CPP: u32 = 3;
const XP_HOOK_DOUBLE_PLAYER_COUNTER: i64 = 0xE1D0;
const XP_HOOK_ZERO_PLAYER_COUNTER: i64 = 0xE1D1;
const XP_HOOK_MAX_PLAYER_COUNTER: i64 = 0xE1D2;
const XP_HOOK_GUARD_PLAYER_COUNTER: i64 = 0xE1D3;

static XP_HOOK_DOUBLE_CALLS: AtomicUsize = AtomicUsize::new(0);
static XP_HOOK_ZERO_CALLS: AtomicUsize = AtomicUsize::new(0);
static XP_HOOK_MAX_CALLS: AtomicUsize = AtomicUsize::new(0);
static XP_HOOK_GUARD_CALLS: AtomicUsize = AtomicUsize::new(0);

fn represented_test_give_player_xp_hook_like_cpp(
    context: wow_script::player::GivePlayerXpContextLikeCpp,
    amount: &mut u32,
) {
    match context.player_guid.counter() {
        XP_HOOK_DOUBLE_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D0);
            XP_HOOK_DOUBLE_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            *amount = amount.saturating_mul(2);
        }
        XP_HOOK_ZERO_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D1);
            XP_HOOK_ZERO_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            *amount = 0;
        }
        XP_HOOK_MAX_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D2);
            XP_HOOK_MAX_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
        }
        XP_HOOK_GUARD_PLAYER_COUNTER => {
            XP_HOOK_GUARD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
        }
        _ => {}
    }
}

/// Exact propagated Map-tick time needed for a scheduled assistance call.
/// There is no scheduler margin after #371 because wall time cannot advance
/// the creature between scheduling and the explicit logical-clock step.
const ASSISTANCE_DELAY_ELAPSED_LIKE_CPP: Duration =
    Duration::from_millis(wow_movement::CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP as u64);

fn make_session() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, send_rx) = flume::unbounded();

    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9, // account_expansion (raw from DB)
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_active_player_local_flags_like_cpp(
        PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    );

    (session, pkt_tx, send_rx)
}

fn make_session_with_give_player_xp_hook() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, pkt_tx, send_rx) = make_session();
    session.set_give_player_xp_script_dispatcher_like_cpp(Arc::new(
        represented_test_give_player_xp_hook_like_cpp,
    ));
    (session, pkt_tx, send_rx)
}

fn difficulty_entry(id: u32, instance_type: u8, flags: DifficultyFlags) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type,
        flags: flags.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }
}

fn test_spell_proc_entry_like_cpp(chance: f32) -> wow_data::SpellProcEntryLikeCpp {
    wow_data::SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0; 4],
        proc_flags: [1, 0],
        spell_type_mask: 0,
        spell_phase_mask: 0,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance,
        cooldown_ms: 0,
        charges: 0,
    }
}

fn test_spell_threat_entry_like_cpp(flat_mod: i32) -> wow_data::SpellThreatEntryLikeCpp {
    wow_data::SpellThreatEntryLikeCpp {
        flat_mod,
        pct_mod: 1.0,
        ap_pct_mod: 0.0,
    }
}

fn test_spell_required_store_like_cpp() -> wow_data::SpellRequiredStoreLikeCpp {
    let outcome = wow_data::SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 100,
                req_spell: 10,
            },
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 100,
                req_spell: 11,
            },
            wow_data::SpellRequiredRowLikeCpp {
                spell_id: 101,
                req_spell: 10,
            },
        ],
        |_| true,
        |_, _| false,
    );
    outcome.store
}

fn test_spell_group_store_like_cpp() -> wow_data::SpellGroupStoreLikeCpp {
    let outcome = wow_data::SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            },
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: -1002,
            },
            wow_data::SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 20,
            },
        ],
        |_| true,
        |_| 1,
    );
    outcome.store
}

fn test_spell_group_stack_rule_store_like_cpp(
    spell_groups: &wow_data::SpellGroupStoreLikeCpp,
) -> wow_data::SpellGroupStackRuleStoreLikeCpp {
    let outcome = wow_data::SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: wow_data::SpellGroupStackRuleLikeCpp::ExclusiveHighest as u8,
            },
            wow_data::SpellGroupStackRuleRowLikeCpp {
                group_id: 1002,
                stack_rule: wow_data::SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
            },
        ],
        spell_groups,
        |spell_id| {
            let mut spell = wow_data::SpellInfo {
                spell_id: spell_id as i32,
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
                effects: Vec::new(),
            };
            spell.effects.push(wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: 31,
                ..Default::default()
            });
            Some(spell)
        },
        |_| None,
    );
    outcome.store
}

fn test_spell_linked_store_like_cpp() -> wow_data::SpellLinkedStoreLikeCpp {
    let outcome = wow_data::SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 20,
                link_type: 0,
            },
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: -30,
                link_type: 0,
            },
            wow_data::SpellLinkedRowLikeCpp {
                spell_trigger: -40,
                spell_effect: 50,
                link_type: 1,
            },
        ],
        |_| {
            Some(wow_data::SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );
    outcome.store
}

fn test_spell_totem_model_store_like_cpp() -> wow_data::SpellTotemModelStoreLikeCpp {
    let outcome = wow_data::SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 2,
                display_id: 1000,
            },
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 2,
                display_id: 2000,
            },
            wow_data::SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 3000,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );
    outcome.store
}

fn test_spell_pet_aura_store_like_cpp() -> wow_data::SpellPetAuraStoreLikeCpp {
    let outcome = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            wow_data::SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 0,
                aura_id: 900,
            },
            wow_data::SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 501,
                aura_id: 901,
            },
        ],
        |_, _| {
            wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                wow_data::SpellPetAuraSourceEffectLikeCpp {
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                    apply_aura_name: 0,
                    target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                    calc_value: 35,
                },
            )
        },
        |_| true,
    );
    outcome.store
}

fn test_spell_area_store_like_cpp() -> wow_data::SpellAreaStoreLikeCpp {
    let outcome = wow_data::SpellAreaStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellAreaRowLikeCpp {
                spell_id: 100,
                area_id: 10,
                quest_start: 20,
                quest_start_status: 1,
                quest_end_status: 2,
                quest_end: 30,
                aura_spell: -40,
                race_mask: 1,
                gender: wow_data::GENDER_NONE_LIKE_CPP,
                flags: wow_data::SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP,
            },
            wow_data::SpellAreaRowLikeCpp {
                spell_id: 101,
                area_id: 11,
                quest_start: 30,
                quest_start_status: 3,
                quest_end_status: 4,
                quest_end: 30,
                aura_spell: 0,
                race_mask: 0,
                gender: wow_data::GENDER_MALE_LIKE_CPP,
                flags: 0,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );
    outcome.store
}

fn test_spell_custom_attribute_store_like_cpp() -> wow_data::SpellCustomAttributeStoreLikeCpp {
    let outcome = wow_data::SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [
            wow_data::SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: wow_data::SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            },
            wow_data::SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: wow_data::SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
            },
        ],
        |spell_id| {
            (spell_id == 100)
                .then(|| {
                    vec![
                        wow_data::SpellCustomAttributeSourceSpellInfoLikeCpp {
                            spell_id: 100,
                            difficulty: 0,
                            effects: vec![wow_data::SpellEffectInfo {
                                effect_index: 0,
                                effect:
                                    wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                                ..Default::default()
                            }],
                        },
                        wow_data::SpellCustomAttributeSourceSpellInfoLikeCpp {
                            spell_id: 100,
                            difficulty: 2,
                            effects: vec![wow_data::SpellEffectInfo {
                                effect_index: 0,
                                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                                ..Default::default()
                            }],
                        },
                    ]
                })
                .unwrap_or_default()
        },
    );
    outcome.store
}

fn test_serverside_spell_store_like_cpp() -> wow_data::ServersideSpellStoreLikeCpp {
    let outcome = wow_data::ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [wow_data::ServersideSpellRowLikeCpp {
            spell_id: 100,
            difficulty_id: 0,
            category_id: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: [0; 14],
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            facing_caster_flags: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            caster_aura_type: 0,
            target_aura_type: 0,
            exclude_caster_aura_type: 0,
            exclude_target_aura_type: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: [0; 2],
            channel_interrupt_flags: [0; 2],
            proc_flags: [0; 2],
            proc_chance: 0,
            proc_charges: 0,
            proc_cooldown: 0,
            proc_base_ppm: 0.0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            range_index: 0,
            speed: 0.0,
            launch_delay: 0.0,
            stack_amount: 0,
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            content_tuning_id: 0,
            spell_name: "server spell".to_string(),
            cone_angle: 0.0,
            cone_width: 0.0,
            max_target_level: 0,
            max_affected_targets: 0,
            spell_family_name: 0,
            spell_family_flags: [0; 4],
            dmg_class: 0,
            prevention_type: 0,
            area_group_id: 0,
            school_mask: 0,
            charge_category_id: 0,
        }],
        &wow_data::ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    outcome.store
}

fn test_spell_learn_skill_store_like_cpp() -> wow_data::SpellLearnSkillStoreLikeCpp {
    let outcome = wow_data::SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([
        wow_data::SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id: 10,
            difficulty_none: true,
            effects: vec![wow_data::SpellLearnSkillEffectLikeCpp {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 755,
                calc_value: 4,
            }],
        },
        wow_data::SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id: 20,
            difficulty_none: true,
            effects: vec![wow_data::SpellLearnSkillEffectLikeCpp {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                misc_value: 0,
                calc_value: 0,
            }],
        },
    ]);
    outcome.store
}

fn test_spell_learn_skill_rank_store_like_cpp(
    skill_id: u16,
) -> wow_data::SpellLearnSkillStoreLikeCpp {
    wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([
            (
                10,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: skill_id,
                    step: 2,
                    value: 0,
                    maxvalue: 0,
                },
            ),
            (
                20,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: skill_id,
                    step: 3,
                    value: 0,
                    maxvalue: 0,
                },
            ),
        ]),
        ..Default::default()
    }
}

fn test_skill_line_entry_like_cpp(skill_id: u16, category_id: i8) -> wow_data::SkillLineEntry {
    wow_data::SkillLineEntry {
        id: u32::from(skill_id),
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id: 0,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

fn test_skill_race_class_info_like_cpp(
    skill_id: u16,
    flags: u16,
    skill_tier_id: i16,
) -> wow_data::SkillRaceClassInfoRecord {
    wow_data::SkillRaceClassInfoRecord {
        id: u32::from(skill_id),
        race_mask: 0,
        skill_id,
        class_mask: 0,
        flags,
        availability: 0,
        min_level: 0,
        skill_tier_id,
    }
}

fn prepare_remove_spell_skill_range_fixture_like_cpp(
    session: &mut WorldSession,
    skill_id: u16,
    category_id: i8,
    race_class_flags: u16,
    skill_tier_id: i16,
    skill_tiers_store: wow_data::SkillTiersStoreLikeCpp,
    skill_value: u16,
    skill_max: u16,
) {
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 12, 0);
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_rank_store_like_cpp(
        skill_id,
    )));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([
        test_skill_line_entry_like_cpp(skill_id, category_id),
    ])));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            std::iter::empty::<wow_data::SkillLineAbilityRecord>(),
            [test_skill_race_class_info_like_cpp(
                skill_id,
                race_class_flags,
                skill_tier_id,
            )],
        ),
    ));
    session.set_skill_tiers_store(Arc::new(skill_tiers_store));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        skill_id,
        RepresentedPlayerSkillLikeCpp {
            skill_id,
            step: 3,
            value: skill_value,
            max: skill_max,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);
}

fn test_spell_learn_spell_store_like_cpp() -> wow_data::SpellLearnSpellStoreLikeCpp {
    let outcome = wow_data::SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [wow_data::SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [wow_data::SpellLearnSourceSpellInfoLikeCpp {
            spell_id: 30,
            difficulty_none: true,
            is_talent: false,
            is_passive: true,
            has_skill_step_effect: false,
            learn_spell_effects: vec![wow_data::SpellLearnSpellEffectLikeCpp {
                trigger_spell: 40,
                target_unit_pet: false,
            }],
        }],
        std::iter::empty::<wow_data::SpellLearnSpellEntry>(),
        |spell_id| {
            Some(wow_data::SpellLearnSourceSpellInfoLikeCpp {
                spell_id,
                difficulty_none: true,
                is_talent: false,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: Vec::new(),
            })
        },
        |_| true,
    );
    outcome.store
}

fn test_pet_levelup_spell_store_like_cpp() -> wow_data::PetLevelupSpellStoreLikeCpp {
    let skill_store = wow_data::SkillStore::from_skill_line_abilities_like_cpp([
        wow_data::SkillLineAbilityRecord {
            id: 1,
            race_mask: 0,
            skill_line: 10,
            spell: 700,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
        wow_data::SkillLineAbilityRecord {
            id: 2,
            race_mask: 0,
            skill_line: 10,
            spell: 701,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
    ]);

    wow_data::PetLevelupSpellStoreLikeCpp::load_like_cpp(
        [wow_data::CreatureFamilyEntry {
            id: 44,
            name: String::new(),
            min_scale: 0.0,
            min_scale_level: 0,
            max_scale: 0.0,
            max_scale_level: 0,
            pet_food_mask: 0,
            pet_talent_type: 0,
            category_enum_id: 0,
            icon_file_id: 0,
            skill_line: [10, 0],
        }],
        &skill_store,
        |spell_id| match spell_id {
            700 => Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: 700,
                spell_level: 20,
            }),
            701 => Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: 701,
                spell_level: 10,
            }),
            _ => None,
        },
    )
}

fn test_pet_default_spell_store_like_cpp() -> wow_data::PetDefaultSpellStoreLikeCpp {
    wow_data::PetDefaultSpellStoreLikeCpp::load_like_cpp(
        [wow_data::PetDefaultSpellInfoLikeCpp {
            difficulty_none: true,
            effects: vec![wow_data::PetDefaultSpellEffectLikeCpp {
                effect: 56,
                misc_value: 500,
            }],
        }],
        [wow_data::PetDefaultSpellCreatureTemplateLikeCpp {
            entry: 500,
            family: 0,
            spells: [10, 0, 11, 0],
        }],
        &wow_data::PetLevelupSpellStoreLikeCpp::default(),
    )
}

fn test_pet_family_spell_store_like_cpp() -> wow_data::PetFamilySpellStoreLikeCpp {
    let skill_store = wow_data::SkillStore::from_skill_line_abilities_like_cpp([
        wow_data::SkillLineAbilityRecord {
            id: 1,
            race_mask: 0,
            skill_line: 10,
            spell: 800,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
        wow_data::SkillLineAbilityRecord {
            id: 2,
            race_mask: 0,
            skill_line: 10,
            spell: 801,
            min_skill_line_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 2,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        },
    ]);

    wow_data::PetFamilySpellStoreLikeCpp::load_like_cpp(
        &skill_store,
        [wow_data::CreatureFamilyEntry {
            id: 44,
            name: String::new(),
            min_scale: 0.0,
            min_scale_level: 0,
            max_scale: 0.0,
            max_scale_level: 0,
            pet_food_mask: 0,
            pet_talent_type: 0,
            category_enum_id: 0,
            icon_file_id: 0,
            skill_line: [10, 0],
        }],
        [],
        |spell_id| match spell_id {
            800 => Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: 800,
                is_passive: true,
            }),
            801 => Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: 801,
                is_passive: false,
            }),
            _ => None,
        },
    )
}

fn install_remove_spell_offhand_templates_like_cpp(
    session: &mut WorldSession,
    items: &[(u32, InventoryType, u32, ItemClass, u8)],
) {
    session.set_item_store(Arc::new(ItemStore::from_records(items.iter().map(
        |&(item_id, inventory_type, _, class_id, subclass_id)| ItemRecord {
            id: item_id,
            class_id: class_id as u8,
            subclass_id,
            material: 0,
            inventory_type: inventory_type as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ))));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates(
        items
            .iter()
            .map(|&(item_id, inventory_type, flags3, _, _)| {
                (
                    item_id,
                    ItemSparseTemplateEntry {
                        flags: [0, 0, flags3, 0],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 0.0,
                        price_random_value: 0.0,
                        max_durability: 0,
                        other_faction_item_id: 0,
                        content_tuning_id: 0,
                        player_level_to_item_level_curve_id: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0, 0],
                        required_reputation_faction: 0,
                        allowable_class: -1,
                        required_expansion: 0,
                        bonding: ItemBondingType::None as u8,
                        container_slots: if inventory_type == InventoryType::Bag {
                            4
                        } else {
                            0
                        },
                        inventory_type: inventory_type as i8,
                    },
                )
            }),
    )));
}

fn sparse_template_for_inventory_type_like_cpp(
    inventory_type: InventoryType,
    flags3: u32,
) -> ItemSparseTemplateEntry {
    sparse_template_with_scaling_like_cpp(inventory_type, flags3, 0, 0)
}

fn sparse_template_with_scaling_like_cpp(
    inventory_type: InventoryType,
    flags3: u32,
    content_tuning_id: i32,
    player_level_to_item_level_curve_id: i32,
) -> ItemSparseTemplateEntry {
    ItemSparseTemplateEntry {
        flags: [0, 0, flags3, 0],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 0.0,
        price_random_value: 0.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id,
        player_level_to_item_level_curve_id,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: if inventory_type == InventoryType::Bag {
            4
        } else {
            0
        },
        inventory_type: inventory_type as i8,
    }
}

fn equip_represented_test_item_like_cpp(
    session: &mut WorldSession,
    slot: u8,
    item_guid: ObjectGuid,
    item_id: u32,
    inventory_type: InventoryType,
) {
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        owner,
        1,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(inventory_type as u8),
        },
    );
}

fn install_represented_item_level_curve_fixture_like_cpp(
    session: &mut WorldSession,
    item_id: u32,
    content_tuning_id: i32,
    curve_id: i32,
) {
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_with_scaling_like_cpp(
                    InventoryType::Chest,
                    0,
                    content_tuning_id,
                    curve_id,
                ),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 10,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_curve_store(Arc::new(CurveStore::from_entries([CurveEntry {
        id: curve_id as u32,
        curve_type: 0,
        flags: 0,
    }])));
    session.set_curve_point_store(Arc::new(CurvePointStore::from_entries([
        CurvePointEntry {
            id: 1,
            pos: [1.0, 101.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: curve_id as u32,
            order_index: 0,
        },
        CurvePointEntry {
            id: 2,
            pos: [60.0, 160.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: curve_id as u32,
            order_index: 1,
        },
    ])));
}

fn represented_test_item_record_like_cpp(
    item_id: u32,
    inventory_type: InventoryType,
    class_id: ItemClass,
    subclass_id: u8,
) -> ItemRecord {
    ItemRecord {
        id: item_id,
        class_id: class_id as u8,
        subclass_id,
        material: 0,
        inventory_type: inventory_type as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}

fn install_represented_pvp_item_level_fixture_like_cpp(
    session: &mut WorldSession,
    item_id: u32,
    pvp_delta: u8,
) {
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_pvp_item_store(Arc::new(PvpItemStore::from_entries([PvpItemEntry {
        id: 1,
        item_id: item_id as i32,
        item_level_delta: pvp_delta,
    }])));
}

fn represented_item_level_area_map_like_cpp(
    map_id: u32,
    instance_type: i8,
    flags2: u32,
) -> wow_data::map::MapEntry {
    wow_data::map::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2,
    }
}

fn install_create_map_difficulty_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    default_difficulty_id: u8,
    default_difficulty_flags: DifficultyFlags,
) {
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(
            2,
            MAP_INSTANCE_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT,
        ),
        difficulty_entry(
            3,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            u32::from(default_difficulty_id),
            MAP_RAID_LIKE_CPP,
            default_difficulty_flags,
        ),
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id,
            difficulty_id: default_difficulty_id,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
    ])));
}

fn represented_map_entry_for_create_map_context_like_cpp(
    map_id: u32,
    instance_type: i8,
) -> wow_data::map::MapEntry {
    wow_data::map::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }
}

fn install_create_map_active_lock_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
) {
    install_create_map_active_lock_stores_with_max_players_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
        10,
    );
}

fn install_create_map_active_lock_stores_with_max_players_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
    max_players: u32,
) {
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
        0,
        max_players,
    );
}

fn install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
    expansion_id: u8,
    max_players: u32,
) {
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: map_id,
            instance_type: wow_data::map::MAP_RAID,
            expansion_id,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: u32::from(difficulty_id),
        instance_type: MAP_RAID_LIKE_CPP,
        flags: DifficultyFlags::CAN_SELECT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id,
            difficulty_id,
            lock_id,
            reset_interval,
            max_players,
            flags: 0,
        },
    ])));
}

fn install_access_requirement_store_like_cpp(
    session: &mut WorldSession,
    requirement: wow_data::AccessRequirementLikeCpp,
) {
    session.set_access_requirement_store(Arc::new(
        wow_data::AccessRequirementStoreLikeCpp::from_entries_like_cpp([requirement]),
    ));
}

fn access_requirement_like_cpp(map_id: u32, difficulty: u8) -> wow_data::AccessRequirementLikeCpp {
    wow_data::AccessRequirementLikeCpp {
        map_id,
        difficulty,
        level_min: 0,
        level_max: 0,
        item: 0,
        item2: 0,
        quest_done_a: 0,
        quest_done_h: 0,
        completed_achievement: 0,
        quest_failed_text: String::new(),
    }
}

fn trinity_string_entry_like_cpp(
    entry: u32,
    content_default: &str,
) -> wow_data::TrinityStringEntryLikeCpp {
    wow_data::TrinityStringEntryLikeCpp {
        entry,
        content: std::array::from_fn(|idx| {
            if idx == 0 {
                content_default.to_string()
            } else {
                String::new()
            }
        }),
    }
}

fn install_access_notification_stores_like_cpp(session: &mut WorldSession) {
    session.set_trinity_string_store(Arc::new(
        wow_data::TrinityStringStoreLikeCpp::from_entries_like_cpp([
            trinity_string_entry_like_cpp(
                wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP,
                "You must be at least level %u to enter.",
            ),
            trinity_string_entry_like_cpp(
                wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                "You must be at least level %u and have %s to enter.",
            ),
        ]),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: 700,
            allowable_race: 0,
            display: "The Workshop Key".to_string(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: 701,
            allowable_race: 0,
            display: "The Scarlet Key".to_string(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
    ])));
}

fn install_create_map_encounter_lock_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
) {
    install_create_map_active_lock_stores_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
    );
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 901,
            message: String::new(),
            map_id,
            difficulty_id,
            lock_id,
            reset_interval,
            max_players: 10,
            flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        },
    ])));
}

fn install_active_instance_lock_mgr_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    map_id: u32,
    difficulty_id: u8,
    instance_id: u32,
) -> u64 {
    let entries = session
        .create_map_db2_entries_like_cpp(map_id, difficulty_id)
        .unwrap();
    let now = u64::try_from(unix_now()).unwrap_or(0);
    let mut mgr = wow_instances::InstanceLockMgr::default();
    let lock = mgr
        .create_instance_lock_for_new_instance_at(
            owner_guid,
            &entries,
            instance_id,
            wow_instances::ResetSchedule::default(),
            now,
        )
        .unwrap();
    let expected_token = create_map_instance_lock_token_like_cpp(owner_guid, &entries, lock);
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(mgr)));
    expected_token
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(bytes);
    }
    packets
}

fn party_update_sequence_num_like_cpp(bytes: &[u8]) -> i32 {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::PartyUpdate as u16
    );
    let _party_flags = pkt.read_uint16().unwrap();
    let _party_index = pkt.read_uint8().unwrap();
    let _party_type = pkt.read_uint8().unwrap();
    let _my_index = pkt.read_int32().unwrap();
    let _party_guid = pkt.read_packed_guid().unwrap();
    pkt.read_int32().unwrap()
}

fn packet_contains_quest_ids_in_order(bytes: &[u8], quest_ids: &[u32]) -> bool {
    let mut search_from = 0usize;
    for quest_id in quest_ids {
        let needle = quest_id.to_le_bytes();
        let Some(relative_pos) = bytes[search_from..]
            .windows(needle.len())
            .position(|window| window == needle)
        else {
            return false;
        };
        search_from += relative_pos + needle.len();
    }
    true
}

#[derive(Debug, PartialEq, Eq)]
struct QuestGiverRequestItemsSummaryLikeCpp {
    giver_creature_id: i32,
    quest_id: u32,
    status_flags: u32,
    auto_launched: bool,
}

fn quest_giver_request_items_summary_like_cpp(
    bytes: &[u8],
) -> QuestGiverRequestItemsSummaryLikeCpp {
    let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
    pkt.read_packed_guid().unwrap();
    let giver_creature_id = pkt.read_int32().unwrap();
    let quest_id = pkt.read_int32().unwrap() as u32;
    pkt.read_int32().unwrap(); // CompEmoteDelay
    pkt.read_int32().unwrap(); // CompEmoteType
    pkt.read_uint32().unwrap(); // QuestFlags[0]
    pkt.read_uint32().unwrap(); // QuestFlags[1]
    pkt.read_uint32().unwrap(); // QuestFlags[2]
    pkt.read_int32().unwrap(); // SuggestedPartyMembers
    pkt.read_int32().unwrap(); // MoneyToGet
    let collect_count = pkt.read_int32().unwrap().max(0) as usize;
    let currency_count = pkt.read_int32().unwrap().max(0) as usize;
    let status_flags = pkt.read_int32().unwrap() as u32;

    for _ in 0..collect_count {
        pkt.read_int32().unwrap(); // ObjectID
        pkt.read_int32().unwrap(); // Amount
        pkt.read_uint32().unwrap(); // Flags
    }
    for _ in 0..currency_count {
        pkt.read_int32().unwrap(); // CurrencyID
        pkt.read_int32().unwrap(); // Amount
    }

    QuestGiverRequestItemsSummaryLikeCpp {
        giver_creature_id,
        quest_id,
        status_flags,
        auto_launched: pkt.read_bit().unwrap(),
    }
}

fn quest_list_level_fields_like_cpp(bytes: &[u8]) -> Vec<(u32, i32, i32)> {
    let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
    pkt.read_packed_guid().unwrap();
    pkt.read_uint32().unwrap();
    pkt.read_uint32().unwrap();
    let quest_count = pkt.read_uint32().unwrap();
    let greeting_len = pkt.read_bits(11).unwrap();

    let mut levels = Vec::new();
    for _ in 0..quest_count {
        let quest_id = pkt.read_uint32().unwrap();
        let _content_tuning_id = pkt.read_uint32().unwrap();
        let _quest_type = pkt.read_int32().unwrap();
        let quest_level = pkt.read_int32().unwrap();
        let quest_max_scaling_level = pkt.read_int32().unwrap();
        let _quest_flags = pkt.read_uint32().unwrap();
        let _quest_flags_ex = pkt.read_uint32().unwrap();
        let _repeatable = pkt.read_bit().unwrap();
        let _important = pkt.read_bit().unwrap();
        let title_len = pkt.read_bits(9).unwrap();
        let _title = pkt.read_string(title_len as usize).unwrap();
        levels.push((quest_id, quest_level, quest_max_scaling_level));
    }

    let _greeting = pkt.read_string(greeting_len as usize).unwrap();
    levels
}

// ── Slice 4A.1b: SendIfVisibleLikeCpp per-session gate tests ────────────
// C++ anchor: GridNotifiers.h : MessageDistDeliverer::SendPacket — HaveAtClient
// (client_visible_guids_like_cpp) is the final gate; map/instance filter first.

fn install_committed_canonical_player_health_for_melee_test_like_cpp(
    session: &mut WorldSession,
    victim_guid: ObjectGuid,
    health: u64,
    death_state: wow_constants::DeathState,
) -> u64 {
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, victim_guid, Position::ZERO, 571, 0);
    let revision = {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(victim_guid)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        player.unit_mut().set_health(health);
        player.unit_mut().set_death_state(death_state);
        player.unit().health_state_revision_like_cpp()
    };
    session.set_canonical_map_manager(canonical);
    revision
}

fn expected_gameobject_dynamic_flags_update_like_cpp(
    guid: ObjectGuid,
    map_id: u16,
    dynamic_flags: u32,
) -> Vec<u8> {
    represented_gameobject_dynamic_flags_update_like_cpp(guid, map_id, dynamic_flags)
        .expect("dynamic flags update")
        .to_bytes()
}

fn expected_active_player_farsight_object_values_update_like_cpp(
    player_guid: ObjectGuid,
    map_id: u16,
    farsight_guid: ObjectGuid,
) -> Vec<u8> {
    use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

    let mut data = ActivePlayerDataValuesUpdate::default();
    set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 0);
    set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 26);
    data.farsight_object = farsight_guid;
    UpdateObject::full_active_player_values_update(player_guid, map_id, data).to_bytes()
}

fn expected_dynamic_object_create_packet_like_cpp(
    map_id: u16,
    create_data: wow_packet::packets::update::DynamicObjectCreateData,
) -> Vec<u8> {
    use wow_packet::packets::update::UpdateObject;

    UpdateObject::create_world_objects(
        vec![UpdateObject::create_dynamic_object_block(create_data)],
        map_id,
    )
    .to_bytes()
}

fn update_object_packet_count_like_cpp(packets: &[Vec<u8>]) -> usize {
    packets
        .iter()
        .filter(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)
        })
        .count()
}

fn test_quest_template(id: u32) -> wow_data::quest::QuestTemplate {
    wow_data::quest::QuestTemplate {
        id,
        quest_type: 0,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        log_title: String::new(),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn seasonal_test_quest_template(
    id: u32,
    quest_sort_id: i32,
    event_id_for_quest: u16,
) -> wow_data::quest::QuestTemplate {
    let mut quest = test_quest_template(id);
    quest.quest_sort_id = quest_sort_id;
    quest.min_level = 0;
    quest.event_id_for_quest = event_id_for_quest;
    quest
}

fn seasonal_quest_store_like_cpp(
    ids: impl IntoIterator<Item = u32>,
) -> wow_data::quest::QuestStore {
    wow_data::quest::QuestStore::from_quests_like_cpp(
        ids.into_iter()
            .map(|id| seasonal_test_quest_template(id, -376, 9)),
    )
}

/// Move an old handle-less quest fixture onto the same generation-checked
/// canonical `Player` owner exercised by production before the behavior under
/// test can trigger map/visibility work and establish that owner itself.
fn adopt_player_quest_fixture_into_canonical_owner_like_cpp(session: &mut WorldSession) {
    assert!(session.player_handle_like_cpp.is_none());
    let quests = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("handle-less quest fixture");
    let currencies = session
        .player_currencies_like_cpp()
        .expect("handle-less currency fixture");
    let gold = session.player_gold_like_cpp();
    let reputation = session.reputation_mgr_like_cpp().clone();
    install_canonical_player_owner_for_test(session, 0, 0);
    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    assert!(session.set_player_currencies_like_cpp(currencies));
    session.set_player_gold_like_cpp(gold);
    assert!(
        session
            .mutate_reputation_mgr_like_cpp(|manager| *manager = reputation)
            .is_some()
    );
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| *state = quests)
            .is_some(),
        "quest fixture must move to the canonical Player owner"
    );
}

fn assert_canonical_quest_status_like_cpp(
    session: &WorldSession,
    quest_id: u32,
    expected_status: Option<u8>,
    expected_rewarded: bool,
) {
    let state = session
        .player_quest_gameplay_snapshot_like_cpp()
        .expect("canonical Player quest state");
    assert_eq!(
        state.statuses.get(&quest_id).map(|status| status.status),
        expected_status
    );
    assert_eq!(
        state.rewarded_quest_ids.contains(&quest_id),
        expected_rewarded
    );
}

fn seasonal_quest_v2_store_like_cpp(entries: impl IntoIterator<Item = (u32, u16)>) -> QuestV2Store {
    QuestV2Store::from_entries(entries.into_iter().map(|(id, unique_bit_flag)| {
        wow_data::progression_rewards::QuestV2Entry {
            id,
            unique_bit_flag,
        }
    }))
}

fn summon_go_template_store_like_cpp(
    entry: u32,
) -> Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp> {
    Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: 6,
                display_id: 44,
                name: "spell summoned gameobject".to_string(),
                size: 1.0,
                data: [0; wow_entities::MAX_GAMEOBJECT_DATA],
                content_tuning_id: 80,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    )
}

fn summon_go_spell_misc_entry_like_cpp(
    spell_id: u32,
    duration_index: u16,
) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id: spell_id,
        attributes: [0; 15],
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

fn summon_object_wild_effect_like_cpp(entry: i32) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD,
        effect_misc_value_1: entry,
        ..Default::default()
    }
}

fn summon_object_slot_effect_like_cpp(entry: i32, slot: u32) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect_index: slot,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1 + slot,
        effect_misc_value_1: entry,
        ..Default::default()
    }
}

fn gameobject_summon_spell_info_like_cpp(
    spell_id: i32,
    requires_spell_focus: u32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus,
        power_costs: Vec::new(),
        effects,
    }
}

fn teleport_units_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects,
    }
}

fn bind_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects,
    }
}

fn environmental_damage_spell_info_like_cpp(
    spell_id: i32,
    effects: Vec<wow_data::SpellEffectInfo>,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects,
    }
}

fn configure_gameobject_summon_live_session_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    player_position: Position,
    template_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    spell_info: wow_data::SpellInfo,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Summoner".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(canonical, player_guid, player_position, 571, 0);
    session.set_gameobject_template_lifecycle_store(template_store);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        summon_go_spell_misc_entry_like_cpp(spell_info.spell_id as u32, 0),
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 0,
            duration: 0,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_info.spell_id, spell_info);
    session.set_spell_store(Arc::new(spell_store));
}

fn target_data_with_destination_like_cpp(position: Position) -> SpellTargetData {
    SpellTargetData {
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position,
        }),
        ..Default::default()
    }
}

fn insert_test_player_into_canonical_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    position: Position,
) {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();

    let mut manager = canonical.lock().unwrap();
    manager
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_player(player).unwrap(),
        )
        .unwrap();
}

fn decode_spell_go_target_data_like_cpp(bytes: &[u8], expected_spell_id: i32) -> SpellTargetData {
    let mut spell_go = WorldPacket::from_bytes(bytes);
    assert_eq!(
        spell_go.read_uint16().expect("SpellGo opcode"),
        ServerOpcodes::SpellGo as u16
    );
    for label in ["caster", "caster unit", "cast id", "original cast id"] {
        spell_go.read_packed_guid().expect(label);
    }
    assert_eq!(spell_go.read_int32().expect("spell id"), expected_spell_id);
    wow_packet::packets::spell::SpellCastVisual::read(&mut spell_go).expect("spell visual");
    spell_go.read_uint32().expect("cast flags");
    spell_go.read_uint32().expect("cast flags ex");
    spell_go.read_uint32().expect("cast time");
    spell_go.read_int32().expect("missile travel time");
    spell_go.read_float().expect("missile pitch");
    spell_go.read_uint8().expect("destination cast index");
    spell_go.read_uint32().expect("immunity school");
    spell_go.read_uint32().expect("immunity value");
    spell_go.read_uint32().expect("heal prediction points");
    spell_go.read_uint8().expect("heal prediction type");
    spell_go.read_packed_guid().expect("heal prediction beacon");
    spell_go.read_bits(16).expect("hit target count");
    spell_go.read_bits(16).expect("miss target count");
    spell_go.read_bits(16).expect("miss status count");
    spell_go.read_bits(9).expect("remaining power count");
    spell_go.read_bit().expect("remaining runes presence");
    spell_go.read_bits(16).expect("target point count");
    spell_go.read_bit().expect("ammo display presence");
    spell_go.read_bit().expect("ammo inventory presence");
    SpellTargetData::read(&mut spell_go).expect("SpellGo target data")
}

fn shared_map_manager() -> crate::map_manager::SharedMapManager {
    Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()))
}

fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

fn configure_player_shape_mount_collision_stores_like_cpp(session: &mut WorldSession) {
    let native_display_id = crate::handlers::character::default_display_id(
        session.player_race_like_cpp(),
        session.player_gender_like_cpp(),
    );
    session.set_creature_template_mount_store(Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_entries([
            wow_data::CreatureTemplateMountEntryLikeCpp {
                entry: 1234,
                vehicle_id: 55,
                models: vec![wow_data::CreatureTemplateMountModelLikeCpp {
                    display_id: 4321,
                    display_scale: 1.0,
                    probability: 0.0,
                }],
            },
        ]),
    ));
    session.set_creature_display_info_store(Arc::new(
        wow_data::CreatureDisplayInfoStore::from_entries([
            wow_data::CreatureDisplayInfoEntry {
                id: native_display_id,
                model_id: 100,
                extended_display_info_id: 0,
                creature_model_scale: 1.2,
            },
            wow_data::CreatureDisplayInfoEntry {
                id: 4321,
                model_id: 200,
                extended_display_info_id: 0,
                creature_model_scale: 1.5,
            },
        ]),
    ));
    session.set_creature_model_data_store(Arc::new(
        wow_data::CreatureModelDataStore::from_entries([
            wow_data::CreatureModelDataEntry {
                id: 100,
                flags: 0,
                file_data_id: 0,
                collision_height: 2.0,
                hover_height: 0.75,
                model_scale: 1.1,
                mount_height: 0.0,
            },
            wow_data::CreatureModelDataEntry {
                id: 200,
                flags: 0,
                file_data_id: 0,
                collision_height: 0.0,
                hover_height: 1.25,
                model_scale: 1.0,
                mount_height: 4.0,
            },
        ]),
    ));
}

fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

fn test_vehicle_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Vehicle, 0, 1, 0, 0, 2, counter)
}

fn test_pet_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 3, counter)
}

fn test_gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::GameObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn add_canonical_test_creature(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
) {
    add_canonical_test_creature_on_map(canonical, guid, entry, position, npc_flags, 571, 0);
}

fn add_canonical_test_creature_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_creature_on_map_with_world_state(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        true,
    );
}

fn add_canonical_test_creature_indexed_on_map_with_level(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
    level: u8,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, 0, 0);

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
        )
        .unwrap();
}

fn add_canonical_test_creature_on_map_with_world_state(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
) {
    add_canonical_test_creature_on_map_with_world_state_and_owner(
        canonical,
        guid,
        entry,
        position,
        npc_flags,
        map_id,
        instance_id,
        is_in_world,
        None,
    );
}

fn add_canonical_test_creature_on_map_with_world_state_and_owner(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    map_id: u32,
    instance_id: u32,
    is_in_world: bool,
    owner_guid: Option<ObjectGuid>,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(owner_guid);
    if is_in_world {
        creature.unit_mut().world_mut().object_mut().add_to_world();
    }

    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn add_canonical_test_pet(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
) {
    add_canonical_test_pet_with_visible_aura(
        canonical, guid, owner_guid, entry, position, npc_flags, None, None,
    );
}

fn add_canonical_test_pet_with_visible_aura(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    npc_flags: u32,
    visible_aura: Option<(u8, u32, ObjectGuid, u32)>,
    visible_application: Option<wow_entities::VisibleAuraApplicationLikeCpp>,
) {
    let mut pet = wow_entities::Pet::new(owner_guid, wow_entities::PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_combat_reach(1.0);
    pet.creature_mut().unit_mut().set_level(80);
    pet.creature_mut().unit_mut().set_max_health(100);
    pet.creature_mut().unit_mut().set_health(100);
    pet.creature_mut()
        .set_ai_identity_runtime(1, 35, npc_flags, 0);
    if let Some((slot, spell_id, caster_guid, effect_mask)) = visible_aura {
        let aura = wow_entities::AppliedAuraRef::new(spell_id, caster_guid, slot, effect_mask);
        pet.creature_mut()
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(aura);
        if let Some(application) = visible_application {
            pet.creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_visible_with_application_like_cpp(slot, aura.aura_ref(), application);
        } else {
            pet.creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_visible(slot, aura.aura_ref());
        }
    }
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}

fn add_canonical_test_pet_with_number(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    entry: u32,
    position: Position,
    pet_number: u32,
    created_by_spell_id: u32,
    duration_ms: i32,
) {
    let mut pet = wow_entities::Pet::new(owner_guid, wow_entities::PetType::Hunter);
    pet.set_created_by_spell_id_like_cpp(created_by_spell_id);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .init_charm_info()
        .pet_number = pet_number;
    pet.set_duration(duration_ms);

    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}

fn add_canonical_test_gameobject(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
) {
    add_canonical_test_gameobject_on_map(canonical, guid, entry, position, 571, 0);
}

fn add_canonical_test_player_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        guid,
        position,
        map_id,
        instance_id,
        0,
    );
}

/// C++ `Player::GetNPCIfCanInteractWith` requires the canonical Player to be
/// in-world, alive and to carry a resolvable faction before a positive NPC
/// interaction fixture is meaningful.
fn adopt_live_canonical_test_player_for_interaction_like_cpp(session: &mut WorldSession) {
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            player.unit_mut().set_faction(1);
        })
        .expect("live canonical Player interaction fixture");
}

fn bind_canonical_test_player_to_registry_like_cpp(
    session: &mut WorldSession,
    registry: &Arc<PlayerRegistry>,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
) -> SharedCanonicalMapManager {
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, position, map_id, 0);
    canonical
}

fn canonical_party_type_for_test(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> [u8; 2] {
    with_canonical_player_at_like_cpp(canonical, guid, 571, 0, |player| player.data().party_type)
        .expect("canonical player")
}

fn add_canonical_test_player_on_map_with_difficulty(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
    difficulty_id: u8,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("InstanceOwner");
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .create_map_entry(
            map_id,
            instance_id,
            difficulty_id,
            wow_map::ManagedMapKind::World,
        )
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn add_canonical_test_gameobject_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn add_canonical_lifecycle_gameobject_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    rotation: [f32; 4],
    map_id: u32,
    instance_id: u32,
) {
    let mut template_data = [0_u32; wow_entities::MAX_GAMEOBJECT_DATA];
    template_data[wow_entities::GAMEOBJECT_DATA_CHEST_LOOT] = 9_001;
    let gameobject =
        GameObject::try_create_from_lifecycle(wow_entities::GameObjectCreateLifecycleRecord {
            guid,
            map_id,
            instance_id,
            position,
            rotation,
            anim_progress: 33,
            go_state: wow_entities::GoState::Ready,
            art_kit: 4,
            dynamic: false,
            spawn_id: 98_765,
            template: wow_entities::GameObjectTemplateLifecycleRecord {
                entry,
                name: "canonical visible chest".to_string(),
                go_type: wow_entities::GAMEOBJECT_TYPE_CHEST,
                display_id: 7_777,
                scale: 1.75,
                faction: 35,
                flags: 0x24,
                data: template_data,
                world_effect_id: 0,
                anim_kit_id: 0,
                level: 80,
                percent_health: 100,
                custom_param: 0,
            },
        })
        .expect("valid gameobject lifecycle");

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    map.map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn add_canonical_spell_focus_gameobject_on_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    focus_type: u32,
    radius: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_represented_spell_focus_use_source_like_cpp(Some(
        wow_entities::SpellFocusUseSource {
            focus_type,
            radius,
            linked_trap_entry: 0,
        },
    ));

    let mut guard = canonical.lock().unwrap();
    guard
        .create_world_map(map_id, instance_id)
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn add_canonical_visibility_on_destroy_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(5);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_delay_time(30);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn add_canonical_visual_despawn_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(5);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_go_anim_progress_like_cpp(1);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
    gameobject.set_flags(0x88);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn add_canonical_capture_point_delete_gameobject_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    entry: u32,
    spawn_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(map_id, instance_id).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(wow_entities::GAMEOBJECT_TYPE_CAPTURE_POINT as u8);
    gameobject.set_spawn_id(u64::from(spawn_id));
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    assert!(gameobject.schedule_despawn_or_unsummon_like_cpp(1, 0));
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject.world().clone());
    gameobject.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn test_dynamic_object_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::DynamicObject,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn test_area_trigger_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::AreaTrigger,
        0,
        1,
        571,
        0,
        entry,
        counter,
    )
}

fn prepare_dynamic_object_values_snapshot_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    dynamic_object_guid: ObjectGuid,
    radius: f32,
) {
    {
        let mut guard = canonical.lock().unwrap();
        let managed = guard.create_world_map(map_id, instance_id);
        let record = managed
            .map_mut()
            .get_typed_dynamic_object_mut(dynamic_object_guid)
            .unwrap();
        record.set_radius(radius);
    }
    let mut guard = canonical.lock().unwrap();
    let _ = guard.update(1);
    assert_eq!(
        guard
            .find_map(map_id, instance_id)
            .unwrap()
            .last_send_object_updates_summary_like_cpp()
            .dynamic_object_values_updates
            .len(),
        1
    );
    assert!(
        !guard
            .find_map(map_id, instance_id)
            .unwrap()
            .map()
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .dynamic_object_data_changes_mask()
            .is_any_set()
    );
}

fn configure_dynamic_object_values_snapshot_session_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DynamicObjectViewer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        map_id as u16,
        1,
        1,
        80,
        0,
    ));
    session.set_state(SessionState::LoggedIn);
    add_canonical_test_player_on_map(
        canonical,
        player_guid,
        Position::new(10.0, 20.0, 30.0, 0.0),
        map_id,
        instance_id,
    );
}

fn add_shared_vision_viewer_to_canonical_target_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    target_guid: ObjectGuid,
    viewer_guid: ObjectGuid,
) {
    let mut guard = canonical.lock().unwrap();
    let map = guard.find_map_mut(map_id, instance_id).unwrap().map_mut();
    if let Some(target) = map.get_typed_player_mut(target_guid) {
        target
            .unit_mut()
            .subsystems_mut()
            .control
            .add_shared_vision(viewer_guid);
    } else {
        map.get_typed_creature_mut(target_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .add_shared_vision(viewer_guid);
    }
}

fn add_canonical_test_dynamic_object_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    caster: ObjectGuid,
    spell_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut dynamic_object = wow_entities::DynamicObject::new(true);
    dynamic_object.world_mut().object_mut().create(guid);
    dynamic_object.world_mut().object_mut().set_entry(spell_id);
    dynamic_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    dynamic_object.world_mut().relocate(position);
    dynamic_object.set_caster_guid(caster);
    dynamic_object.set_dynamic_object_type(wow_entities::DynamicObjectType::FarsightFocus);
    dynamic_object.set_spell_visual_id(700);
    dynamic_object.set_spell_id(spell_id as i32);
    dynamic_object.set_radius(25.0);
    dynamic_object.set_cast_time_ms(1500);
    dynamic_object.set_duration(5000);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map.map_mut().add_to_map_like_cpp(
        AccessorObjectKind::DynamicObject,
        dynamic_object.world().clone(),
    );
    dynamic_object.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
        )
        .unwrap();
}

fn add_canonical_visibility_misc_objects_on_map(
    canonical: &SharedCanonicalMapManager,
    corpse_guid: ObjectGuid,
    scene_object_guid: ObjectGuid,
    conversation_guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 0);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().object_mut().set_entry(501);
    corpse.world_mut().set_map(map_id, instance_id).unwrap();
    corpse.world_mut().relocate(position);
    corpse.set_display_id(7001);
    corpse.set_race(1);
    corpse.set_class(1);
    corpse.set_owner_guid(ObjectGuid::create_player(1, 501));

    let mut scene_object = wow_entities::SceneObject::new();
    scene_object
        .world_mut()
        .object_mut()
        .create(scene_object_guid);
    scene_object.world_mut().object_mut().set_entry(502);
    scene_object
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    scene_object.world_mut().relocate(position);
    scene_object.relocate_stationary_position(position);
    scene_object.set_script_package_id(502);
    scene_object.set_rnd_seed_val(1234);

    let mut conversation = wow_entities::Conversation::new();
    conversation
        .world_mut()
        .object_mut()
        .create(conversation_guid);
    conversation.world_mut().object_mut().set_entry(503);
    conversation
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    conversation.world_mut().relocate(position);
    conversation.relocate_stationary_position(position);
    conversation.set_duration_ms(10_000);
    conversation.set_texture_kit_id(77);
    conversation.add_line(wow_entities::ConversationLine {
        conversation_line_id: 9,
        start_time: 10,
        ui_camera_id: 0,
        actor_index: 0,
        flags: 0,
    });

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    for (kind, world) in [
        (AccessorObjectKind::Corpse, corpse.world().clone()),
        (
            AccessorObjectKind::SceneObject,
            scene_object.world().clone(),
        ),
        (
            AccessorObjectKind::Conversation,
            conversation.world().clone(),
        ),
    ] {
        map.map_mut().add_to_map_like_cpp(kind, world).unwrap();
    }
    corpse.world_mut().object_mut().add_to_world();
    scene_object.world_mut().object_mut().add_to_world();
    conversation.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_corpse(corpse).unwrap())
        .unwrap();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_scene_object(scene_object).unwrap(),
        )
        .unwrap();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_conversation(conversation).unwrap(),
        )
        .unwrap();
}

fn add_canonical_test_area_trigger_on_map(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    caster: ObjectGuid,
    spell_id: u32,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut area_trigger = wow_entities::AreaTrigger::new();
    area_trigger.world_mut().object_mut().create(guid);
    area_trigger.world_mut().object_mut().set_entry(spell_id);
    area_trigger
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    area_trigger.world_mut().relocate(position);
    area_trigger.set_caster_guid(caster);
    area_trigger.set_spell_id(spell_id as i32);
    area_trigger.set_duration(5000);

    let mut guard = canonical.lock().unwrap();
    let map = guard.create_world_map(map_id, instance_id);
    let _ = map.map_mut().add_to_map_like_cpp(
        AccessorObjectKind::AreaTrigger,
        area_trigger.world().clone(),
    );
    area_trigger.world_mut().object_mut().add_to_world();
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
        )
        .unwrap();
}

fn configure_test_dynamic_object_for_visual_despawn_like_cpp(
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    duration_ms: i32,
    bound_caster: Option<ObjectGuid>,
    phase_shift: Option<PhaseShift>,
) {
    let mut guard = canonical.lock().unwrap();
    let dynamic_object = guard
        .find_map_mut(map_id, instance_id)
        .unwrap()
        .map_mut()
        .get_typed_dynamic_object_mut(guid)
        .unwrap();
    dynamic_object.set_duration(duration_ms);
    if let Some(bound_caster) = bound_caster {
        dynamic_object.bind_to_caster(bound_caster);
    }
    if let Some(phase_shift) = phase_shift {
        *dynamic_object.world_mut().phase_shift_mut() = phase_shift;
    }
}

fn configure_add_farsight_live_session_like_cpp(
    session: &mut WorldSession,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    player_guid: ObjectGuid,
    spell_id: i32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Farseer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 1500,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT,
                effect_radius_index_1: 11,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: 1,
            attributes: [0; 15],
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 7,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: spell_id as u32,
        },
    ])));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 7,
            duration: -5000,
            duration_per_level: 0,
            max_duration: 0,
        },
    ])));
    session.set_spell_radius_store(Arc::new(wow_data::SpellRadiusStore::from_entries([
        wow_data::SpellRadiusEntry {
            id: 11,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: 25.0,
        },
    ])));
}

fn add_farsight_target_data(destination: Option<Position>) -> SpellTargetData {
    SpellTargetData {
        flags: if destination.is_some() { 0x40 } else { 0 },
        dst_location: destination.map(|position| wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::EMPTY,
            position,
        }),
        ..Default::default()
    }
}

fn canonical_player_transfer_test_map_store_like_cpp() -> Arc<wow_data::MapStore> {
    Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]))
}

fn test_creature_create_data(
    guid: ObjectGuid,
    entry: u32,
    hp: u32,
) -> wow_packet::packets::update::CreatureCreateData {
    wow_packet::packets::update::CreatureCreateData {
        guid,
        entry,
        display_id: 100,
        native_display_id: 100,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: hp as i64,
        max_health: hp as i64,
        level: 2,
        faction_template: 14,
        npc_flags: 0,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    }
}

fn register_test_creature(
    session: &mut WorldSession,
    manager: crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
    hp: u32,
) {
    session.set_map_manager(manager);
    session.current_map_id = 0;
    if session.player_position_like_cpp().is_none() {
        session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    }
    session.register_world_creature(
        0,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, hp),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(guid, |creature| {
            // This test fixture represents a fully hydrated DB-backed
            // creature with no addon/template-addon auras.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(true);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_cast_log_aura_authority_inert_like_cpp(true);
            creature.seed_runtime_rng_like_cpp(0x5E11_117);
        })
        .expect("registered test creature must remain available");
}

/// Register a legacy creature the way production does: through a session
/// that already owns the canonical map manager.
///
/// `register_world_creature` reconciles the legacy and canonical loot and
/// health-state authorities while mirroring, which is what makes the two
/// entities one incarnation. Cast and melee paths that prove that identity
/// need this fixture rather than two independently constructed creatures.
fn register_test_creature_mirrored_like_cpp(
    session: &mut WorldSession,
    manager: crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    hp: u32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    register_test_creature(session, manager, guid, hp);
}

fn creature_template_lifecycle_store_for_test(
    entries: impl IntoIterator<Item = u32>,
) -> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates(entries.into_iter().map(
        |entry| wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: format!("Creature {entry}"),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            faction: 14,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family: 0,
            trainer_class: 0,
            unit_class: 1,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ))
}

fn lfg_dungeon_entry_for_test(
    id: u32,
    map_id: i16,
    difficulty_id: u8,
    target_level: u8,
) -> wow_data::LfgDungeonsEntry {
    wow_data::LfgDungeonsEntry {
        id,
        name: String::new(),
        description: String::new(),
        min_level: 0,
        max_level: 0,
        type_id: 0,
        subtype: 0,
        faction: 0,
        icon_texture_file_id: 0,
        rewards_bg_texture_file_id: 0,
        popup_bg_texture_file_id: 0,
        expansion_level: 0,
        map_id,
        difficulty_id,
        min_gear: 0.0,
        group_id: 0,
        order_index: 0,
        required_player_condition_id: 0,
        target_level,
        target_level_min: 0,
        target_level_max: 0,
        random_id: 0,
        scenario_id: 0,
        final_encounter_id: 0,
        count_tank: 0,
        count_healer: 0,
        count_damage: 0,
        min_count_tank: 0,
        min_count_healer: 0,
        min_count_damage: 0,
        bonus_reputation_amount: 0,
        mentor_item_level: 0,
        mentor_char_level: 0,
        flags: [0; 2],
    }
}

fn configure_single_creature_kill_reputation_for_test(session: &mut WorldSession) {
    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let creature_template_store = creature_template_lifecycle_store_for_test([9001]);
    let (onkill_store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
                creature_id: 9001,
                entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: 7,
                    rep_faction_2: 0,
                    reputation_max_cap_1: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_1: 250,
                    reputation_max_cap_2: 0,
                    rep_value_2: 0,
                    is_team_award_1: false,
                    is_team_award_2: false,
                    team_dependent: false,
                },
            }],
            &creature_template_store,
            &faction_store,
        );
    assert_eq!(report.loaded, 1);
    session.set_faction_store(Arc::new(faction_store));
    session.set_creature_onkill_reputation_store(Arc::new(onkill_store));
}

fn configure_two_player_group_for_reputation_test(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    other_guid: ObjectGuid,
) {
    let (other_tx, _other_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut other_info = broadcast_info(other_guid, other_tx);
    other_info.placement.map_id = 0;
    other_info.placement.position = Position::new(10.0, 10.0, 0.0, 0.0);
    other_info.placement.level = 80;
    other_info.placement.is_alive = true;
    player_registry.register_or_replace(other_guid, other_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
}

fn reputation_aura_for_test(
    slot: u8,
    effect: RepresentedAuraEffectLikeCpp,
    amount: i32,
    misc_value: Option<i32>,
) -> AuraApplication {
    AuraApplication {
        spell_id: 69_500 + i32::from(slot),
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x0000_0001,
        effect_mask: 0x0000_0001,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: Some(effect),
        represented_amount: amount,
        represented_effect_amounts: vec![RepresentedAuraEffectAmountLikeCpp {
            effect_index: 0,
            amount,
        }],
        represented_misc_value: misc_value,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn install_stackable_test_item_template(
    session: &mut WorldSession,
    entry: u32,
    max_stack_size: i32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn send_new_item_plan(delivery: SendNewItemDelivery) -> SendNewItemPlan {
    SendNewItemPlan {
        player_guid: ObjectGuid::create_player(1, 42),
        item_guid: ObjectGuid::create_item(1, 500),
        item_entry: 9001,
        item_instance: SendNewItemInstancePlan {
            item_id: 9001,
            random_properties_seed: 456,
            random_properties_id: -77,
            modifications: vec![
                SendNewItemModifier {
                    value: 123,
                    modifier_type: 3,
                },
                SendNewItemModifier {
                    value: 25,
                    modifier_type: 5,
                },
            ],
        },
        slot: 4,
        slot_in_bag: 7,
        quest_log_item_id: 777,
        quantity: 3,
        quantity_in_inventory: 9,
        battle_pet_species_id: 123,
        battle_pet_breed_id: 0xBC,
        battle_pet_breed_quality: 0x1A,
        battle_pet_level: 25,
        pushed: true,
        created: false,
        display_text: SendNewItemDisplayText::EncounterLoot,
        dungeon_encounter_id: 615,
        is_encounter_loot: true,
        delivery,
    }
}

fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    broadcast_info_with_command(guid, send_tx, command_tx)
}

fn broadcast_info_with_command(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new(
            format!("Player{}", guid.counter()),
            guid.counter() as u32,
            0,
            1,
            1,
            0,
            2,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn represented_vehicle_interact_map_store_like_cpp(instance_type: i8) -> Arc<MapStore> {
    Arc::new(MapStore::from_entries([wow_data::MapEntry {
        id: 571,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]))
}

fn insert_represented_vehicle_target_like_cpp(
    registry: &PlayerRegistry,
    canonical: &SharedCanonicalMapManager,
    target_guid: ObjectGuid,
    position: Position,
    has_vehicle_kit_like_cpp: bool,
) {
    let (send_tx, _send_rx) = flume::bounded(4);
    let mut info = broadcast_info(target_guid, send_tx);
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.position = position;
    registry.register_or_replace(target_guid, info, Default::default());
    add_canonical_test_player_on_map(canonical, target_guid, position, 571, 0);
    assert!(
        with_canonical_player_at_mut_like_cpp(canonical, target_guid, 571, 0, |player| {
            player.gameplay_state_mut().mount_vehicle_kit = has_vehicle_kit_like_cpp.then(|| {
                represented_vehicle_kit_with_passenger_like_cpp(
                    target_guid,
                    test_creature_guid(62_000),
                    true,
                )
            });
        })
        .is_some()
    );
}

fn represented_vehicle_interact_session_like_cpp(
    target_guid: ObjectGuid,
    target_has_vehicle_kit: bool,
    target_position: Position,
    group_target: bool,
    map_instance_type: i8,
) -> WorldSession {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 62_001);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    if group_target {
        group.add_member(target_guid);
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_name_like_cpp("RideVehicleInteractTester".to_string());
    session.set_player_map_position_like_cpp(571, Position::new(0.0, 0.0, 0.0, 0.0));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_map_store(represented_vehicle_interact_map_store_like_cpp(
        map_instance_type,
    ));
    insert_represented_vehicle_target_like_cpp(
        &registry,
        &canonical,
        target_guid,
        target_position,
        target_has_vehicle_kit,
    );

    session
}

fn represented_vehicle_kit_with_passenger_like_cpp(
    base_guid: ObjectGuid,
    passenger_guid: ObjectGuid,
    ejectable: bool,
) -> wow_entities::Vehicle {
    let mut vehicle = wow_entities::Vehicle::new(
        base_guid,
        TypeId::Player,
        Position::ZERO,
        77,
        0,
        [(
            0,
            wow_entities::VehicleSeatInfo {
                id: 1007,
                attachment_offset: Position::ZERO,
                can_enter_or_exit: true,
                usable_by_override: false,
                can_control: false,
                can_switch_from_seat: false,
                ejectable,
                disables_gravity: false,
                passenger_not_selectable: false,
                keep_pet: false,
            },
            wow_entities::VehicleSeatAddon::default(),
        )],
    );
    vehicle.install();
    assert!(vehicle.add_vehicle_passenger(passenger_guid, 0));
    vehicle
}

fn configure_self_resurrect_canonical_player_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    health: u32,
    max_health: u32,
) {
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "SelfRez".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(health, max_health);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_power_index(PowerType::Rage, Some(1));
            player
                .unit_mut()
                .set_power_index(PowerType::Energy, Some(3));
            player.unit_mut().set_power_index(PowerType::Focus, Some(4));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.unit_mut().set_max_power(PowerType::Rage, 100);
            player.unit_mut().set_power(PowerType::Rage, 50);
            player.unit_mut().set_max_power(PowerType::Energy, 100);
            player.unit_mut().set_power(PowerType::Energy, 30);
            player.unit_mut().set_max_power(PowerType::Focus, 100);
            player.unit_mut().set_power(PowerType::Focus, 40);
            player.clear_data_changes();
        })
        .unwrap();
}

fn stuck_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK,
            ..Default::default()
        }],
    }
}

fn hearthstone_spell_info_like_cpp() -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id: 8690,
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
        effects: Vec::new(),
    }
}

fn set_stuck_spell_store_like_cpp(session: &mut WorldSession, spell_id: i32) {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, stuck_spell_info_like_cpp(spell_id));
    spell_store.insert(8690, hearthstone_spell_info_like_cpp());
    session.set_spell_store(Arc::new(spell_store));
}

fn threat_spell_info_like_cpp(spell_id: i32, effect: u32, damage: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            ..Default::default()
        }],
    }
}

fn cooldown_or_charges_spell_info_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    misc_value: i32,
    trigger_spell: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            effect_misc_value_1: misc_value,
            effect_trigger_spell: trigger_spell,
            ..Default::default()
        }],
    }
}

fn power_spell_info_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    power_type: PowerType,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            effect_misc_value_1: power_type as i32,
            ..Default::default()
        }],
    }
}

fn represented_hunter_pet_stable_like_cpp(pet_number: u32, creature_id: u32) -> PetStable {
    PetStable {
        current_pet_index: Some(0),
        active_pets: vec![Some(PetStableInfo {
            name: "Misha".to_string(),
            pet_number,
            creature_id,
            display_id: 12_345,
            health: 345,
            mana: 67,
            created_by_spell_id: 9_001,
            specialization_id: 2,
            level: 80,
            react_state: wow_entities::ReactState::Defensive,
            pet_type: wow_entities::PetType::Hunter,
            ..PetStableInfo::default()
        })],
        stabled_pets: Vec::new(),
        unslotted_pets: Vec::new(),
    }
}

fn character_pet_stable_row_like_cpp(
    pet_number: u32,
    slot: i16,
    pet_type: u8,
) -> CharacterPetStableRowLikeCpp {
    CharacterPetStableRowLikeCpp {
        pet_number,
        creature_id: 500 + pet_number,
        display_id: 12_000 + pet_number,
        level: 70,
        experience: 123_456,
        react_state: 1,
        slot,
        name: format!("Pet{pet_number}"),
        was_renamed: true,
        health: 345,
        mana: 67,
        action_bar: "1 2 3".to_string(),
        last_save_time: 98_765,
        created_by_spell_id: 9_001,
        pet_type,
        specialization_id: 2,
    }
}

fn first_login_cast_spell_store_like_cpp(
    normal_spell: u32,
    npe_spell: u32,
) -> PlayerCreateInfoCastSpellStoreLikeCpp {
    PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
        wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: normal_spell,
            create_mode: wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP as i8,
        },
        wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: npe_spell,
            create_mode: wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP as i8,
        },
    ])
}

fn player_create_custom_spell_store_like_cpp() -> PlayerCreateInfoCustomSpellStoreLikeCpp {
    PlayerCreateInfoCustomSpellStoreLikeCpp::from_rows_like_cpp([
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: 80_001,
        },
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: 80_002,
        },
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 2,
            class_mask: 1,
            spell_id: 80_003,
        },
    ])
}

fn first_login_noop_spell_store_like_cpp(spell_ids: impl IntoIterator<Item = i32>) -> SpellStore {
    let mut store = SpellStore::new();
    for spell_id in spell_ids {
        store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
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
                effects: Vec::new(),
            },
        );
    }
    store
}

fn first_login_reputation_faction_store_like_cpp() -> FactionStore {
    let mut entries = Vec::new();
    for (rep_index, faction_id) in FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP
        .iter()
        .chain(FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP.iter())
        .chain(FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP.iter())
        .enumerate()
    {
        let mut entry = FactionEntry::for_test_like_cpp(*faction_id, rep_index as i16);
        entry.reputation_race_mask[0] = 1;
        entry.reputation_max[0] = wow_data::reputation::REPUTATION_CAP_LIKE_CPP;
        entries.push(entry);
    }
    FactionStore::from_entries(entries)
}

#[path = "session/effect_learning_tests.rs"]
mod effect_learning_tests;

fn install_xp_victim_like_cpp(session: &mut WorldSession, creature_guid: ObjectGuid, tapped: bool) {
    let player_guid = session
        .player_guid()
        .unwrap_or_else(|| ObjectGuid::create_player(1, 0xE1C0));
    session.set_player_guid(Some(player_guid));
    let map_id = session.player_map_id_like_cpp();
    session.set_map_manager(shared_map_manager());
    if session.player_position_like_cpp().is_none() {
        session.set_player_map_position_like_cpp(map_id, Position::new(10.0, 10.0, 0.0, 0.0));
    }
    session.register_world_creature(
        map_id,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(creature_guid, 9_001, 100),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    if tapped {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.set_tapped_by_player(player_guid, &[]);
            })
            .expect("XP victim is installed in the represented map");
    }
}

fn install_tapped_xp_victim_like_cpp(session: &mut WorldSession, creature_guid: ObjectGuid) {
    install_xp_victim_like_cpp(session, creature_guid, true);
}

fn test_db2_area_trigger_like_cpp(
    trigger_id: u32,
    map_id: u16,
    pos: Position,
) -> wow_data::AreaTriggerDb2Entry {
    wow_data::AreaTriggerDb2Entry {
        id: trigger_id,
        message: String::new(),
        pos: wow_data::Db2Position3 {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        continent_id: map_id as i16,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        radius: 5.0,
        box_length: 0.0,
        box_width: 0.0,
        box_height: 0.0,
        box_yaw: 0.0,
        shape_type: 0,
        shape_id: 0,
        area_trigger_action_set_id: 0,
        flags: 0,
    }
}

fn test_talent_entry_like_cpp(id: u32, rank: u8, spell_id: i32) -> wow_data::TalentEntry {
    let mut spell_rank = [0; 9];
    spell_rank[usize::from(rank)] = spell_id;
    wow_data::TalentEntry {
        id,
        description: String::new(),
        tier_id: 0,
        flags: 0,
        column_index: 0,
        tab_id: 0,
        class_id: 0,
        spec_id: 0,
        spell_id,
        overrides_spell_id: 0,
        required_spell_id: 0,
        category_mask: [0; 2],
        spell_rank,
        prereq_talent: [0; 3],
        prereq_rank: [0; 3],
    }
}

fn test_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: Vec::new(),
    }
}

fn test_visible_aura(slot: u8, spell_id: i32) -> AuraApplication {
    AuraApplication {
        spell_id,
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x1,
        effect_mask: 0x1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

fn install_test_talent_tab_store_like_cpp(session: &mut WorldSession) -> wow_data::TalentTabStore {
    let talent_tabs = wow_data::TalentTabStore::from_entries([wow_data::TalentTabEntry {
        id: 0,
        name: String::new(),
        background_file: String::new(),
        order_index: 0,
        race_mask: 0,
        class_mask: 1,
        pet_talent_mask: 0,
        spell_icon_id: 0,
    }]);
    session.set_player_class_like_cpp(1);
    talent_tabs
}

fn cuf_profile_for_save_test(name: &str, height: u16) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: name.to_string(),
        frame_height: height,
        frame_width: 120,
        sort_by: 1,
        health_text: 2,
        top_point: 3,
        bottom_point: 4,
        left_point: 5,
        top_offset: 6,
        bottom_offset: 7,
        left_offset: 8,
        bool_options: 0b10101,
    }
}

fn currency_entry(id: u32) -> wow_data::CurrencyTypesEntry {
    wow_data::CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}

fn install_transmog_can_add_test_item(
    session: &mut WorldSession,
    item_id: u32,
    class_id: ItemClass,
    subclass_id: u8,
    inventory_type: InventoryType,
    quality: ItemQuality,
    flags: [u32; 4],
    allowable_class: i16,
) {
    install_transmog_can_add_test_items(
        session,
        [(
            item_id,
            class_id,
            subclass_id,
            inventory_type,
            quality,
            flags,
            allowable_class,
        )],
    );
}

fn install_transmog_can_add_test_items<const N: usize>(
    session: &mut WorldSession,
    items: [(
        u32,
        ItemClass,
        u8,
        InventoryType,
        ItemQuality,
        [u32; 4],
        i16,
    ); N],
) {
    session.set_item_store(Arc::new(ItemStore::from_records(
        items.iter().copied().map(
            |(item_id, class_id, subclass_id, inventory_type, _, _, _)| ItemRecord {
                id: item_id,
                class_id: class_id as u8,
                subclass_id,
                material: 0,
                inventory_type: inventory_type as i8,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
                scaling_stat_distribution_id: 0,
                scaling_stat_value: 0,
            },
        ),
    )));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries(
        items
            .iter()
            .copied()
            .map(
                |(item_id, _, _, _, quality, flags, allowable_class)| ItemSearchNameEntry {
                    id: item_id,
                    allowable_race: 0,
                    display: String::new(),
                    overall_quality_id: quality as u8,
                    expansion_id: 0,
                    min_faction_id: 0,
                    min_reputation: 0,
                    allowable_class: i32::from(allowable_class),
                    required_level: 0,
                    required_skill: 0,
                    required_skill_rank: 0,
                    required_ability: 0,
                    item_level: 1,
                    flags: flags.map(|flag| flag as i32),
                },
            ),
    )));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            items.iter().copied().map(
                |(item_id, _, _, inventory_type, _, flags, allowable_class)| {
                    (
                        item_id,
                        ItemSparseTemplateEntry {
                            flags,
                            bag_family: 0,
                            start_quest_id: 0,
                            stackable: 1,
                            max_count: 0,
                            lock_id: 0,
                            required_reputation_rank: 0,
                            sell_price: 0,
                            buy_price: 0,
                            vendor_stack_count: 1,
                            price_variance: 0.0,
                            price_random_value: 0.0,
                            max_durability: 0,
                            other_faction_item_id: 0,
                            content_tuning_id: 0,
                            player_level_to_item_level_curve_id: 0,
                            limit_category: 0,
                            instance_bound: 0,
                            zone_bound: [0, 0],
                            required_reputation_faction: 0,
                            allowable_class,
                            required_expansion: 0,
                            bonding: ItemBondingType::None as u8,
                            container_slots: 0,
                            inventory_type: inventory_type as i8,
                        },
                    )
                },
            ),
            items
                .iter()
                .copied()
                .map(|(item_id, _, _, inventory_type, quality, _, _)| {
                    (
                        item_id,
                        ItemRandomPropertyTemplateEntry {
                            item_level: 1,
                            quality: quality as i8,
                            inventory_type: inventory_type as i8,
                        },
                    )
                }),
        ),
    ));
}

fn install_represented_battle_pet_stat_stores_like_cpp(session: &mut WorldSession) {
    session.set_battle_pet_breed_state_store(Arc::new(BattlePetBreedStateStore::from_entries([
        wow_data::BattlePetBreedStateEntry {
            id: 1,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
            value: 500,
            battle_pet_breed_id: 7,
        },
        wow_data::BattlePetBreedStateEntry {
            id: 2,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
            value: 300,
            battle_pet_breed_id: 7,
        },
        wow_data::BattlePetBreedStateEntry {
            id: 3,
            battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
            value: 200,
            battle_pet_breed_id: 7,
        },
    ])));
    session.set_battle_pet_species_state_store(Arc::new(BattlePetSpeciesStateStore::from_entries(
        [
            wow_data::BattlePetSpeciesStateEntry {
                id: 10,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: 11,
            },
            wow_data::BattlePetSpeciesStateEntry {
                id: 11,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                value: 50,
                battle_pet_species_id: 11,
            },
            wow_data::BattlePetSpeciesStateEntry {
                id: 12,
                battle_pet_state_id: wow_data::BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                value: 25,
                battle_pet_species_id: 11,
            },
        ],
    )));
    session.set_battle_pet_breed_quality_store(Arc::new(BattlePetBreedQualityStore::from_entries(
        [wow_data::BattlePetBreedQualityEntry {
            id: 20,
            state_multiplier: 1.5,
            quality_enum: 3,
        }],
    )));
}

fn install_represented_battle_pet_species_flags_like_cpp(
    session: &mut WorldSession,
    species: u32,
    flags: i32,
) {
    install_represented_battle_pet_species_like_cpp(session, species, 0, flags);
}

fn install_represented_battle_pet_species_like_cpp(
    session: &mut WorldSession,
    species: u32,
    creature_id: i32,
    flags: i32,
) {
    session.set_battle_pet_species_store(Arc::new(wow_data::BattlePetSpeciesStore::from_entries(
        [wow_data::BattlePetSpeciesEntry {
            id: species,
            description: String::new(),
            source_text: String::new(),
            creature_id,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        }],
    )));
}

fn write_minimal_use_toy_packet_like_cpp(
    item_id: u32,
    spell_id: i32,
    cast_id: ObjectGuid,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::UseToy as u16);
    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(i32::try_from(item_id).unwrap());
    pkt.write_int32(0);
    pkt.write_int32(spell_id);
    wow_packet::packets::spell::SpellCastVisual::default().write(&mut pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(false);
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(&mut pkt);
    pkt
}

fn instant_toy_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
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
        effects: Vec::new(),
    }
}

fn insert_open_item_bag_with_child(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    bag_slot: u8,
    inner_slot: u8,
) -> (ObjectGuid, ObjectGuid) {
    let bag_guid = ObjectGuid::create_item(1, 1001);
    session.inventory_items.insert(
        bag_slot,
        InventoryItem {
            guid: bag_guid,
            entry_id: 101,
            db_guid: 1001,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        101,
        player_guid,
        1,
        0,
        ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);

    let child_guid = ObjectGuid::create_item(1, 1002);
    let mut child = session.make_inventory_item_object(
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        inner_slot,
    );
    child.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child);

    (bag_guid, child_guid)
}

fn install_open_item_has_loot_template(session: &mut WorldSession, entry: u32) {
    install_open_item_has_loot_template_with_lock(session, entry, 0);
}

fn install_open_item_template_with_flags(
    session: &mut WorldSession,
    entry: u32,
    flags: ItemFlags,
    lock_id: u16,
) {
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
            flags: [flags.bits() as u32, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 1.0,
            price_random_value: 1.0,
            max_durability: 0,
            other_faction_item_id: 0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            limit_category: 0,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn install_open_item_has_loot_template_with_lock(
    session: &mut WorldSession,
    entry: u32,
    lock_id: u16,
) {
    install_open_item_template_with_flags(session, entry, ItemFlags::HAS_LOOT, lock_id);
}

fn install_lock_store(session: &mut WorldSession, lock_id: u32) {
    session.set_lock_store(Arc::new(LockStore::from_entries([LockEntry {
        id: lock_id,
        index: [0; 8],
        skill: [0; 8],
        lock_type: [0; 8],
        action: [0; 8],
    }])));
}

fn insert_open_item_top_level(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    item_guid: ObjectGuid,
    entry: u32,
    unlocked: bool,
) {
    session.inventory_items.insert(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: entry,
            db_guid: item_guid.counter() as u64,
            inventory_type: None,
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if unlocked {
        item.set_item_flag(ItemFieldFlags::UNLOCKED);
    }
    session.insert_inventory_item_object(item);
}

async fn assert_open_item_nested_has_loot_opens_without_internal_bag_error(bag_slot: u8) {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    install_open_item_has_loot_template(&mut session, 700);
    let (_, child_guid) = insert_open_item_bag_with_child(&mut session, player_guid, bag_slot, 5);

    session
        .handle_open_item(WorldPacket::from_bytes(&[bag_slot, 5]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::LootResponse as u16);
    assert_ne!(opcode, ServerOpcodes::InventoryChangeFailure as u16);
    assert!(session.loot_table.contains_key(&child_guid));
    assert!(
        session
            .inventory_item_objects
            .get(&child_guid)
            .is_some_and(|item| item.loot_generated())
    );
}

fn assert_open_item_release_destroy_nested_item_leaves_container_in_place(bag_slot: u8) {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, bag_slot, 5);
    let child = session.inventory_item_objects.get(&child_guid).unwrap();
    let child_bag = child.bag_slot();
    let child_slot = child.slot();

    let inv = session.get_inventory_item_by_pos(child_bag, child_slot);
    assert!(inv.is_some());
    assert_eq!(inv.unwrap().guid, child_guid);

    session.remove_fully_looted_runtime_item(child_bag, child_slot, child_guid);
    assert!(
        session
            .get_inventory_item_by_pos(child_bag, child_slot)
            .is_none()
    );
    assert!(!session.inventory_item_objects.contains_key(&child_guid));
    assert!(session.inventory_items.contains_key(&bag_slot));
    assert_eq!(session.inventory_items[&bag_slot].guid, bag_guid);
}

fn run_canonical_player_owner_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("canonical-player-owner".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

fn insert_session_player_into_canonical_map_like_cpp(
    session: &WorldSession,
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
) {
    if let Some(handle) = session.player_handle_like_cpp {
        let position = session
            .player_position_like_cpp()
            .expect("canonical Player fixture position");
        let key = wow_map::MapKey::new(map_id, instance_id);
        let mut manager = canonical.lock().unwrap();
        manager.create_world_map(map_id, instance_id);
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
                .attach_player_like_cpp(handle, key, position)
                .expect("attach detached canonical Player fixture"),
            Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => {}
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager
                    .detach_player_like_cpp(handle)
                    .expect("detach canonical Player fixture");
                manager
                    .attach_player_like_cpp(handle, key, position)
                    .expect("reattach canonical Player fixture");
            }
            None => panic!("stale canonical Player fixture handle"),
        }
        return;
    }

    let player = session
        .build_initial_player_for_owner_like_cpp(wow_map::MapKey::new(map_id, instance_id), None)
        .expect("complete canonical player fixture");
    let record =
        wow_entities::MapObjectRecord::new_player(player).expect("canonical Player fixture record");
    let mut manager = canonical.lock().unwrap();
    manager
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(record)
        .expect("insert canonical Player fixture");
}

fn session_with_canonical_player_for_away_like_cpp()
-> (WorldSession, SharedCanonicalMapManager, ObjectGuid) {
    let (session, _, canonical, player_guid) =
        session_with_canonical_player_for_away_like_cpp_with_packet_tx();
    (session, canonical, player_guid)
}

fn session_with_canonical_player_for_away_like_cpp_with_packet_tx() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    SharedCanonicalMapManager,
    ObjectGuid,
) {
    let (mut session, pkt_tx, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xAFD0);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "AwayTester".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    (session, pkt_tx, canonical, player_guid)
}

fn set_canonical_player_farsight_object_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    farsight_object: ObjectGuid,
) {
    set_canonical_player_farsight_object_on_map_like_cpp(
        canonical,
        player_guid,
        farsight_object,
        571,
        0,
    );
}

fn set_canonical_player_farsight_object_on_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    farsight_object: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(map_id, instance_id)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .set_farsight_object_like_cpp(farsight_object);
}

fn faction_template_entry(
    id: u32,
    faction: u16,
    faction_group: u8,
    friend_group: u8,
    enemy: u16,
) -> wow_data::progression_rewards::FactionTemplateEntry {
    let mut enemies = [0; 8];
    enemies[0] = enemy;
    wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group,
        friend_group,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    }
}

fn represented_get_reaction_input_like_cpp() -> RepresentedGetReactionInputLikeCpp {
    RepresentedGetReactionInputLikeCpp {
        self_faction_template_id: 1,
        target_faction_template_id: 2,
        same_object: false,
        attackable_by_summoner: false,
        same_charmer_or_owner_or_self: false,
        self_has_player_owner: true,
        target_has_player_owner: true,
        target_player_owner_is_current_session: true,
        target_owner_forced_rank_for_self: None,
        same_player_owner: false,
        duel_in_progress: false,
        same_raid: false,
        self_unit_player_controlled: true,
        target_unit_player_controlled: true,
        self_ffa_pvp: false,
        target_ffa_pvp: false,
        self_ignores_reputation: false,
        target_ignores_reputation: false,
        target_is_unit: true,
        target_player_contested_pvp: false,
    }
}

fn quest_giver_complete_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
    from_script: bool,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    pkt.write_bit(from_script);
    pkt.flush_bits();
    pkt
}

fn quest_giver_request_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    pkt
}

fn quest_giver_choose_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
    choice_item_id: u32,
    loot_item_type: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    // C++ QuestChoiceItem: 2-bit LootItemType, ItemInstance, int32 Quantity.
    pkt.write_bits(loot_item_type, 2);
    pkt.write_int32(choice_item_id as i32);
    pkt.write_int32(0); // RandomPropertiesSeed
    pkt.write_int32(0); // RandomPropertiesID
    pkt.write_bit(false); // ItemBonus.has_value()
    pkt.flush_bits();
    pkt.write_bits(0, 6); // ItemModList.Values.size()
    pkt.flush_bits();
    pkt.write_int32(if choice_item_id == 0 { 0 } else { 1 });
    pkt
}

// ── RuntimeTickOwner / RuntimeOutput tests (#NEXT.RUNTIME.L3.001) ─────────

fn legacy_aggro_candidate_like_cpp(
    player_guid: ObjectGuid,
    position: Position,
) -> LegacyCreatureAggroCandidateLikeCpp {
    LegacyCreatureAggroCandidateLikeCpp {
        player_guid,
        map_id: 0,
        instance_id: 0,
        map_difficulty_id: 0,
        position,
        player_visibility_represented: true,
        player_phase_shift: PhaseShift::default(),
        player_visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
        player_combat_reach: 0.0,
        player_detected_range_aura_mod: 0.0,
        player_level: 80,
        player_gray_level: 70,
        player_liquid_status_like_cpp: 0,
        player_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        player_unit_flags2: 0,
        player_unit_state: 0,
        player_is_game_master: false,
        player_is_contested_pvp: false,
        player_faction_template_id: 1,
        player_reputation_standings: Vec::new(),
        player_reputation_state_flags: Vec::new(),
        player_forced_reputation_ranks: Vec::new(),
        player_forced_reputation_faction_ids: Vec::new(),
        player_school_immunity_mask: 0,
        player_damage_immunity_mask: 0,
        player_has_confuse_aura: false,
        player_has_breakable_stun_aura: false,
    }
}

fn legacy_aggro_relation_config_like_cpp(
    creature_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    player_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    creature_faction: FactionEntry,
) -> LegacyCreatureAggroConfigLikeCpp {
    LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                creature_faction_template,
                player_faction_template,
            ]),
        )),
        faction_store: Some(Arc::new(FactionStore::from_entries([creature_faction]))),
        ..Default::default()
    }
}

fn legacy_aggro_hostile_config_like_cpp() -> LegacyCreatureAggroConfigLikeCpp {
    legacy_aggro_relation_config_like_cpp(
        faction_template_entry(14, 72, 0, 0, 930),
        faction_template_entry(1, 930, 0, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    )
}

fn legacy_aggro_hostile_config_with_rate_like_cpp(
    creature_aggro_rate: f32,
) -> LegacyCreatureAggroConfigLikeCpp {
    LegacyCreatureAggroConfigLikeCpp {
        creature_aggro_rate,
        ..legacy_aggro_hostile_config_like_cpp()
    }
}

fn represented_creature_spell_test_attributes_like_cpp(no_attack_miss: bool) -> [u32; 15] {
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    if no_attack_miss {
        attributes[7] = 0x0200_0000; // SPELL_ATTR7_NO_ATTACK_MISS
    }
    attributes
}

fn spell_misc_entry_like_cpp(id: u32, spell_id: u32, range_index: u16) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id,
        attributes: represented_creature_spell_test_attributes_like_cpp(true)
            .map(|attribute| attribute as i32),
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index,
        school_mask: 0x01,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

fn spell_range_entry_like_cpp(
    id: u32,
    range_min: f32,
    range_max: f32,
) -> wow_data::SpellRangeEntry {
    wow_data::SpellRangeEntry {
        id,
        display_name: String::new(),
        display_name_short: String::new(),
        flags: 0,
        range_min: [range_min, range_min],
        range_max: [range_max, range_max],
    }
}

fn creature_ai_test_spell_info_like_cpp(
    spell_id: i32,
    implicit_target_1: u32,
    implicit_target_2: u32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        // SpellStore hydrates GetRecoveryTime=max(RecoveryTime,
        // CategoryRecoveryTime). CombatAI must still use raw 6s below.
        recovery_time_ms: 12_000,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
        effect_base_points: 7,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            implicit_target_1,
            implicit_target_2,
            ..Default::default()
        }],
    }
}

fn creature_ai_spell_test_config_like_cpp(
    spell: wow_data::SpellInfo,
    passive: bool,
    range_max: f32,
) -> LegacyCreatureAggroConfigLikeCpp {
    let spell_id = u32::try_from(spell.spell_id).unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell.spell_id, spell);
    let mut misc = spell_misc_entry_like_cpp(8_001, spell_id, 71);
    if passive {
        misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    }
    spell_store.insert_spell_misc_attributes_like_cpp(
        i32::try_from(spell_id).unwrap(),
        misc.attributes.map(|attribute| attribute as u32),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        i32::try_from(spell_id).unwrap(),
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    LegacyCreatureAggroConfigLikeCpp {
        spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc]))),
        spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 0.0, range_max),
        ]))),
        spell_cooldowns_store: Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
            wow_data::SpellCooldownsEntry {
                id: 8_002,
                difficulty_id: 0,
                category_recovery_time: 12_000,
                recovery_time: 6_000,
                start_recovery_time: 0,
                spell_id,
            },
        ]))),
        spell_category_store: Some(Arc::new(SpellCategoryStore::from_entries([]))),
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: 8_003,
                difficulty_id: 0,
                spell_visual_id: 99_999,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id,
            }],
        ))),
        spell_target_restrictions_store: Some(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([]),
        )),
        spell_casting_requirements_store: Some(Arc::new(
            wow_data::SpellCastingRequirementsStore::from_entries([]),
        )),
        spell_aura_restrictions_store: Some(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([]),
        )),
        spell_store: Some(Arc::new(spell_store)),
        spell_chain_store: Some(Arc::new(SpellChainStoreLikeCpp::default())),
        spell_linked_store: Some(Arc::new(SpellLinkedStoreLikeCpp::default())),
        spell_condition_store: Some(Arc::new(ConditionEntriesByTypeStore::default())),
        spell_script_exact_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_script_all_rank_root_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        legacy_spell_script_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_linked_rejected_trigger_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        ..legacy_aggro_hostile_config_like_cpp()
    }
}

fn creature_spell_bound_hook_tick_fixture_like_cpp(
    ai_name: &str,
    passive: bool,
    spell_id: i32,
    creature_counter: i64,
    victim_counter: i64,
) -> (
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
    LegacyCreatureAggroConfigLikeCpp,
    ObjectGuid,
) {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(creature_counter);
    let victim_guid = ObjectGuid::create_player(1, victim_counter);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp(ai_name, String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        passive,
        30.0,
    );
    config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([u32::try_from(spell_id).unwrap()])));
    (manager, canonical, config, creature_guid)
}

fn add_canonical_creature_spell_test_pair_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
) {
    add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
        canonical,
        creature_guid,
        victim_guid,
        0,
    );
}

fn add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    difficulty_id: u8,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        victim_guid,
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
        0,
        difficulty_id,
    );
    add_canonical_test_creature_on_map(
        canonical,
        creature_guid,
        9_001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    let mut manager = canonical.lock().unwrap();
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    let creature = map.get_typed_creature_mut(creature_guid).unwrap();
    creature.unit_mut().set_faction(14);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    let player = map.get_typed_player_mut(victim_guid).unwrap();
    player.unit_mut().set_faction(1);
    player.unit_mut().set_level(80);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Alive);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
}

#[derive(Debug, PartialEq, Eq)]
struct CreatureSpellWireHeaderLikeCpp {
    opcode: u16,
    caster: ObjectGuid,
    caster_unit: ObjectGuid,
    cast_id: ObjectGuid,
    original_cast_id: ObjectGuid,
    spell_id: i32,
    spell_x_spell_visual_id: u32,
    cast_flags: u32,
    cast_flags_ex: u32,
    cast_time_ms: u32,
    target_flags: u32,
    target_unit: ObjectGuid,
    hit_targets: Vec<ObjectGuid>,
    miss_targets: Vec<(ObjectGuid, u8)>,
}

fn decode_creature_spell_wire_header_and_tail_like_cpp(
    bytes: &[u8],
) -> (CreatureSpellWireHeaderLikeCpp, WorldPacket) {
    let mut packet = WorldPacket::from_bytes(bytes);
    let opcode = packet.opcode_raw();
    packet.skip_opcode();
    let caster = packet.read_packed_guid().unwrap();
    let caster_unit = packet.read_packed_guid().unwrap();
    let cast_id = packet.read_packed_guid().unwrap();
    let original_cast_id = packet.read_packed_guid().unwrap();
    let spell_id = packet.read_int32().unwrap();
    let spell_x_spell_visual_id = packet.read_uint32().unwrap();
    let cast_flags = packet.read_uint32().unwrap();
    let cast_flags_ex = packet.read_uint32().unwrap();
    let cast_time_ms = packet.read_uint32().unwrap();
    assert_eq!(packet.read_int32().unwrap(), 0); // missile travel time
    assert_eq!(packet.read_float().unwrap(), 0.0); // missile pitch
    assert_eq!(packet.read_uint8().unwrap(), 0); // destination cast index
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity school
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity value
    assert_eq!(packet.read_uint32().unwrap(), 0); // heal prediction
    assert_eq!(packet.read_uint8().unwrap(), 0); // prediction type
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    let hit_count = packet.read_bits(16).unwrap() as usize;
    let miss_count = packet.read_bits(16).unwrap() as usize;
    assert_eq!(packet.read_bits(16).unwrap() as usize, miss_count);
    assert_eq!(packet.read_bits(9).unwrap(), 0); // remaining power
    assert!(!packet.read_bit().unwrap()); // remaining runes
    assert_eq!(packet.read_bits(16).unwrap(), 0); // target points
    assert!(!packet.read_bit().unwrap()); // ammo display
    assert!(!packet.read_bit().unwrap()); // ammo inventory type
    let target = wow_packet::packets::spell::SpellTargetData::read(&mut packet).unwrap();
    let hit_targets = (0..hit_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_guids: Vec<_> = (0..miss_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_targets = miss_guids
        .into_iter()
        .map(|guid| {
            let reason = packet.read_uint8().unwrap();
            if reason == wow_packet::packets::spell::SpellMissReason::Reflect as u8 {
                let _reflect_status = packet.read_uint8().unwrap();
            }
            (guid, reason)
        })
        .collect();
    let header = CreatureSpellWireHeaderLikeCpp {
        opcode,
        caster,
        caster_unit,
        cast_id,
        original_cast_id,
        spell_id,
        spell_x_spell_visual_id,
        cast_flags,
        cast_flags_ex,
        cast_time_ms,
        target_flags: target.flags,
        target_unit: target.unit,
        hit_targets,
        miss_targets,
    };
    (header, packet)
}

fn decode_creature_spell_wire_header_like_cpp(bytes: &[u8]) -> CreatureSpellWireHeaderLikeCpp {
    decode_creature_spell_wire_header_and_tail_like_cpp(bytes).0
}

fn decode_creature_spell_full_log_like_cpp(
    bytes: &[u8],
) -> wow_packet::packets::spell::SpellCastLogData {
    use wow_packet::packets::spell::{SpellCastLogData, SpellLogPowerData};

    let (_, mut packet) = decode_creature_spell_wire_header_and_tail_like_cpp(bytes);
    assert!(packet.read_bit().expect("full combat-log presence bit"));
    let health = packet.read_int64().expect("combat-log health");
    let attack_power = packet.read_int32().expect("combat-log attack power");
    let spell_power = packet.read_int32().expect("combat-log spell power");
    let armor = packet.read_int32().expect("combat-log armor");
    let power_count = packet.read_bits(9).expect("combat-log power count") as usize;
    let power_data = (0..power_count)
        .map(|_| SpellLogPowerData {
            power_type: packet.read_int32().expect("combat-log power type"),
            amount: packet.read_int32().expect("combat-log power amount"),
            cost: packet.read_int32().expect("combat-log power cost"),
        })
        .collect();
    assert!(packet.is_empty());
    SpellCastLogData {
        health,
        attack_power,
        spell_power,
        armor,
        power_data,
    }
}

fn decode_atomic_creature_spell_wire_pair_like_cpp(
    event: &RuntimeEvent,
) -> (
    CreatureSpellWireHeaderLikeCpp,
    CreatureSpellWireHeaderLikeCpp,
) {
    let basic_go_packet_bytes = match &event.recipients {
        crate::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
            basic_go_packet_bytes,
            ..
        } => basic_go_packet_bytes,
        _ => panic!("creature spell START/GO must use the atomic cast recipient rule"),
    };
    (
        decode_creature_spell_wire_header_like_cpp(&event.packet_bytes),
        decode_creature_spell_wire_header_like_cpp(basic_go_packet_bytes),
    )
}

fn creature_melee_sync_state_for_test_like_cpp(
    victim: &crate::map_manager::WorldCreature,
    applied_damage: u32,
) -> CreatureMeleeVictimSyncStateLikeCpp {
    let mut canonical = victim.clone();
    let identity_authority = canonical.creature.loot_authority_like_cpp().clone();
    let identity_health_authority = canonical
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let health_before = canonical.creature.unit().data().health;
    let revision_before = canonical.creature.unit().health_state_revision_like_cpp();
    let lifecycle_before = canonical.creature.loot_lifecycle_revision_like_cpp();
    let death_before = canonical.creature.unit().death_state();
    let ai_before = canonical.creature.ai_ownership().state;
    let killed = canonical.take_damage_before_death_state_at_game_time_like_cpp(
        applied_damage,
        wow_entities::game_time_secs_like_cpp(),
    );
    if killed {
        canonical.complete_death_state_after_kill_hooks_like_cpp();
    }
    CreatureMeleeVictimSyncStateLikeCpp {
        applied_damage,
        victim_health_before: health_before,
        victim_health_after: canonical.creature.unit().data().health,
        victim_health_state_revision_before: revision_before,
        victim_health_state_revision_after: canonical
            .creature
            .unit()
            .health_state_revision_like_cpp(),
        identity: CreatureMeleeVictimSyncIdentityLikeCpp {
            authority: identity_authority,
            health_state_revision_authority: identity_health_authority,
            spawn_id: canonical.creature.spawn_id(),
            loot_lifecycle_revision_before: lifecycle_before,
            loot_lifecycle_revision_after: canonical.creature.loot_lifecycle_revision_like_cpp(),
            death_state_before: death_before,
            death_state_after: canonical.creature.unit().death_state(),
            ai_state_before: ai_before,
            ai_state_after: canonical.creature.ai_ownership().state,
        },
    }
}

struct CreatureMeleeLosTestEnvironment {
    los: bool,
}

impl wow_entities::WorldObjectEnvironment for CreatureMeleeLosTestEnvironment {
    fn map_id(&self) -> u32 {
        0
    }

    fn instance_id(&self) -> u32 {
        0
    }

    fn visibility_range(&self) -> f32 {
        100.0
    }

    fn line_of_sight(&self, _query: wow_entities::LineOfSightQuery<'_>) -> bool {
        self.los
    }

    fn map_height(
        &self,
        _object: &wow_entities::WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: wow_entities::WorldObjectHeightQuery,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }

    fn floor_z(
        &self,
        _object: &wow_entities::WorldObject,
        _position: Position,
        _max_search_dist: f32,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }
}

fn melee_los_test_world_object(
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: wow_constants::TypeMask,
    position: Position,
) -> wow_entities::WorldObject {
    let mut object = wow_entities::WorldObject::new(true, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(0, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

fn lifecycle_test_map_store_like_cpp(
    map_id: u32,
    instance_type: i8,
    flags1: u32,
) -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1,
        flags2: 0,
    }])
}

fn assert_instanceable_map_does_not_persist_respawn_like_cpp(
    map_id: u16,
    instance_id: u32,
    instance_type: i8,
    flags1: u32,
    managed_kind: wow_map::ManagedMapKind,
    spawn_id: u64,
) {
    use crate::map_manager::{RuntimeTickOwner, world_to_grid_coords};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical
        .lock()
        .unwrap()
        .create_map_entry(u32::from(map_id), instance_id, 0, managed_kind);

    let guid = test_creature_guid(
        i64::try_from(spawn_id).expect("test spawn id must fit the ObjectGuid counter"),
    );
    let mut creature = crate::map_manager::WorldCreature::new(
        guid,
        9005,
        Position::new(17.0, 18.0, 19.0, 1.0),
        50,
        7,
        8,
        12,
        20.0,
        104,
        14,
        0,
        0,
    );
    creature.creature.set_spawn_id(spawn_id);
    assert!(creature.take_damage(50));
    creature.set_corpse_despawn_at(Some(Instant::now() - Duration::from_secs(1)));
    let (grid_x, grid_y) = world_to_grid_coords(creature.position().x, creature.position().y);
    {
        let mut guard = manager.write().unwrap();
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        assert!(guard.add_creature(map_id, instance_id, grid_x, grid_y, creature));
    }

    let outcome = run_legacy_creature_lifecycle_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &lifecycle_test_map_store_like_cpp(u32::from(map_id), instance_type, flags1),
        Instant::now(),
    );

    assert_eq!(outcome.corpses_despawned, 1);
    assert!(
        outcome.respawn_db_mutations.is_empty(),
        "C++ Map::SaveRespawnInfoDB returns for Instanceable maps"
    );
    assert_eq!(
        manager.read().unwrap().persisted_respawn_time_like_cpp(
            map_id,
            instance_id,
            wow_map::SpawnObjectType::Creature,
            spawn_id,
        ),
        None
    );
}

// ── Slice 4A.2b — respawn queue ownership migrated to MapInstance ──────

/// Prepare a dead creature whose corpse timer has already elapsed.
/// Returns the session and the creature GUID so the caller can drive
/// `run_creatures_tick` to trigger the despawn-then-respawn path.
fn setup_dead_creature_past_despawn(
    guid_counter: i64,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(guid_counter);
    register_test_creature(&mut session, manager.clone(), guid, 10);

    // Kill the creature and set corpse_despawn_at to the past so that
    // the very next run_creatures_tick triggers despawn.
    let past = Instant::now() - Duration::from_secs(1);
    session
        .mutate_world_creature(guid, |c| {
            c.take_damage(10);
            c.set_corpse_despawn_at(Some(past));
        })
        .unwrap();

    // Mark the creature as client-visible so the DESTROY packet is built.
    session.client_visible_guids_like_cpp.insert(guid);

    (session, send_rx, guid)
}

/// After despawn, force the pending respawn entry to be immediately ready
/// by rewriting its `respawn_at` to the past via the map's queue.
fn force_respawn_ready(session: &mut WorldSession) {
    let map_id = session.player_map_id_like_cpp();
    let past = Instant::now() - Duration::from_secs(1);
    // Drain whatever is in the queue, rewrite respawn_at, push back.
    if let Some(manager) = &session.map_manager {
        let mut mgr = manager.write().unwrap_or_else(|p| p.into_inner());
        let entries =
            mgr.drain_ready_respawns(map_id, 0, Instant::now() + Duration::from_secs(9999));
        for mut r in entries {
            r.respawn_at = past;
            mgr.push_respawn(map_id, 0, r);
        }
    }
}

// ── step_creature_movement_like_cpp unit tests (no WorldSession) ──────────

fn make_test_world_creature(guid: ObjectGuid) -> crate::map_manager::WorldCreature {
    crate::map_manager::WorldCreature::new(
        guid,
        9999,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    )
}
