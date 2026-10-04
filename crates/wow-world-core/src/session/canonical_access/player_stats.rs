// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow read access for the character-stat application projection.

use crate::session::state::{HubMut, HubRef, SessionCore};
use crate::session::{SessionCatalogs, SessionWorldConfig};
use crate::session::OwnedInventoryAccessLikeCpp;
use std::collections::{BTreeMap, HashMap};
use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_data::{PlayerStatsStore, StatsLimitsLikeCpp};
use wow_entities::PlayerEffectiveCombatStatsLikeCpp;

#[path = "../player_stat_queries.rs"]
mod queries;

mod inventory_scaling;

/// Borrowed fixture inputs consumed by the character-stats projection.
///
/// These are selected fields only: the reward owner can reborrow them at the
/// level-up phase without carrying the session fixture aggregate into App.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct StatsCombatFixtureRefs<'a> {
    player_health_like_cpp: &'a mut u32,
    player_max_health_like_cpp: &'a mut u32,
    player_alive_like_cpp: &'a mut bool,
    represented_player_powers_slot0_like_cpp: &'a mut Option<i32>,
    represented_player_max_powers_slot0_like_cpp: &'a mut Option<i32>,
    represented_player_base_mana_like_cpp: &'a mut i32,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> StatsCombatFixtureRefs<'a> {
    pub(crate) fn health_refs_like_cpp(&self) -> (&u32, &u32, &bool) {
        (&*self.player_health_like_cpp, &*self.player_max_health_like_cpp,
            &*self.player_alive_like_cpp)
    }
    pub(crate) fn reborrow_like_cpp(&mut self) -> StatsCombatFixtureRefs<'_> {
        StatsCombatFixtureRefs {
            player_health_like_cpp: &mut *self.player_health_like_cpp,
            player_max_health_like_cpp: &mut *self.player_max_health_like_cpp,
            player_alive_like_cpp: &mut *self.player_alive_like_cpp,
            represented_player_powers_slot0_like_cpp: &mut *self.represented_player_powers_slot0_like_cpp,
            represented_player_max_powers_slot0_like_cpp: &mut *self.represented_player_max_powers_slot0_like_cpp,
            represented_player_base_mana_like_cpp: &mut *self.represented_player_base_mana_like_cpp,
        }
    }
    pub fn new_like_cpp(
        player_health_like_cpp: &'a mut u32,
        player_max_health_like_cpp: &'a mut u32,
        player_alive_like_cpp: &'a mut bool,
        represented_player_powers_slot0_like_cpp: &'a mut Option<i32>,
        represented_player_max_powers_slot0_like_cpp: &'a mut Option<i32>,
        represented_player_base_mana_like_cpp: &'a mut i32,
    ) -> Self {
        Self {
            player_health_like_cpp,
            player_max_health_like_cpp,
            player_alive_like_cpp,
            represented_player_powers_slot0_like_cpp,
            represented_player_max_powers_slot0_like_cpp,
            represented_player_base_mana_like_cpp,
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub struct StatsAuraFixtureRefs<'a> {
    represented_shapeshift_form_like_cpp: &'a u32,
    player_aura_authority_complete_like_cpp: &'a bool,
    player_spell_hit_aura_authority_tombstoned_like_cpp: &'a bool,
    visible_auras_like_cpp: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    canonical_threat_aura_snapshots_like_cpp:
        &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> StatsAuraFixtureRefs<'a> {
    pub fn new_like_cpp(
        represented_shapeshift_form_like_cpp: &'a u32,
        player_aura_authority_complete_like_cpp: &'a bool,
        player_spell_hit_aura_authority_tombstoned_like_cpp: &'a bool,
        visible_auras_like_cpp: &'a HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        canonical_threat_aura_snapshots_like_cpp:
            &'a HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    ) -> Self {
        Self {
            represented_shapeshift_form_like_cpp,
            player_aura_authority_complete_like_cpp,
            player_spell_hit_aura_authority_tombstoned_like_cpp,
            visible_auras_like_cpp,
            canonical_threat_aura_snapshots_like_cpp,
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub struct StatsFixtureRefs<'a> {
    combat: StatsCombatFixtureRefs<'a>,
    auras: StatsAuraFixtureRefs<'a>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> StatsFixtureRefs<'a> {
    pub fn new_like_cpp(
        combat: StatsCombatFixtureRefs<'a>,
        auras: StatsAuraFixtureRefs<'a>,
    ) -> Self {
        Self { combat, auras }
    }

    /// Reborrow each selected field for another synchronous stats pass.
    pub fn reborrow_like_cpp(&mut self) -> StatsFixtureRefs<'_> {
        StatsFixtureRefs {
            combat: StatsCombatFixtureRefs {
                player_health_like_cpp: &mut *self.combat.player_health_like_cpp,
                player_max_health_like_cpp: &mut *self.combat.player_max_health_like_cpp,
                player_alive_like_cpp: &mut *self.combat.player_alive_like_cpp,
                represented_player_powers_slot0_like_cpp:
                    &mut *self.combat.represented_player_powers_slot0_like_cpp,
                represented_player_max_powers_slot0_like_cpp:
                    &mut *self.combat.represented_player_max_powers_slot0_like_cpp,
                represented_player_base_mana_like_cpp:
                    &mut *self.combat.represented_player_base_mana_like_cpp,
            },
            auras: StatsAuraFixtureRefs {
                represented_shapeshift_form_like_cpp:
                    self.auras.represented_shapeshift_form_like_cpp,
                player_aura_authority_complete_like_cpp:
                    self.auras.player_aura_authority_complete_like_cpp,
                player_spell_hit_aura_authority_tombstoned_like_cpp:
                    self.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                visible_auras_like_cpp: self.auras.visible_auras_like_cpp,
                canonical_threat_aura_snapshots_like_cpp:
                    self.auras.canonical_threat_aura_snapshots_like_cpp,
            },
        }
    }

    /// Lend the current health/max-health/alive participants to a selected
    /// read-only consumer (XP/rest/resurrection projection) at its own phase.
    pub fn vitals_fixture_refs_like_cpp(&self) -> (&u32, &u32, &bool) {
        self.combat.health_refs_like_cpp()
    }

}

/// Borrowed, operation-specific access used while projecting Player stats.
/// The canonical Player and fixture state remain owned by Core.
pub struct PlayerStatsAccessLikeCpp<'a> {
    core: &'a SessionCore,
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: StatsFixtureRefs<'a>,
}

fn represented_total_stat_multiplier_from_snapshot_like_cpp(
    catalogs: &SessionCatalogs,
    spell_store: &wow_data::SpellStore,
    visible_auras: &HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    stat: usize,
    uses_misc_value_b: bool,
) -> f32 {
    let aura_type = wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE;
    let mut multiplier = 1.0f32;
    let mut same_effect_spell_groups = BTreeMap::<u32, i32>::new();

    for aura in visible_auras.values() {
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };

        for effect in spell.effects().iter().filter(|effect| {
            1u32.checked_shl(effect.effect_index)
                .is_some_and(|bit| aura.effect_mask & bit != 0)
                && effect.effect_aura == aura_type
                && if uses_misc_value_b {
                    effect.effect_misc_value_2 == 0
                        || effect.effect_misc_value_2 & (1 << stat) != 0
                } else {
                    effect.effect_misc_value_1 == -1
                        || effect.effect_misc_value_1 == stat as i32
                }
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| u32::from(represented.effect_index) == effect.effect_index)
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());

            let same_effect_group = catalogs
                .spell_spell_group_map_bounds_like_cpp(aura.spell_id as u32)
                .iter()
                .copied()
                .find(|group_id| {
                    catalogs
                        .same_effect_stack_rule_aura_types_like_cpp(*group_id)
                        .is_some_and(|aura_types| aura_types.contains(&aura_type))
                });
            if let Some(group_id) = same_effect_group {
                same_effect_spell_groups
                    .entry(group_id)
                    .and_modify(|current| {
                        if current.unsigned_abs() < amount.unsigned_abs() {
                            *current = amount;
                        }
                    })
                    .or_insert(amount);
            } else {
                multiplier += multiplier * amount as f32 / 100.0;
            }
        }
    }

    for amount in same_effect_spell_groups.into_values() {
        multiplier += multiplier * amount as f32 / 100.0;
    }
    multiplier
}

