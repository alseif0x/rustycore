//! Handle-less battle-pet state used by isolated Session tests.

use super::{
    Arc, BATTLE_PET_SLOT_COUNT_LIKE_CPP, BTreeMap, BattlePetBreedQualityStore,
    BattlePetBreedStateStore, BattlePetSpeciesStateStore, BattlePetSpeciesStore,
    BattlePetXpGameTableLikeCpp, HashMap, ObjectGuid, RepresentedBattlePetCageItemLikeCpp,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetLevelCriteriaLikeCpp,
    RepresentedBattlePetQueryCompanionLikeCpp, RepresentedBattlePetSlotLikeCpp,
};

pub(crate) struct BattlePetTestFixtureLikeCpp {
    // C++ `BattlePet::CalculateStats` inputs projected for isolated tests.
    pub(in crate::session) battle_pet_breed_quality_store: Option<Arc<BattlePetBreedQualityStore>>,
    pub(in crate::session) battle_pet_breed_state_store: Option<Arc<BattlePetBreedStateStore>>,
    pub(in crate::session) battle_pet_species_store: Option<Arc<BattlePetSpeciesStore>>,
    pub(in crate::session) battle_pet_species_state_store: Option<Arc<BattlePetSpeciesStateStore>>,
    pub(in crate::session) battle_pet_xp_game_table: Option<Arc<BattlePetXpGameTableLikeCpp>>,

    /// C++ `BattlePetMgr::_pets`, represented minimally until full battle-pet runtime is ported.
    /// Production sessions use `battle_pet_account_attachment_like_cpp`; this
    /// map remains only as the isolated represented/test fallback.
    pub(crate) represented_battle_pets_like_cpp:
        HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    /// World-DB breed/quality selection tables for battle-pet trainer
    /// purchases (issue #161), loaded once at bootstrap.
    pub(in crate::session) battle_pet_selection_store_like_cpp:
        Option<Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>>,
    /// Deterministic purchase selection override for saga tests (#161).
    pub(in crate::session) battle_pet_purchase_selection_override_like_cpp:
        Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    /// C++ `BattlePetMgr::_hasJournalLock`, represented until full battle-pet runtime is ported.
    pub(crate) represented_battle_pet_journal_lock_like_cpp: bool,
    /// C++ `BattlePetMgr::_slots`, represented until full battle-pet slot
    /// persistence is ported.
    pub(crate) represented_battle_pet_slots_like_cpp:
        [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
    /// True only when the isolated represented/test fallback above was
    /// replaced from one complete account-DB slot query. Production sessions
    /// instead prove this through `battle_pet_account_attachment_like_cpp`,
    /// which is published only after the canonical account load succeeds.
    pub(in crate::session) represented_battle_pet_slots_authority_complete_like_cpp: bool,
    /// Isolated-test fallback for canonical C++
    /// `ActivePlayerData::SummonedBattlePetGUID` ownership.
    pub(crate) represented_summoned_battle_pet_guid_like_cpp: Option<ObjectGuid>,
    /// Isolated-test fallback for canonical C++ `UnitData::Critter` ownership.
    pub(crate) represented_critter_guid_like_cpp: Option<ObjectGuid>,
    /// Evidence for represented `TempSummon::UnSummon` from `CMSG_DISMISS_CRITTER`.
    pub(crate) represented_dismissed_critter_guids_like_cpp: Vec<ObjectGuid>,
    /// C++ ObjectAccessor/TempSummon query state for `CMSG_QUERY_BATTLE_PET_NAME`.
    pub(crate) represented_battle_pet_query_companions_like_cpp:
        HashMap<ObjectGuid, RepresentedBattlePetQueryCompanionLikeCpp>,
    /// Represented caged-item creations from C++ `BattlePetMgr::CageBattlePet`.
    pub(crate) represented_battle_pet_cage_items_like_cpp: Vec<RepresentedBattlePetCageItemLikeCpp>,
    /// C++ `sBattlePetXPGameTable` projected as level -> `uint16(Wins * Xp)`.
    pub(crate) represented_battle_pet_xp_per_level_like_cpp: BTreeMap<u16, u16>,
    /// Represented `CriteriaType::BattlePetReachLevel` events from battle-pet level grants.
    pub(crate) represented_battle_pet_level_criteria_like_cpp:
        Vec<RepresentedBattlePetLevelCriteriaLikeCpp>,
    /// Represented `CriteriaType::ActivelyEarnPetLevel` events from pet-battle XP grants.
    pub(crate) represented_battle_pet_active_level_criteria_like_cpp:
        Vec<RepresentedBattlePetLevelCriteriaLikeCpp>,
    /// Represented `CriteriaType::UniquePetsOwned` updates from `BattlePetMgr::AddPet`.
    pub(crate) represented_battle_pet_unique_owned_criteria_like_cpp: u32,
    /// Represented `CriteriaType::LearnedNewPet` updates from `BattlePetMgr::AddPet`.
    pub(crate) represented_battle_pet_learned_new_pet_criteria_like_cpp: Vec<u32>,
    /// Evidence for represented `BattlePetMgr::UpdateBattlePetData` calls.
    pub(crate) represented_battle_pet_data_updates_like_cpp: Vec<ObjectGuid>,
}

impl Default for BattlePetTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            battle_pet_breed_quality_store: None,
            battle_pet_breed_state_store: None,
            battle_pet_species_store: None,
            battle_pet_species_state_store: None,
            battle_pet_xp_game_table: None,
            represented_battle_pets_like_cpp: HashMap::new(),
            battle_pet_selection_store_like_cpp: None,
            battle_pet_purchase_selection_override_like_cpp: None,
            represented_battle_pet_journal_lock_like_cpp: false,
            represented_battle_pet_slots_like_cpp: std::array::from_fn(|index| {
                RepresentedBattlePetSlotLikeCpp::locked_empty(index as u8)
            }),
            represented_battle_pet_slots_authority_complete_like_cpp: false,
            represented_summoned_battle_pet_guid_like_cpp: None,
            represented_critter_guid_like_cpp: None,
            represented_dismissed_critter_guids_like_cpp: Vec::new(),
            represented_battle_pet_query_companions_like_cpp: HashMap::new(),
            represented_battle_pet_cage_items_like_cpp: Vec::new(),
            represented_battle_pet_xp_per_level_like_cpp: BTreeMap::new(),
            represented_battle_pet_level_criteria_like_cpp: Vec::new(),
            represented_battle_pet_active_level_criteria_like_cpp: Vec::new(),
            represented_battle_pet_unique_owned_criteria_like_cpp: 0,
            represented_battle_pet_learned_new_pet_criteria_like_cpp: Vec::new(),
            represented_battle_pet_data_updates_like_cpp: Vec::new(),
        }
    }
}
