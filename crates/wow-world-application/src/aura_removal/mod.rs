// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Aura application/removal choreography over selected canonical owners.
//! Effect phases remain private and recursive transitions reuse this owner.

mod initial;
mod threat;
mod mount_control;
mod movement_speeds;
mod publication;
mod shapeshift;
mod item_effects;
pub use item_effects::plan_item_set_aura_refresh_with_access_like_cpp;
mod apply;
mod remove;
mod item_scaling;
mod context;
mod feign_death;
mod trainer_views;
pub use context::AuraApplicationCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use context::AuraApplicationFixtureRefsLikeCpp;

pub struct AuraRemovalCxLikeCpp<'a> {
    spell: &'a mut wow_world_spell::SessionSpellState,
    inventory: &'a mut wow_world_inventory::InventoryState,
    player: wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'a>,
    mount: wow_world_core::session::AuraMountControlAccessLikeCpp<'a>,
    stats: wow_world_core::session::AuraStatsAccessBuilderLikeCpp<'a>,
    mount_capabilities: Option<&'a wow_data::MountCapabilityStore>,
    spell_store: Option<&'a std::sync::Arc<wow_data::SpellStore>>,
    classes_store: Option<&'a wow_data::character_progression::ChrClassesStore>,
    difficulty_store: Option<&'a wow_data::DifficultyStore>,
    item_sets: wow_world_core::session::OwnedItemSetAccessLikeCpp<'a>,
    item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
    item_stats: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
    item_effects: Option<&'a std::sync::Arc<wow_data::ItemEffectStore>>,
    form_store: Option<&'a wow_data::SpellShapeshiftFormStore>,
    map_store: Option<&'a wow_data::map::MapStore>,
    item_mod_catalogs: wow_world_inventory::ItemModsCatalogsViewLikeCpp<'a>,
    loot: &'a wow_world_loot::LootState,
    consumer_test: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    shapeshift_form: &'a mut u32,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    scaling_fixture: item_scaling::AuraScalingFixtureRefsLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    mount_evidence: mount_control::MountRemovalEvidenceRefsLikeCpp<'a>,
}

struct RemovedAuraLikeCpp {
    aura: wow_entities::AuraApplicationLikeCpp,
    mounted_aura: bool,
    was_mounted: bool,
}