impl SessionCore {
    pub fn apply_shapeshift_base_attack_times_like_cpp(
        &self, regular: Option<[u32; 3]>, combat_round_time: Option<f32>,
    ) -> bool {
        self
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                let (base, offhand, ranged) = match combat_round_time {
                    Some(round_time) => (round_time as u32, round_time as u32, 2_000),
                    None => {
                        let Some(regular) = regular else {
                            return;
                        };
                        // C++ `Player::SetRegularAttackTime` only writes an attack
                        // whose equipped weapon declares a delay; every other attack
                        // keeps its current time.
                        let current = unit.base_attack_speed();
                        (
                            if regular[0] > 0 {
                                regular[0]
                            } else {
                                current[0]
                            },
                            if regular[1] > 0 {
                                regular[1]
                            } else {
                                current[1]
                            },
                            if regular[2] > 0 {
                                regular[2]
                            } else {
                                current[2]
                            },
                        )
                    }
                };
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::BaseAttack,
                    base,
                );
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::OffAttack,
                    offhand,
                );
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::RangedAttack,
                    ranged,
                );
            })
            .is_some()
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn player_stats_access_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        PlayerStatsAccessLikeCpp {
            core: self,
            catalogs,
            config,
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_stats_access_with_fixture_refs_like_cpp<'a>(
        &'a self,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        player_race: &'a u8,
        player_class: &'a u8,
        player_level: &'a u8,
        fixtures: StatsFixtureRefs<'a>,
    ) -> PlayerStatsAccessLikeCpp<'a> {
        PlayerStatsAccessLikeCpp {
            core: self,
            catalogs,
            config,
            player_race,
            player_class,
            player_level,
            fixtures,
        }
    }
}

