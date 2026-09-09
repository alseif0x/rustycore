use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rand::{Rng, SeedableRng, rngs::StdRng};

use wow_constants::{
    BagFamilyMask, DeathState, ItemContext, ItemFieldFlags, ItemFlags, ItemUpdateState, PowerType,
    ServerOpcodes, SpellCastResult,
};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_entities::{
    AppliedAuraRef, Creature, Item, ItemCreateInfo, MAX_ITEM_SPELLS, Pet, PetType, Player,
    UNIT_MASK_TOTEM,
};
use wow_loot::{
    LootConditionRowLikeCpp, condition_compare_values_like_cpp,
    loot_conditions_allow_player_like_cpp_representable,
};
use wow_packet::WorldPacket;
use wow_packet::packets::loot::{LootEntry, LootEntryFlags};
use wow_packet::packets::movement::MovementInfo;
use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};
use wow_persistence::{
    ItemTemplateAddonCatalogPersistencePortLikeCpp, ItemTemplateAddonCatalogRequestLikeCpp,
    ItemTemplateAddonLootMetadataOutcomeLikeCpp, ItemTemplateAddonMoneyOutcomeLikeCpp,
    PersistenceFutureLikeCpp,
};

use super::{
    ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP, ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP,
    ItemTemplateAddonLootMetadataLikeCpp, LOOT_MODE_DEFAULT_LIKE_CPP, LootTemplateRow,
    add_loot_item_stacks_like_cpp, add_loot_template_row_item_like_cpp,
    apply_wrapped_gift_transform_like_cpp, item_loot_quest_status_allows_like_cpp,
    loot_entry_flags_for_row_metadata_like_cpp, loot_template_group_row_can_roll_like_cpp,
    loot_template_plain_row_can_roll_like_cpp, loot_template_reference_row_can_roll_like_cpp,
    normalize_item_money_loot_bounds_like_cpp, player_class_mask_like_cpp,
    player_quest_status_mask_like_cpp, player_race_mask_like_cpp,
    referenced_loot_max_count_like_cpp, roll_chance_with_rate_like_cpp,
    roll_group_loot_row_like_cpp, stored_item_row_can_load_like_cpp_representable,
    stored_loot_item_should_persist_like_cpp,
};
use crate::session::{
    AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedPendingSpellCastRequestLikeCpp,
    SessionPlayerController, SharedCanonicalMapManager, SpellCastMetadata, SpellCastState,
};

mod fixtures;
mod port_fixtures;
mod spell_stores;
#[allow(unused_imports)]
use fixtures::*;
#[allow(unused_imports)]
use port_fixtures::*;
#[allow(unused_imports)]
use spell_stores::*;

mod aura;
mod cast;
mod loot_template;
mod misc;
mod mount;
mod stored_item;
mod totem;
