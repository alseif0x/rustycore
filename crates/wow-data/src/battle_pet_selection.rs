// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle-pet trainer purchase materialization inputs (issue #161).
//!
//! C++ anchors
//! (`/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp`):
//! - `LoadAvailablePetBreeds` (115-144): world `battle_pet_breeds` table,
//!   rows naming an unknown species are skipped; an empty/missing table is
//!   tolerated.
//! - `LoadDefaultPetQualities` (146-184): world `battle_pet_quality` table,
//!   rows naming an unknown species, a quality `>= Count`, or a `WellKnown`
//!   species with quality above `Rare` are skipped.
//! - `RollPetBreed` (201-208): uniform random element of the species breed
//!   set, default `3` ("B/B") when the species has no rows.
//! - `GetDefaultPetQuality` (210-217): table lookup, default `Poor` (0).
//! - `SelectPetDisplay` (219-227): `0` when the species creature template is
//!   missing or the species carries `BattlePetSpeciesFlags::RandomDisplay`;
//!   otherwise `CreatureTemplate::GetRandomValidModel()->CreatureDisplayID`.
//!
//! Every random choice takes an injectable RNG so tests can pin C++-shaped
//! outcomes deterministically.

use std::collections::HashMap;

use rand::Rng;
use tracing::warn;
use wow_database::{WorldDatabase, WorldStatements};

use crate::creature_template::{
    CreatureModelSelectionRandomLikeCpp, CreatureTemplateLifecycleRecordLikeCpp,
};
use crate::item_collections::{
    BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP, BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
    BattlePetSpeciesEntry,
};

/// C++ `BattlePetBreedQuality::Poor`.
pub const BATTLE_PET_BREED_QUALITY_POOR_LIKE_CPP: u8 = 0;
/// C++ `BattlePetBreedQuality::Rare`; `WellKnown` species overrides may not exceed it.
pub const BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP: u8 = 3;
/// C++ `BattlePetBreedQuality::Count`; loaded qualities must stay below it.
pub const BATTLE_PET_BREED_QUALITY_COUNT_LIKE_CPP: u8 = 6;
/// C++ `RollPetBreed` fallback ("default B/B").
pub const BATTLE_PET_BREED_DEFAULT_BB_LIKE_CPP: u16 = 3;

/// Uniform index source for C++ `Trinity::Containers::SelectRandomContainerElement`.
pub trait BattlePetSelectionRandomLikeCpp {
    fn uniform_index_like_cpp(&mut self, len: usize) -> usize;
}

impl<R: Rng + ?Sized> BattlePetSelectionRandomLikeCpp for R {
    fn uniform_index_like_cpp(&mut self, len: usize) -> usize {
        self.gen_range(0..len)
    }
}

/// World-DB battle-pet breed/quality tables, loaded once at bootstrap like
/// the C++ static loaders.
#[derive(Debug, Default)]
pub struct BattlePetSelectionStoreLikeCpp {
    /// Species → sorted deduplicated breed set (C++ keeps a `std::set`; only
    /// membership and size are observable through the uniform roll).
    breeds: HashMap<u32, Vec<u16>>,
    /// Species → validated default quality override.
    qualities: HashMap<u32, u8>,
}

impl BattlePetSelectionStoreLikeCpp {
    pub fn new_for_test_like_cpp(
        breeds: HashMap<u32, Vec<u16>>,
        qualities: HashMap<u32, u8>,
    ) -> Self {
        Self { breeds, qualities }
    }