impl PlayerStatsAccessLikeCpp<'_> {
    /// Reborrow the selected inputs without reading canonical or fixture state.
    pub fn reborrow_like_cpp(&mut self) -> PlayerStatsAccessLikeCpp<'_> {
        PlayerStatsAccessLikeCpp {
            core: self.core,
            catalogs: self.catalogs,
            config: self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_race: self.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_class: self.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level: self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: self.fixtures.reborrow_like_cpp(),
        }
    }

    pub fn apply_shapeshift_base_attack_times_like_cpp(
        &self, regular: Option<[u32; 3]>, combat_round_time: Option<f32>,
    ) -> bool {
        self.core.apply_shapeshift_base_attack_times_like_cpp(regular, combat_round_time)
    }

    /// Lend the mutable-vitals participants to the Registry responsibility.
    ///
    /// This is a typed projection for the final registry publication, not a
    /// general-purpose fixture view: the returned builder keeps the values in
    /// `PlayerStatsAccessLikeCpp`, so the caller does not have to reborrow the
    /// session fixtures that this capability already holds mutably.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn registry_sync_inputs_like_cpp(&self) -> crate::session::RegistrySyncInputs<'_> {
        let (health, max_health, alive) = self.fixtures.combat.health_refs_like_cpp();
        crate::session::RegistrySyncInputs::new_like_cpp(health, max_health, alive)
    }

    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn player_race_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_race_with_fixture_like_cpp(self.player_race)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_race_with_fixture_like_cpp()
        }
    }

    pub fn player_class_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_class_with_fixture_like_cpp(self.player_class)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_class_with_fixture_like_cpp()
        }
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core
                .player_level_with_fixture_like_cpp(self.player_level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_level_with_fixture_like_cpp()
        }
    }

    pub fn player_stats_store_like_cpp(&self) -> Option<&PlayerStatsStore> {
        self.catalogs.player_stats.as_deref()
    }

    pub fn player_class_attack_power_coefficients_like_cpp(
        &self,
        class: u8,
    ) -> Option<(u8, u8, u8)> {
        self.catalogs
            .player_class_attack_power_coefficients_like_cpp(class)
    }

    pub fn combat_rating_multiplier_like_cpp(&self, level: u8, rating: u32) -> f32 {
        self.catalogs.combat_rating_multiplier_like_cpp(level, rating)
    }

    pub fn stats_limits_like_cpp(&self) -> StatsLimitsLikeCpp {
        self.config.stats_limits_like_cpp
    }

    pub fn canonical_player_parry_block_snapshot_like_cpp(&self) -> (bool, bool) {
        self.core.canonical_player_parry_block_snapshot_like_cpp()
    }

    pub fn canonical_player_power_snapshot_like_cpp(
        &self,
        power_type: PowerType,
    ) -> Option<(i32, i32)> {
        self.core.canonical_player_power_snapshot_like_cpp(power_type)
    }

    pub fn canonical_player_effective_combat_stats_like_cpp(
        &self,
    ) -> Option<PlayerEffectiveCombatStatsLikeCpp> {
        self.core.canonical_player_effective_combat_stats_like_cpp()
    }

    pub fn resolved_player_vitals_like_cpp(&self) -> Option<(u32, u32, bool)> {
        self.core.resolved_player_vitals_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.fixtures.combat.player_health_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.fixtures.combat.player_max_health_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.fixtures.combat.player_alive_like_cpp,
        )
    }

    fn resolved_player_visible_auras_like_cpp(
        &self,
    ) -> Option<HashMap<u8, wow_entities::AuraApplicationLikeCpp>> {
        self.core
            .player_aura_subsystem_snapshot_with_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.auras.player_aura_authority_complete_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.auras.visible_auras_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            )
            .map(|auras| auras.runtime_applications_like_cpp().clone())
    }

    pub fn resolved_aura_effects_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(crate::session::player_aura_effects_by_spell_aura_type_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    pub fn resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(crate::session::aura_effects_with_misc_values_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    pub fn resolved_aura_effects_with_spell_and_misc_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(crate::session::aura_effects_with_spell_and_misc_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    pub fn resolved_aura_effect_amounts_by_spell_like_cpp(
        &self,
        aura_type: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let spell_store = self.catalogs.spell_store()?;
        Some(crate::session::aura_effect_amounts_by_spell_from_snapshot_like_cpp(
            &visible_auras,
            spell_store,
            aura_type,
        ))
    }

    pub fn resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<f32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| {
                effects
                    .into_iter()
                    .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
                    .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
            })
    }

    pub fn resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
        &self,
        aura_type: i32,
        misc_value: i32,
    ) -> Option<i32> {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
            .map(|effects| {
                effects
                    .into_iter()
                    .filter(|(effect_misc_value, _)| *effect_misc_value == misc_value)
                    .map(|(_, amount)| amount)
                    .sum()
            })
    }

    pub fn resolved_represented_total_stat_multipliers_like_cpp(
        &self,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier = self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, true)?;
        }
        Some(multipliers)
    }

    pub fn resolved_represented_total_stat_buff_multipliers_like_cpp(
        &self,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            *multiplier = self.resolved_represented_total_stat_multiplier_for_stat_like_cpp(stat, false)?;
        }
        Some(multipliers)
    }

    fn resolved_represented_total_stat_multiplier_for_stat_like_cpp(
        &self,
        stat: usize,
        uses_misc_value_b: bool,
    ) -> Option<f32> {
        let spell_store = self.catalogs.spell_store()?;
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        Some(represented_total_stat_multiplier_from_snapshot_like_cpp(
            self.catalogs,
            spell_store,
            &visible_auras,
            stat,
            uses_misc_value_b,
        ))
    }

    pub fn represented_shapeshift_combat_round_time_like_cpp(&self) -> Option<f32> {
        self.core.represented_shapeshift_combat_round_time_with_fixture_refs_like_cpp(
            self.catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.auras.represented_shapeshift_form_like_cpp,
        )
    }

    pub fn item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        self.core
            .owned_item_modifiers_access_like_cpp()
            .item_modifier_runtime_snapshot_like_cpp()
    }

    pub fn owned_inventory_access_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn represented_aura_spell_fits_weapon_like_cpp(
        &self,
        spell_id: i32,
        weapon_item_id: Option<u32>,
    ) -> bool {
        self.catalogs
            .represented_aura_spell_fits_weapon_like_cpp(spell_id, weapon_item_id)
    }

    pub fn spell_spell_group_map_bounds_like_cpp(&self, spell_id: u32) -> &[u32] {
        self.catalogs.spell_spell_group_map_bounds_like_cpp(spell_id)
    }

    pub fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&std::collections::BTreeSet<i32>> {
        self.catalogs
            .same_effect_stack_rule_aura_types_like_cpp(group_id)
    }

    pub fn spell_item_enchantment_store_like_cpp(
        &self,
    ) -> Option<&std::sync::Arc<wow_data::SpellItemEnchantmentStore>> {
        self.catalogs.spell_item_enchantment_store()
    }

    pub fn represented_weapon_delay_seconds_like_cpp(&self, item_id: u32) -> f32 {
        self.catalogs
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.weapon_template(item_id))
            .map(|weapon| f32::from(weapon.item_delay) / 1000.0)
            .unwrap_or(0.0)
    }

    pub fn replace_player_effective_combat_stats_like_cpp(
        &self,
        stats: PlayerEffectiveCombatStatsLikeCpp,
    ) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(stats);
        });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

}

