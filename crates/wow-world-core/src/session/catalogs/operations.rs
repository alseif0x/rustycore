use std::sync::Arc;

use crate::session::state::SessionCatalogs;
use wow_packet::packets::misc::{FeatureSystemStatus, FeatureSystemStatusGlueScreen};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::ObjectMgrCatalogsLikeCpp;
use wow_data::trait_tree::TraitDefinitionStore;
use wow_data::{
    AccessRequirementStoreLikeCpp, ChrSpecializationStore, CinematicSequencesStore,
    ConditionEntriesByTypeStore, LockStore, MountStore, PlayerConditionStore,
    RandPropPointsStore, SpellPetAuraStoreLikeCpp, TrainerStoreLikeCpp, WorldSafeLocStore,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{
    BattlemasterListStore, ExplorationBaseXpStoreLikeCpp, GraveyardStore, ImportPriceStores,
    LfgDungeonStoreLikeCpp, PetDefaultSpellStoreLikeCpp, PetFamilySpellStoreLikeCpp,
    PetLevelupSpellStoreLikeCpp, TactKeyStore,
};

use crate::session::SupportFeaturePolicyLikeCpp;

impl crate::session::state::SessionCore {
    pub fn feature_system_status_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
    ) -> FeatureSystemStatus {
        FeatureSystemStatus::from_config_like_cpp(
            policy.feature_system_config_like_cpp(),
            !self.can_speak_like_cpp(),
        )
    }

    pub fn feature_system_status_glue_screen_with_policy_like_cpp(
        &self,
        policy: &SupportFeaturePolicyLikeCpp,
    ) -> FeatureSystemStatusGlueScreen {
        FeatureSystemStatusGlueScreen::from_config_like_cpp(
            policy.feature_system_config_like_cpp(),
            policy.max_characters_per_realm as i32,
            i32::from(self.realm_policy.server_expansion_like_cpp),
        )
    }
}

impl SessionCatalogs {
    pub fn trainer_store_like_cpp(&self) -> Option<&Arc<TrainerStoreLikeCpp>> {
        self.trainer_store_like_cpp.as_ref()
    }

    /// Set the C++ ImportPrice*.db2 stores for this session.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_import_price_stores(&mut self, stores: Arc<ImportPriceStores>) {
        self.import_price_stores = Some(stores);
    }

    /// Get the random property points store reference.
    pub fn rand_prop_points_store(&self) -> Option<&Arc<RandPropPointsStore>> {
        self.rand_prop_points_store.as_ref()
    }

    /// Get the loaded ConditionMgr store reference.
    pub fn condition_store(&self) -> Option<&Arc<ConditionEntriesByTypeStore>> {
        self.condition_store.as_ref()
    }

    /// Get the loaded PlayerCondition.db2 store reference.
    pub fn player_condition_store(&self) -> Option<&Arc<PlayerConditionStore>> {
        self.player_condition_store.as_ref()
    }

    pub fn lock_store(&self) -> Option<&Arc<LockStore>> {
        self.lock_store.as_ref()
    }

    /// Set the TactKey.db2 store for typed SMSG_DB_REPLY serialization.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_tact_key_store(&mut self, store: Arc<TactKeyStore>) {
        self.tact_key_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_graveyard_store(&mut self, store: Arc<GraveyardStore>) {
        self.graveyard_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn graveyard_store(&self) -> Option<&Arc<GraveyardStore>> {
        self.graveyard_store.as_ref()
    }

    /// Get the ChrSpecialization store reference.
    pub fn chr_specialization_store(&self) -> Option<&Arc<ChrSpecializationStore>> {
        self.chr.specialization_store.as_ref()
    }

    pub fn world_safe_loc_store_like_cpp(&self) -> Option<&Arc<WorldSafeLocStore>> {
        self.world_safe_loc_store_like_cpp.as_ref()
    }

    pub fn access_requirement_store(&self) -> Option<&Arc<AccessRequirementStoreLikeCpp>> {
        self.access_requirement_store.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn lfg_dungeon_store_like_cpp(&self) -> Option<&Arc<LfgDungeonStoreLikeCpp>> {
        self.lfg_dungeon_store_like_cpp.as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battlemaster_list_store(&mut self, store: Arc<BattlemasterListStore>) {
        self.battlemaster_list_store = Some(store);
    }

    pub fn faction_store(&self) -> Option<&Arc<wow_data::progression_rewards::FactionStore>> {
        self.factions.store.as_ref()
    }

    pub fn mount_store(&self) -> Option<&Arc<MountStore>> {
        self.mount_store.as_ref()
    }

    pub fn trait_definition_store(&self) -> Option<&Arc<TraitDefinitionStore>> {
        self.trait_definition_store.as_ref()
    }

    pub fn trait_tree_skill_line_index(
        &self,
    ) -> Option<&Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>> {
        self.trait_tree_skill_line_index.as_ref()
    }

    pub fn spell_pet_aura_store_like_cpp(&self) -> Option<&SpellPetAuraStoreLikeCpp> {
        self.spell_catalogs.spell_pet_aura_store.as_deref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_pet_levelup_spell_store(&mut self, store: Arc<PetLevelupSpellStoreLikeCpp>) {
        self.spell_catalogs.pet_levelup_spell_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_pet_default_spell_store(&mut self, store: Arc<PetDefaultSpellStoreLikeCpp>) {
        self.spell_catalogs.pet_default_spell_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_pet_family_spell_store(&mut self, store: Arc<PetFamilySpellStoreLikeCpp>) {
        self.spell_catalogs.pet_family_spell_store = Some(store);
    }

    pub fn set_cinematic_sequences_store(&mut self, store: Arc<CinematicSequencesStore>) {
        self.cinematic_sequences_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_object_mgr_catalogs_like_cpp(&mut self, catalogs: Arc<ObjectMgrCatalogsLikeCpp>) {
        self.object_mgr_catalogs_like_cpp = Some(catalogs);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn world_query_catalogs_like_cpp(&self) -> Option<&ObjectMgrCatalogsLikeCpp> {
        self.object_mgr_catalogs_like_cpp.as_deref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_exploration_base_xp_store_like_cpp(
        &mut self,
        store: Arc<ExplorationBaseXpStoreLikeCpp>,
    ) {
        self.exploration_base_xp_store = Some(store);
    }
}
