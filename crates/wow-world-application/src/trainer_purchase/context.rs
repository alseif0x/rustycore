// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_data::{ItemStatsStore, ItemStore, SkillLineStore, SkillStore, SkillTiersStoreLikeCpp};
use wow_world_core::session::PlayerAcquisitionOwnerAccessLikeCpp;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot::LootState;
use wow_world_spell::SessionSpellState;

use crate::SessionQuestState;

/// The catalog handles read by the admitted trainer acquisition operation.
pub struct TrainerAcquisitionCatalogsLikeCpp<'a> {
    pub(crate) skills: Option<&'a Arc<SkillStore>>,
    pub(crate) skill_lines: Option<&'a Arc<SkillLineStore>>,
    pub(crate) skill_tiers: Option<&'a Arc<SkillTiersStoreLikeCpp>>,
    pub(crate) item_store: Option<&'a Arc<ItemStore>>,
    pub(crate) item_stats_store: Option<&'a Arc<ItemStatsStore>>,
}

impl<'a> TrainerAcquisitionCatalogsLikeCpp<'a> {
    /// Select only the catalogs read by trainer publication and validation.
    pub fn new(
        skills: Option<&'a Arc<SkillStore>>,
        skill_lines: Option<&'a Arc<SkillLineStore>>,
        skill_tiers: Option<&'a Arc<SkillTiersStoreLikeCpp>>,
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    ) -> Self {
        Self {
            skills,
            skill_lines,
            skill_tiers,
            item_store,
            item_stats_store,
        }
    }
}

/// Existing caller-owned test inputs used by trainer installation and registry
/// synchronization. The consumer's `cfg(test)` choice remains an explicit
/// value so a test-fixtures feature build does not change production behavior.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct TrainerAcquisitionFixturesLikeCpp<'a> {
    pub(crate) player_race: &'a u8,
    pub(crate) player_class: &'a u8,
    pub(crate) player_level: &'a u8,
    pub(crate) player_skill_fixture:
        &'a mut wow_world_core::session::PlayerSkillTestFixtureLikeCpp,
    pub(crate) represented_enchanting_skill: &'a mut u16,
    pub(crate) registry_position: &'a Option<wow_core::Position>,
    pub(crate) registry_health: &'a u32,
    pub(crate) registry_max_health: &'a u32,
    pub(crate) registry_alive: &'a bool,
    pub(crate) registry_level: &'a u8,
    pub(crate) registry_transport:
        &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    pub(crate) registry_mount_vehicle_kit: &'a Option<wow_entities::Vehicle>,
    pub(crate) registry_vehicle_seat_flags: &'a Option<i32>,
    pub(crate) registry_vehicle_seat_id: &'a Option<u32>,
    pub(crate) registry_pet_guid: &'a Option<wow_core::ObjectGuid>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> TrainerAcquisitionFixturesLikeCpp<'a> {
    /// Borrow the existing identity, skill and registry fixtures selected by
    /// the World consumer; none are synthesized by the application.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        player_race: &'a u8,
        player_class: &'a u8,
        player_level: &'a u8,
        player_skill_fixture: &'a mut wow_world_core::session::PlayerSkillTestFixtureLikeCpp,
        represented_enchanting_skill: &'a mut u16,
        registry_position: &'a Option<wow_core::Position>,
        registry_health: &'a u32,
        registry_max_health: &'a u32,
        registry_alive: &'a bool,
        registry_level: &'a u8,
        registry_transport: &'a Option<
            Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>,
        >,
        registry_mount_vehicle_kit: &'a Option<wow_entities::Vehicle>,
        registry_vehicle_seat_flags: &'a Option<i32>,
        registry_vehicle_seat_id: &'a Option<u32>,
        registry_pet_guid: &'a Option<wow_core::ObjectGuid>,
    ) -> Self {
        Self {
            player_race,
            player_class,
            player_level,
            player_skill_fixture,
            represented_enchanting_skill,
            registry_position,
            registry_health,
            registry_max_health,
            registry_alive,
            registry_level,
            registry_transport,
            registry_mount_vehicle_kit,
            registry_vehicle_seat_flags,
            registry_vehicle_seat_id,
            registry_pet_guid,
        }
    }
}

/// Concrete participants for the admitted normal-trainer operation. Each
/// field is borrowed from its canonical owner; no aggregate Session/Hub is
/// retained or exposed to the application.
pub struct AppTrainerCx<'a> {
    pub(super) owner: PlayerAcquisitionOwnerAccessLikeCpp<'a>,
    pub(super) lifecycle: &'a mut SessionLifecycleState,
    pub(super) inventory: &'a mut InventoryState,
    pub(super) spell_state: &'a mut SessionSpellState,
    pub(super) quest_state: &'a mut SessionQuestState,
    pub(super) loot: &'a LootState,
    pub(super) catalogs: TrainerAcquisitionCatalogsLikeCpp<'a>,
    pub(super) consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fixtures: TrainerAcquisitionFixturesLikeCpp<'a>,
}

impl<'a> AppTrainerCx<'a> {
    /// Build the operation context from disjoint session fields and selected
    /// catalog/fixture inputs. This only stores borrows; reads occur at the
    /// same points as the operation they serve.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: PlayerAcquisitionOwnerAccessLikeCpp<'a>,
        lifecycle: &'a mut SessionLifecycleState,
        inventory: &'a mut InventoryState,
        spell_state: &'a mut SessionSpellState,
        quest_state: &'a mut SessionQuestState,
        loot: &'a LootState,
        catalogs: TrainerAcquisitionCatalogsLikeCpp<'a>,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: TrainerAcquisitionFixturesLikeCpp<'a>,
    ) -> Self {
        Self {
            owner,
            lifecycle,
            inventory,
            spell_state,
            quest_state,
            loot,
            catalogs,
            consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
        }
    }
}