impl HubRef<'_> {
    fn resolved_represented_total_stat_multipliers_with_misc_like_cpp(
        &self,
        uses_misc_value_b: bool,
    ) -> Option<[f32; 5]> {
        let mut multipliers = [1.0; 5];
        for (stat, multiplier) in multipliers.iter_mut().enumerate() {
            let spell_store = self.catalogs.spell_store()?;
            let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
            *multiplier = represented_total_stat_multiplier_from_snapshot_like_cpp(
                self.catalogs,
                spell_store,
                &visible_auras,
                stat,
                uses_misc_value_b,
            );
        }
        Some(multipliers)
    }

    pub fn resolved_represented_total_stat_multipliers_like_cpp(&self) -> Option<[f32; 5]> {
        self.resolved_represented_total_stat_multipliers_with_misc_like_cpp(true)
    }

    pub fn resolved_represented_total_stat_buff_multipliers_like_cpp(&self) -> Option<[f32; 5]> {
        self.resolved_represented_total_stat_multipliers_with_misc_like_cpp(false)
    }
}

impl HubMut<'_> {
    pub(crate) fn player_stats_access_like_cpp(&mut self) -> PlayerStatsAccessLikeCpp<'_> {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let core = &*self.core;
            let catalogs = self.catalogs;
            let config = self.config;
            let fixtures = &mut *self.fixtures;
            core.player_stats_access_with_fixture_refs_like_cpp(
                catalogs,
                config,
                &fixtures.identity.player_race,
                &fixtures.identity.player_class,
                &fixtures.identity.player_level,
                StatsFixtureRefs::new_like_cpp(
                    StatsCombatFixtureRefs::new_like_cpp(
                        &mut fixtures.combat.player_health_like_cpp,
                        &mut fixtures.combat.player_max_health_like_cpp,
                        &mut fixtures.combat.player_alive_like_cpp,
                        &mut fixtures.combat.represented_player_powers_like_cpp[0],
                        &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
                        &mut fixtures.combat.represented_player_base_mana_like_cpp,
                    ),
                    StatsAuraFixtureRefs::new_like_cpp(
                        &fixtures.auras.represented_shapeshift_form_like_cpp,
                        &fixtures.auras.player_aura_authority_complete_like_cpp,
                        &fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                        &fixtures.auras.visible_auras,
                        &fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                    ),
                ),
            )
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core
                .player_stats_access_like_cpp(self.catalogs, self.config)
        }
    }
}