    /// Mirrors the C++ loaders' tolerance: a failed query (missing table) is
    /// logged and treated as empty, exactly like a null `QueryResult`.
    pub async fn load_like_cpp<SpeciesFlags>(
        db: &WorldDatabase,
        mut species_flags: SpeciesFlags,
    ) -> Self
    where
        SpeciesFlags: FnMut(u32) -> Option<i32>,
    {
        let mut store = Self::default();

        let statement = db.prepare(WorldStatements::SEL_BATTLE_PET_BREEDS);
        match db.query(&statement).await {
            Ok(mut result) => {
                if !result.is_empty() {
                    loop {
                        let species: u32 = result.try_read(0).unwrap_or_default();
                        let breed: u16 = result.try_read(1).unwrap_or_default();
                        if species_flags(species).is_none() {
                            warn!(
                                target: "sql.sql",
                                species,
                                breed,
                                "Non-existing BattlePetSpecies.db2 entry referenced in `battle_pet_breeds`"
                            );
                        } else {
                            let entry = store.breeds.entry(species).or_default();
                            if !entry.contains(&breed) {
                                entry.push(breed);
                            }
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                warn!(
                    target: "sql.sql",
                    %error,
                    ">> Loaded 0 battle pet breeds. DB table `battle_pet_breeds` could not be read."
                );
            }
        }
        for breeds in store.breeds.values_mut() {
            breeds.sort_unstable();
        }

        let statement = db.prepare(WorldStatements::SEL_BATTLE_PET_QUALITY);
        match db.query(&statement).await {
            Ok(mut result) => {
                if !result.is_empty() {
                    loop {
                        let species: u32 = result.try_read(0).unwrap_or_default();
                        let quality: u8 = result.try_read(1).unwrap_or_default();
                        match species_flags(species) {
                            None => {
                                warn!(
                                    target: "sql.sql",
                                    species,
                                    quality,
                                    "Non-existing BattlePetSpecies.db2 entry referenced in `battle_pet_quality`"
                                );
                            }
                            Some(flags) if quality >= BATTLE_PET_BREED_QUALITY_COUNT_LIKE_CPP => {
                                warn!(
                                    target: "sql.sql",
                                    species,
                                    quality,
                                    "BattlePetSpecies.db2 entry referenced in `battle_pet_quality` with non-existing quality"
                                );
                                let _ = flags;
                            }
                            Some(flags)
                                if flags & BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP != 0
                                    && quality > BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP =>
                            {
                                warn!(
                                    target: "sql.sql",
                                    species,
                                    quality,
                                    "Learnable BattlePetSpecies.db2 entry referenced in `battle_pet_quality` with invalid quality"
                                );
                            }
                            Some(_) => {
                                store.qualities.insert(species, quality);
                            }
                        }
                        if !result.next_row() {
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                warn!(
                    target: "sql.sql",
                    %error,
                    ">> Loaded 0 battle pet qualities. DB table `battle_pet_quality` could not be read."
                );
            }
        }

        store
    }

    pub fn len_like_cpp(&self) -> usize {
        self.breeds.len() + self.qualities.len()
    }
}

/// C++ `BattlePetMgr::RollPetBreed`: uniform element of the species breed
/// set, default `3` ("B/B") when the species has no rows.
pub fn roll_pet_breed_like_cpp<R: BattlePetSelectionRandomLikeCpp>(
    store: &BattlePetSelectionStoreLikeCpp,
    species: u32,
    random: &mut R,
) -> u16 {
    match store.breeds.get(&species) {
        None => BATTLE_PET_BREED_DEFAULT_BB_LIKE_CPP,
        Some(breeds) => breeds[random.uniform_index_like_cpp(breeds.len())],
    }
}

/// C++ `BattlePetMgr::GetDefaultPetQuality`: table lookup, default `Poor`.
pub fn default_pet_quality_like_cpp(store: &BattlePetSelectionStoreLikeCpp, species: u32) -> u8 {
    store
        .qualities
        .get(&species)
        .copied()
        .unwrap_or(BATTLE_PET_BREED_QUALITY_POOR_LIKE_CPP)
}

/// C++ `BattlePetMgr::SelectPetDisplay`: `0` when the species creature
/// template is missing or the species carries `RandomDisplay`; otherwise the
/// weighted-random valid model display id.
pub fn select_pet_display_like_cpp<R: CreatureModelSelectionRandomLikeCpp>(
    species: &BattlePetSpeciesEntry,
    creature_template: Option<&CreatureTemplateLifecycleRecordLikeCpp>,
    random: &mut R,
) -> u32 {
    if species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP) {
        return 0;
    }
    creature_template
        .and_then(|template| template.random_valid_model_like_cpp(random))
        .map(|model| model.creature_display_id)
        .unwrap_or(0)
}

/// Fully materialized purchase inputs, frozen at admission so a recovered
/// command never re-rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetTrainerSelectionLikeCpp {
    pub species: u32,
    pub breed: u16,
    pub quality: u8,
    pub display_id: u32,
    /// C++ `BattlePetMgr::AddPet` default level for trainer-granted pets.
    pub level: u16,
}

/// Compose the C++ `Trainer::TeachSpell` `AddPet` argument triple plus the
/// C++ default level 1.
pub fn select_battle_pet_trainer_pet_like_cpp<
    B: BattlePetSelectionRandomLikeCpp,
    R: CreatureModelSelectionRandomLikeCpp,
>(
    store: &BattlePetSelectionStoreLikeCpp,
    species: &BattlePetSpeciesEntry,
    creature_template: Option<&CreatureTemplateLifecycleRecordLikeCpp>,
    breed_random: &mut B,
    display_random: &mut R,
) -> BattlePetTrainerSelectionLikeCpp {
    BattlePetTrainerSelectionLikeCpp {
        species: species.id,
        breed: roll_pet_breed_like_cpp(store, species.id, breed_random),
        quality: default_pet_quality_like_cpp(store, species.id),
        display_id: select_pet_display_like_cpp(species, creature_template, display_random),
        level: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic index source replaying a scripted pick sequence.
    struct ScriptedSelectionRandomLikeCpp {
        picks: Vec<usize>,
    }

    impl BattlePetSelectionRandomLikeCpp for ScriptedSelectionRandomLikeCpp {
        fn uniform_index_like_cpp(&mut self, len: usize) -> usize {
            assert!(!self.picks.is_empty(), "scripted picks exhausted");
            let pick = self.picks.remove(0);
            assert!(pick < len, "scripted pick {pick} out of range {len}");
            pick
        }
    }

    struct ScriptedModelRandomLikeCpp {
        roll: f32,
    }

    impl CreatureModelSelectionRandomLikeCpp for ScriptedModelRandomLikeCpp {
        fn weighted_model_roll_like_cpp(&mut self, _total_weight: f32) -> f32 {
            self.roll
        }

        fn other_gender_roll_zero_like_cpp(&mut self) -> bool {
            true
        }
    }

    fn store_with(
        breeds: &[(u32, u16)],
        qualities: &[(u32, u8)],
    ) -> BattlePetSelectionStoreLikeCpp {
        let mut breed_map: HashMap<u32, Vec<u16>> = HashMap::new();
        for (species, breed) in breeds {
            breed_map.entry(*species).or_default().push(*breed);
        }
        for values in breed_map.values_mut() {
            values.sort_unstable();
            values.dedup();
        }
        BattlePetSelectionStoreLikeCpp::new_for_test_like_cpp(
            breed_map,
            qualities.iter().copied().collect(),
        )
    }

    fn species(id: u32, flags: i32) -> BattlePetSpeciesEntry {
        BattlePetSpeciesEntry {
            id,
            description: String::new(),
            source_text: String::new(),
            creature_id: 100,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        }
    }

    #[test]
    fn breed_roll_defaults_to_bb_like_cpp() {
        let store = store_with(&[], &[]);
        let mut random = ScriptedSelectionRandomLikeCpp { picks: vec![] };
        assert_eq!(roll_pet_breed_like_cpp(&store, 42, &mut random), 3);
    }

    #[test]
    fn breed_roll_picks_a_uniform_set_element_like_cpp() {
        let store = store_with(&[(42, 5), (42, 3), (42, 8)], &[]);
        // Sorted set is [3, 5, 8]; C++ `SelectRandomContainerElement` is a
        // uniform index over the container.
        for (pick, expected) in [(0_usize, 3_u16), (1, 5), (2, 8)] {
            let mut random = ScriptedSelectionRandomLikeCpp { picks: vec![pick] };
            assert_eq!(roll_pet_breed_like_cpp(&store, 42, &mut random), expected);
        }
    }

    #[test]
    fn quality_defaults_to_poor_and_honors_override_like_cpp() {
        let store = store_with(&[], &[(42, 2)]);
        assert_eq!(default_pet_quality_like_cpp(&store, 42), 2);
        assert_eq!(default_pet_quality_like_cpp(&store, 43), 0);
    }

    #[test]
    fn display_is_zero_for_random_display_species_or_missing_template_like_cpp() {
        let store = store_with(&[], &[]);
        let _ = &store;
        let random_display = species(1, BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP);
        let mut model_random = ScriptedModelRandomLikeCpp { roll: 0.0 };
        assert_eq!(
            select_pet_display_like_cpp(&random_display, None, &mut model_random),
            0
        );
        let plain = species(2, 0);
        assert_eq!(
            select_pet_display_like_cpp(&plain, None, &mut model_random),
            0
        );
    }

    #[test]
    fn display_rolls_the_weighted_valid_model_like_cpp() {
        use crate::creature_template::CreatureTemplateLifecycleModelLikeCpp;

        // C++ `CreatureTemplate::GetRandomValidModel` walks the weighted
        // model list subtracting probabilities; scripted rolls pin the picks.
        let mut record =
            crate::creature_template::tests::creature_template_lifecycle_record_for_test(99);
        record.models = vec![
            CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: 111,
                display_scale: 1.0,
                probability: 0.25,
            },
            CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: 222,
                display_scale: 1.0,
                probability: 0.75,
            },
        ];
        let entry = species(3, 0);
        for (roll, expected) in [(0.2_f32, 111_u32), (0.25, 111), (0.26, 222), (0.9, 222)] {
            let mut model_random = ScriptedModelRandomLikeCpp { roll };
            assert_eq!(
                select_pet_display_like_cpp(&entry, Some(&record), &mut model_random),
                expected,
                "roll {roll}"
            );
        }

        // A `RandomDisplay` species returns 0 even with valid models.
        let random_display = species(4, BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP);
        let mut model_random = ScriptedModelRandomLikeCpp { roll: 0.0 };
        assert_eq!(
            select_pet_display_like_cpp(&random_display, Some(&record), &mut model_random),
            0
        );
    }

    #[test]
    fn selection_composes_breed_quality_display_and_level_one_like_cpp() {
        let store = store_with(&[(42, 7)], &[(42, 1)]);
        let entry = species(42, BATTLE_PET_SPECIES_FLAG_RANDOM_DISPLAY_LIKE_CPP);
        let mut breed_random = ScriptedSelectionRandomLikeCpp { picks: vec![0] };
        let mut model_random = ScriptedModelRandomLikeCpp { roll: 0.0 };
        let selection = select_battle_pet_trainer_pet_like_cpp(
            &store,
            &entry,
            None,
            &mut breed_random,
            &mut model_random,
        );
        assert_eq!(
            selection,
            BattlePetTrainerSelectionLikeCpp {
                species: 42,
                breed: 7,
                quality: 1,
                display_id: 0,
                level: 1,
            }
        );
    }
}
