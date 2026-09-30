//! Battle-pet account attachment, catalog access, and stat resolution.

use super::super::*;

impl WorldSession {
    /// Attach this session to the one canonical journal owner for its
    /// Battle.net account. The attachment releases any held journal lease on
    /// drop, matching C++ `WorldSession` teardown.
    pub fn set_battle_pet_account_attachment_like_cpp(
        &mut self,
        attachment: BattlePetAccountAttachmentLikeCpp,
    ) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.lifecycle.battle_pet_account_attachment_like_cpp = Some(attachment);
    }
    /// The #160 account owner and this session's journal lease id, when the
    /// canonical journal is attached (issue #161 purchase saga).
    pub(crate) fn battle_pet_account_owner_lease_like_cpp(
        &self,
    ) -> Option<(
        Arc<crate::battle_pet_account::BattlePetAccountOwnerLikeCpp>,
        crate::battle_pet_account::BattlePetLeaseIdLikeCpp,
    )> {
        self.lifecycle
            .battle_pet_account_attachment_like_cpp
            .as_ref()
            .map(|attachment| {
                (
                    Arc::clone(attachment.owner_like_cpp()),
                    attachment.lease_id_like_cpp(),
                )
            })
    }
    /// Set the world-DB battle-pet breed/quality selection store (#161).
    #[cfg(test)]
    pub fn set_battle_pet_selection_store_like_cpp(
        &mut self,
        store: Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_selection_store_like_cpp = Some(store);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn battle_pet_selection_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>> {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_selection_store_like_cpp
            .as_ref()
    }
    pub(crate) fn battle_pet_purchase_store_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>> {
        self.lifecycle
            .persistence_ports_like_cpp
            .player
            .battle_pet_purchase
            .as_ref()
            .map(Arc::clone)
    }
    #[cfg(test)]
    pub(crate) fn set_battle_pet_purchase_selection_override_like_cpp(
        &mut self,
        selection: Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_purchase_selection_override_like_cpp = selection;
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_purchase_selection_override_like_cpp(
        &self,
    ) -> Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp> {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_purchase_selection_override_like_cpp
    }
    #[cfg(test)]
    pub fn set_battle_pet_breed_state_store(&mut self, store: Arc<BattlePetBreedStateStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_breed_state_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_store(&mut self, store: Arc<BattlePetSpeciesStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_species_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_state_store(&mut self, store: Arc<BattlePetSpeciesStateStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_species_state_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_xp_game_table(&mut self, table: Arc<BattlePetXpGameTableLikeCpp>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_xp_game_table = Some(table);
    }
    pub(crate) fn battle_pet_calculate_stats_like_cpp(
        &self,
        breed: u16,
        species: u32,
        quality: u8,
        level: u16,
    ) -> Option<RepresentedBattlePetCalculatedStatsLikeCpp> {
        if let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .calculate_stats_like_cpp(breed, species, quality, level);
        }
        #[cfg(test)]
        {
            let stats = calculate_battle_pet_stats_like_cpp(
                breed,
                species,
                quality,
                level,
                self.battle_pet_test_fixture_like_cpp
                    .battle_pet_breed_state_store
                    .as_ref()?,
                self.battle_pet_test_fixture_like_cpp
                    .battle_pet_species_state_store
                    .as_ref()?,
                self.battle_pet_test_fixture_like_cpp
                    .battle_pet_breed_quality_store
                    .as_ref()?,
            )?;

            Some(RepresentedBattlePetCalculatedStatsLikeCpp {
                max_health: stats.max_health,
                power: stats.power,
                speed: stats.speed,
            })
        }
        #[cfg(not(test))]
        None
    }
    pub(in crate::session) fn battle_pet_species_has_flag_like_cpp(
        &self,
        species: u32,
        flag: i32,
    ) -> bool {
        if let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .species_has_flag_like_cpp(species, flag);
        }
        #[cfg(test)]
        return self
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .is_some_and(|entry| entry.has_flag_like_cpp(flag));
        #[cfg(not(test))]
        false
    }
}