impl PlayerStatsAccessLikeCpp<'_> {
    pub fn sync_canonical_player_health_like_cpp(
        &mut self,
        health: u32,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let health = health.min(max_health);
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            if health == 0 {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
            } else if matches!(
                player.unit().death_state(),
                wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
            ) {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
            }
            player.unit_mut().set_max_health(u64::from(max_health));
            player.unit_mut().set_health(u64::from(health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let result = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_death_state(if health == 0 {
                    wow_constants::DeathState::Corpse
                } else {
                    wow_constants::DeathState::Alive
                });
                player.unit_mut().set_max_health(u64::from(max_health));
                player.unit_mut().set_health(u64::from(health));
                (
                    player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                    player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                )
            })
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let result = canonical;
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let (current, max) = result.unwrap_or((health, max_health));
            *self.fixtures.combat.player_health_like_cpp = current;
            *self.fixtures.combat.player_max_health_like_cpp = max;
            *self.fixtures.combat.player_alive_like_cpp = current > 0;
        }
        result
    }

    pub fn sync_canonical_player_max_health_like_cpp(
        &mut self,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(u64::from(max_health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let result = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core
                .mutate_canonical_player_like_cpp(|player| {
                    player.unit_mut().set_max_health(u64::from(max_health));
                    (
                        player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                        player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                    )
                })
                .or_else(|| {
                    Some((
                        (*self.fixtures.combat.player_health_like_cpp).min(max_health),
                        max_health,
                    ))
                })
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let result = canonical;
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some((current, max)) = result {
            *self.fixtures.combat.player_health_like_cpp = current;
            *self.fixtures.combat.player_max_health_like_cpp = max;
            *self.fixtures.combat.player_alive_like_cpp = current > 0;
        }
        result
    }

    pub fn sync_canonical_player_primary_power_max_like_cpp(
        &mut self,
        power_type: PowerType,
        max: i32,
        base_mana: i32,
    ) -> Option<(i32, i32)> {
        let result = self.core.with_owned_player_mut_for_power_like_cpp(|player| {
            if player.unit().get_power_index(power_type).is_none() {
                player.set_power_index(power_type, Some(0));
            }
            player.unit_mut().set_display_power(power_type);
            player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
            player.unit_mut().set_max_power(power_type, max.max(0));
            (
                player.unit().get_power(power_type),
                player.unit().get_max_power(power_type),
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some((current, max)) = result.or_else(|| {
            self.core.player_handle_like_cpp.is_none().then_some((
                (*self.fixtures.combat.represented_player_powers_slot0_like_cpp).unwrap_or(0),
                max.max(0),
            ))
        }) {
            *self.fixtures.combat.represented_player_base_mana_like_cpp = base_mana.max(0);
            *self.fixtures.combat.represented_player_powers_slot0_like_cpp =
                Some(current.max(0));
            *self.fixtures.combat.represented_player_max_powers_slot0_like_cpp =
                Some(max.max(0));
        }
        result
    }

    pub fn sync_canonical_player_primary_power_like_cpp(
        &mut self,
        power_type: PowerType,
        current: i32,
        max: i32,
        base_mana: i32,
    ) -> bool {
        let synced = self
            .core
            .with_owned_player_mut_for_power_like_cpp(|player| {
                for raw_power in 0..=25 {
                    player.set_power_index(crate::session::power_type_from_u8_like_cpp(raw_power), None);
                }
                player.set_power_index(power_type, Some(0));
                player.unit_mut().set_display_power(power_type);
                player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
                player.unit_mut().set_max_power(power_type, max.max(0));
                player.unit_mut().set_power(power_type, current.max(0));
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if synced || self.core.player_handle_like_cpp.is_none() {
            *self.fixtures.combat.represented_player_base_mana_like_cpp = base_mana.max(0);
            *self.fixtures.combat.represented_player_powers_slot0_like_cpp =
                Some(current.max(0));
            *self.fixtures.combat.represented_player_max_powers_slot0_like_cpp =
                Some(max.max(0));
        }
        synced
    }

}
