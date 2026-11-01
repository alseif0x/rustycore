#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;

use crate::battle_pet_account::BattlePetMutationFailureLikeCpp;
use crate::session::battle_pet_adapter::RepresentedBattlePetQueryCompanionLikeCpp;
use tracing::warn;
use wow_core::ObjectGuid;
use wow_entities::Player;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::battle_pet_adapter::{
    BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP, BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP,
    RepresentedBattlePetCageItemLikeCpp, RepresentedBattlePetDataLikeCpp,
    RepresentedBattlePetSaveInfoLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::{
    BattlePetBreedStateStore, BattlePetSpeciesStateStore, BattlePetSpeciesStore,
    BattlePetXpGameTableLikeCpp,
};

impl crate::session::HubMut<'_> {
    /// C++ `BattlePetMgr::HealBattlePetsPct`.
    ///
    /// Fidelity note: legacy C++ does not skip removed pets and would rewrite a
    /// damaged `BATTLE_PET_REMOVED` row to `BATTLE_PET_CHANGED`. Rust keeps the
    /// represented removed row immutable here; if we decide to patch that legacy
    /// bug upstream, this is the intended shared behavior.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_heal_battle_pets_pct_like_cpp(&mut self, pct: u8) -> usize {
        let mut updated = Vec::new();

        for (pet_guid, pet) in &mut self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
        {
            if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                continue;
            }

            if pet.health == pet.max_health {
                continue;
            }

            let heal = (pet.max_health as f32 * f32::from(pct) / 100.0f32) as u32;
            pet.health = pet.health.saturating_add(heal).min(pet.max_health);
            if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
                pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
            }
            updated.push(*pet_guid);
        }

        self.send_battle_pet_updates_like_cpp(&updated, false)
    }

    pub fn set_represented_summoned_battle_pet_guid_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
    ) -> bool {
        if self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_summoned_battle_pet_guid_like_cpp(
                    pet_guid.unwrap_or(wow_core::ObjectGuid::EMPTY),
                );
            })
            .is_some()
        {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_summoned_battle_pet_guid_like_cpp = pet_guid;
            true
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            false
        }
    }
}

impl crate::session::HubRef<'_> {
    pub fn log_battle_pet_mutation_failure_like_cpp(
        &self,
        operation: &'static str,
        pet_guid: ObjectGuid,
        error: &BattlePetMutationFailureLikeCpp,
    ) {
        if matches!(
            error,
            BattlePetMutationFailureLikeCpp::MissingAuthority
                | BattlePetMutationFailureLikeCpp::JournalLocked
                | BattlePetMutationFailureLikeCpp::UnknownPet
        ) {
            return;
        }
        warn!(
            account = self.core.account_id,
            ?pet_guid,
            ?error,
            operation,
            "Durable battle-pet mutation failed"
        );
    }

    pub fn represented_summoned_battle_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) = self
            .core
            .with_owned_player_like_cpp(Player::summoned_battle_pet_guid_like_cpp)
        {
            return guid;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_summoned_battle_pet_guid_like_cpp
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            None
        }
    }

    pub fn represented_battle_pet_query_companion_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetQueryCompanionLikeCpp> {
        if let Some(manager) = self.core.canonical_map_manager.as_ref()
            && let Ok(manager) = manager.lock()
        {
            let mut snapshot = None;
            manager.do_for_all_maps(|managed| {
                if snapshot.is_some() {
                    return;
                }
                let Some(companion) = managed.map().with_creature_like_cpp(unit_guid, |creature| {
                    let unit = creature.unit();
                    RepresentedBattlePetQueryCompanionLikeCpp {
                        creature_id: i32::try_from(creature.entry()).unwrap_or(i32::MAX),
                        name_timestamp: i64::from(
                            unit.battle_pet_companion_name_timestamp_like_cpp(),
                        ),
                        is_summon: creature.is_summon_like_cpp(),
                        owner_is_player: unit
                            .subsystems()
                            .control
                            .owner_guid
                            .is_some_and(|guid| guid.is_player()),
                        battle_pet_companion_guid: unit.battle_pet_companion_guid_like_cpp(),
                    }
                }) else {
                    return;
                };
                snapshot = Some(companion);
            });
            if snapshot.is_some() {
                return snapshot;
            }
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_query_companions_like_cpp
                .get(&unit_guid)
                .copied()
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            None
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_selection_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>> {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_selection_store_like_cpp
            .as_ref()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_purchase_selection_override_like_cpp(
        &mut self,
        selection: Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_purchase_selection_override_like_cpp = selection;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn battle_pet_purchase_selection_override_like_cpp(
        &self,
    ) -> Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp> {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_purchase_selection_override_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_breed_state_store(&mut self, store: Arc<BattlePetBreedStateStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_breed_state_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_species_store(&mut self, store: Arc<BattlePetSpeciesStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_species_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_species_state_store(&mut self, store: Arc<BattlePetSpeciesStateStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_species_state_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_xp_game_table(&mut self, table: Arc<BattlePetXpGameTableLikeCpp>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_xp_game_table = Some(table);
    }

    /// Test/setup seam for represented `BattlePetMgr::_pets`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn add_represented_battle_pet_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        save_info: RepresentedBattlePetSaveInfoLikeCpp,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .insert(
                pet_guid,
                RepresentedBattlePetDataLikeCpp::minimal_like_cpp(flags, save_info),
            );
    }

    /// C++ `BattlePetMgr::ClearFanfare`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_clear_fanfare_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        pet.flags &= !BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }
        true
    }

    /// C++ `WorldSession::HandleBattlePetSetFlags` flag mutation.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn battle_pet_set_flags_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> bool {
        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        if control_type == BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP {
            pet.flags |= flags;
        } else {
            pet.flags &= !flags;
        }

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }
        true
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_unique_owned_criteria_like_cpp(&self) -> u32 {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_unique_owned_criteria_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_learned_new_pet_criteria_like_cpp(&self) -> &[u32] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_learned_new_pet_criteria_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_cage_items_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetCageItemLikeCpp] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_cage_items_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_battle_pet_query_companion_like_cpp(
        &mut self,
        unit_guid: ObjectGuid,
        companion: RepresentedBattlePetQueryCompanionLikeCpp,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_query_companions_like_cpp
            .insert(unit_guid, companion);
    }
}
