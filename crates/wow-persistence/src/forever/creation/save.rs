//! SQL-free durable character-creation input and result.

use crate::PersistenceFutureLikeCpp;

mod auras;
mod bank_tabs;
mod cuf_profiles;
mod currency;
mod equipment_sets;
mod garrison;
mod mail;
mod player_data;
mod statistics;
pub use auras::{NewAuraEffect, NewAuraSave};
pub use bank_tabs::NewBankTabSettings;
pub use cuf_profiles::NewCufProfile;
pub use currency::{NewCurrencySave, NewCurrencySaveState};
pub use equipment_sets::{NewSavedEquipmentSet, NewSavedSetKind, NewSavedSetState};
pub use garrison::{NewGarrisonBuilding, NewGarrisonFollower, NewGarrisonSave};
pub use mail::{NewMailSave, NewMailSaveEntry};
pub use player_data::{NewPlayerDataSave, NewPlayerDataValue};
pub use statistics::NewCharacterStatistics;
mod progress;
mod quests;
pub use progress::{
    NewAchievementSave, NewCompletedAchievement, NewCriteriaProgress, NewQuestCriteriaSave,
    NewReputation,
};
mod session;
mod specializations;
pub use session::{NewCharacterSessionSave, NewInstanceRestriction, NewTutorialSave};
mod spell_history;
pub use spell_history::{
    NewSpellCharge, NewSpellChargeCategory, NewSpellCooldown, NewSpellHistory,
};
mod traits;
mod transmog_outfits;
pub use transmog_outfits::{NewTransmogOutfit, NewTransmogOutfitSituation, NewTransmogOutfitSlot};
mod stored_locations;
pub use quests::{
    NewDailyQuestSave, NewQuestChange, NewQuestObjective, NewQuestSave, NewQuestSpawnTracking,
    NewQuestStatus, NewRewardedQuestChange, NewSeasonalQuest, NewSeasonalQuestGroup,
};
pub use specializations::{NewSpecializations, NewTalent};
pub use stored_locations::{
    NewStoredAuraLocation, NewStoredAuraLocationState, NewStoredAuraPosition,
};
pub use traits::{NewTraitChange, NewTraitConfig, NewTraitEntry, NewTraitState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreationIdentity {
    pub guid: u64,
    pub account: u32,
    pub realm: u32,
}

#[derive(Clone, Copy)]
pub struct CreationLimits {
    pub account: u32,
    pub realm: u32,
}

/// Admission before GUID generation, following CharacterHandler's order.
/// The normalized name is private and deliberately has no Debug implementation.
pub struct CreationAdmission {
    pub account: u32,
    pub realm: u32,
    pub name: String,
    pub limits: CreationLimits,
}

/// The constructor receives identity allocation, without database operations.
pub trait CreationIdentities: Send {
    fn identity(&self) -> CreationIdentity;
    fn next_item_guid(&mut self) -> Result<u64, CreateError>;
}

/// Owns the complete admitted creation and its writer exclusion. Dropping it
/// before save consumes only in-memory GUIDs. Save consumes the operation;
/// cancellation after durable work starts must retain the recovery fence.
pub trait CharacterCreation: CreationIdentities {
    fn save(
        self: Box<Self>,
        character: NewCharacter,
    ) -> PersistenceFutureLikeCpp<'static, Result<CreatedCharacter, CreateError>>;
}

#[derive(Clone, Copy)]
pub struct NewCustomization {
    pub option: u32,
    pub choice: u32,
}

#[derive(Clone, Copy)]
pub struct NewAction {
    pub spec: u8,
    pub trait_config: i32,
    pub button: u8,
    pub action: u64,
    pub kind: u8,
}

#[derive(Clone, Copy)]
pub struct NewSkill {
    pub skill: u16,
    pub value: u16,
    pub maximum: u16,
    pub profession_slot: i8,
}

#[derive(Clone, Copy)]
pub struct NewSpell {
    pub spell: u32,
    pub active: bool,
    pub disabled: bool,
    /// Player::_SaveSpells skips the spell row for dependent spells but still
    /// processes their favorite state.
    pub dependent: bool,
    pub favorite: bool,
}

#[derive(Clone, Copy)]
pub struct NewItemGem {
    pub item_id: u32,
    pub bonus_list_ids: [u16; 16],
    pub context: u8,
    pub scaling_level: u32,
}

#[derive(Clone, Copy)]
pub struct NewItemArtifactPower {
    pub id: u32,
    pub purchased_rank: u8,
}

#[derive(Clone)]
pub struct NewItemArtifact {
    pub xp: u64,
    pub appearance_id: u32,
    pub tier_id: u32,
    pub powers: Vec<NewItemArtifactPower>,
}

#[derive(Clone, Copy)]
pub struct NewItemModifiers {
    pub timewalker_level: u32,
    pub artifact_knowledge_level: u32,
}

#[derive(Clone)]
pub struct NewItemAzeriteSelectedEssences {
    pub specialization_id: u32,
    pub essence_ids: [u32; 4],
}

#[derive(Clone, Copy)]
pub struct NewItemAzeriteUnlockedEssence {
    pub id: u32,
    pub rank: u32,
}

#[derive(Clone)]
pub struct NewItemAzerite {
    pub xp: u64,
    pub level: u32,
    pub knowledge_level: u32,
    pub selected_essences: Vec<NewItemAzeriteSelectedEssences>,
    pub unlocked_milestones: Vec<u32>,
    pub unlocked_essences: Vec<NewItemAzeriteUnlockedEssence>,
}

/// Selected by the effective-template/DB2 construction path. The persistence
/// adapter does not infer subtype from the item entry.
#[derive(Clone)]
pub enum NewItemSubtype {
    Base,
    Azerite(NewItemAzerite),
    AzeriteEmpowered([i32; 5]),
}

/// Base Item::SaveToDB projection plus explicitly selected subtype data. This
/// does not imply support for every template-dependent item subtype.
#[derive(Clone)]
pub struct NewItem {
    pub guid: u64,
    pub entry: u32,
    pub count: u32,
    /// Index of the new container in NewCharacter::items, or root inventory.
    pub bag: Option<usize>,
    pub slot: u8,
    pub creator: NewItemCreator,
    pub gift_creator_guid: u64,
    pub expiration: u32,
    /// The charge prefix selected by Item::SaveToDB's effect-count bound,
    /// preserving signed charges and source effect order.
    pub charges: Vec<i32>,
    pub dynamic_flags: u32,
    /// ItemDefines.h::MAX_ENCHANTMENT_SLOT is 13. The constructor applies
    /// enchantment metadata and excludes DoNotSaveToDB entries here.
    pub enchantments: [Option<NewItemEnchantment>; 13],
    pub random_bonus_list: u32,
    pub durability: u16,
    pub played_time: u32,
    pub create_time: i64,
    pub text: String,
    pub battle_pet_modifiers: [u32; 4],
    pub context: u8,
    pub bonus_lists: Vec<i32>,
    pub gems: Vec<NewItemGem>,
    /// Three groups of six u32 modifiers in Item::SaveToDB source order.
    pub transmog_modifiers: [u32; 18],
    /// `Some` means the effective item template has an artifact, even when
    /// XP and the powers vector are both zero/empty.
    pub artifact: Option<NewItemArtifact>,
    pub modifiers: NewItemModifiers,
    pub subtype: NewItemSubtype,
}

#[derive(Clone, Copy)]
pub enum NewItemCreator {
    None,
    Character,
    /// Child equipment stores its parent's item counter as creator.
    Item(usize),
}

#[derive(Clone, Copy)]
pub struct NewItemEnchantment {
    pub id: u32,
    pub duration: u32,
    pub charges: u32,
}

#[derive(Clone, Copy, Default)]
pub struct NewVisualItem {
    pub item_id: u32,
    pub visible_item_id: u32,
    pub subclass: u8,
    pub inventory_type: u8,
    pub display_id: u32,
    pub display_enchant_id: u32,
    pub secondary_appearance_id: i32,
    pub sheathe_category: u8,
}

/// Canonical battleground save state, supplied explicitly by construction.
/// Position values retain the raw source float representation.
pub struct NewBattlegroundData {
    pub instance: u32,
    pub team: u16,
    pub position: [f32; 4],
    pub map: u16,
    pub taxi: [u32; 2],
    pub mount_spell: u32,
    pub queue_id: u64,
}

/// The represented creation save, using identities reserved before construction.
/// No database handles cross this boundary.
/// This does not yet cover every dependent group in Player::SaveToDB.
/// Names are private and deliberately have no Debug implementation.
pub struct NewCharacter {
    pub guid: u64,
    pub account: u32,
    pub realm: u32,
    pub name: String,
    pub surname: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u8,
    pub experience: u32,
    pub money: u64,
    pub map: u16,
    pub position: [f32; 4],
    pub create_time: i64,
    pub create_mode: i8,
    pub cinematic: u8,
    pub at_login: u16,
    pub health: u32,
    pub powers: [u32; 10],
    pub latency: u32,
    pub primary_specialization: u32,
    pub active_talent_group: u8,
    /// Realm version components, not a raw client build. Player::SaveToDB writes
    /// major * 10000 + minor * 100 + bugfix; no current realm writes zero.
    pub client_version: Option<[u32; 3]>,
    pub state: NewCharacterState,
    pub fishing_steps: u8,
    pub battleground: NewBattlegroundData,
    pub quests: NewQuestSave,
    pub specializations: NewSpecializations,
    pub traits: Vec<NewTraitChange>,
    pub spell_history: NewSpellHistory,
    pub auras: Vec<NewAuraSave>,
    pub stored_aura_locations: Vec<NewStoredAuraLocation>,
    pub achievements: NewAchievementSave,
    pub reputation: std::collections::BTreeMap<u32, NewReputation>,
    pub quest_criteria: NewQuestCriteriaSave,
    pub equipment_sets: std::collections::BTreeMap<u64, NewSavedEquipmentSet>,
    pub transmog_outfits: std::collections::BTreeMap<u32, Option<NewTransmogOutfit>>,
    pub session_save: NewCharacterSessionSave,
    pub currencies: Vec<NewCurrencySave>,
    pub cuf_profiles: [Option<NewCufProfile>; 5],
    pub player_data: NewPlayerDataSave,
    pub bank_tabs: Vec<NewBankTabSettings>,
    pub garrison: Option<NewGarrisonSave>,
    /// None freezes the source eligibility skip, not an unsupported-state fallback.
    pub statistics: Option<NewCharacterStatistics>,
    pub mail: NewMailSave,
    pub customizations: Vec<NewCustomization>,
    pub actions: Vec<NewAction>,
    pub skills: Vec<NewSkill>,
    pub spells: Vec<NewSpell>,
    pub items: Vec<NewItem>,
    pub visual_items: [NewVisualItem; 19],
}

/// Remaining primary-row state from the canonical constructed character.
/// There is deliberately no Default: the constructor must supply these values
/// after its source-backed initialization, rather than SQL inventing defaults.
pub struct NewCharacterState {
    pub inventory_slots: u8,
    pub inventory_bag_flags: u32,
    pub bag_slot_flags: [u32; 5],
    pub bank_slots: u8,
    pub bank_tabs: u8,
    pub bank_bag_flags: u32,
    pub rest_state: u8,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub instance_id: u32,
    pub dungeon_difficulty: u8,
    pub raid_difficulty: u8,
    pub legacy_raid_difficulty: u8,
    pub transport_position: [f32; 4],
    pub transport_guid: u64,
    pub taxi_mask: Vec<u8>,
    pub total_played_time: u32,
    pub level_played_time: u32,
    pub rest_bonus: f32,
    /// Current wall clock at save, distinct from character creation time.
    pub save_time: u64,
    pub logout_resting: bool,
    pub talent_reset_cost: u32,
    pub talent_reset_time: i64,
    pub extra_flags: u16,
    pub death_expire_time: i64,
    pub taxi_path: Option<NewTaxiPath>,
    pub lifetime_honorable_kills: u32,
    pub today_honorable_kills: u16,
    pub yesterday_honorable_kills: u16,
    pub chosen_title: u32,
    pub watched_faction: u32,
    pub drunk: u8,
    pub loot_specialization: u32,
    pub explored_zones: Vec<u64>,
    pub known_titles: Vec<u64>,
    pub action_bars: u8,
    pub personal_tabard: [i32; 5],
    pub transmog_outfit: i32,
    pub transmog_locked: bool,
}

pub struct NewTaxiPath {
    pub flight_master_faction: u32,
    /// PlayerTaxi::SaveTaxiDestinationsToString requires at least two nodes.
    pub nodes: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateError {
    Database,
    InvalidRow,
    NameInUse,
    AccountLimit,
    RealmLimit,
    CommitUnknown,
    LoginCommitUnknown,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreatedCharacter {
    pub guid: u64,
}
