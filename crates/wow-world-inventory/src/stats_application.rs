// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character stat application operations.

use crate::InventoryState;
use crate::plan_player_stats;
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_data::PlayerStatsStore;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::PlayerStatWeaponFacts;
use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_world_core::session::{
    PacketPublicationAccessLikeCpp, PlayerStatApplicationCause, PlayerStatComputation,
    PlayerStatComputationRequest, PlayerStatsAccessLikeCpp,
    primary_power_type_for_player_class_like_cpp,
};

impl InventoryState {
    fn calculate_player_stats_with_access(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        request: &PlayerStatComputationRequest<'_>,
    ) -> Option<PlayerStatComputation> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            let item_bonuses = self
                .player_item_test_fixture_like_cpp
                .represented_item_modifier_runtime_like_cpp
                .bonuses_like_cpp();
            let weapons = self.fixture_weapon_facts();
            let facts = access.fixture_player_stat_facts(item_bonuses, weapons)?;
            let catalogs = access.stat_catalogs();
            return plan_player_stats(&facts, &catalogs, request);
        }

        access.calculate_owned_player_stats(plan_player_stats, request)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    fn fixture_weapon_facts(&self) -> [Option<PlayerStatWeaponFacts>; 3] {
        use wow_entities::{
            EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED,
        };

        [
            EQUIPMENT_SLOT_MAINHAND,
            EQUIPMENT_SLOT_OFFHAND,
            EQUIPMENT_SLOT_RANGED,
        ]
        .map(|slot| {
            let inventory_item = self
                .player_item_test_fixture_like_cpp
                .inventory_items
                .get(&slot)?;
            let item = self.inventory_item_objects.get(&inventory_item.guid)?;
            if item.is_broken() {
                return None;
            }
            let enchantment_ids = item
                .data()
                .enchantments
                .iter()
                .filter_map(|enchantment| u32::try_from(enchantment.id).ok())
                .collect();
            Some(PlayerStatWeaponFacts::from_parts(
                inventory_item.entry_id,
                enchantment_ids,
            ))
        })
    }
}

/// The borrowed participants for a character-stat application operation.
/// Catalog and canonical-player access remain in Core. Inventory supplies one
/// pure stats calculation; this adapter applies its owned result and publishes
/// the existing health, power, packet and trace effects in their current order.
pub struct CharacterStatsApplicationCxLikeCpp<'a> {
    player: PlayerStatsAccessLikeCpp<'a>,
    inventory: &'a InventoryState,
    publication: PacketPublicationAccessLikeCpp<'a>,
}

impl<'a> CharacterStatsApplicationCxLikeCpp<'a> {
    pub fn new(
        player: PlayerStatsAccessLikeCpp<'a>,
        inventory: &'a InventoryState,
        publication: PacketPublicationAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            player,
            inventory,
            publication,
        }
    }

    pub fn represented_player_gear_stats_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        self.inventory
            .represented_player_gear_stats_with_access_like_cpp(&self.player)
    }

    pub fn level_up_stat_deltas_like_cpp(&self, new_level: u8) -> Option<(i32, [i32; 5])> {
        let store = self.player.player_stats_store_like_cpp()?;
        let race = self.player.player_race_like_cpp();
        let class = self.player.player_class_like_cpp();
        let old_level = self.player.player_level_like_cpp();
        level_up_stat_deltas_like_cpp(store, race, class, old_level, new_level)
    }

    /// Build the C++ `Player::UpdateAllStats` value projection from the
    /// canonical player-owned item accumulator and current session inputs.
    pub fn player_stat_system_projection_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        let request = PlayerStatComputationRequest::new(Some((race, class, level)), Some(gear));
        self.inventory
            .calculate_player_stats_with_access(&self.player, &request)
            .map(|calculation| calculation.projection)
    }
}

impl CharacterStatsApplicationCxLikeCpp<'_> {
    /// Check that the current canonical owner and its required catalogs can
    /// produce a stats result without applying or publishing it.
    pub fn can_calculate_current_stats(&self) -> bool {
        let request = PlayerStatComputationRequest::new(None, None);
        self.inventory
            .calculate_player_stats_with_access(&self.player, &request)
            .is_some()
    }

    /// Calculate and apply the canonical derived-stat snapshot without
    /// producing an immediate synthetic stat packet.
    pub fn apply_stats_for_values(
        &self,
        cause: PlayerStatApplicationCause,
    ) -> Option<PlayerStatComputation> {
        let request = PlayerStatComputationRequest::new(None, None);
        self.player
            .calculate_and_apply_stats_for_values(plan_player_stats, &request, cause)
    }

    /// Build the complete stat-update packet values while preserving current
    /// health and primary power around the canonical maximum updates.
    pub fn player_stat_changes_like_cpp(
        &mut self,
    ) -> Option<(
        wow_core::ObjectGuid,
        wow_packet::packets::update::PlayerStatChanges,
    )> {
        self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
    }

    pub fn player_stat_changes_with_represented_item_bonuses_like_cpp(
        &mut self,
        _include_represented_item_bonuses: bool,
    ) -> Option<(
        wow_core::ObjectGuid,
        wow_packet::packets::update::PlayerStatChanges,
    )> {
        use wow_constants::PowerType;
        use wow_packet::packets::update::PlayerStatChanges;

        let player_guid = self.player.player_guid_like_cpp()?;
        let race = self.player.player_race_like_cpp();
        let class = self.player.player_class_like_cpp();
        let level = self.player.player_level_like_cpp();
        if race == 0 || class == 0 || level == 0 {
            return None;
        }

        let request = PlayerStatComputationRequest::new(None, None);
        let calculation = self
            .inventory
            .calculate_player_stats_with_access(&self.player, &request)?;
        let gear = calculation.gear;
        let projection = calculation.projection;
        let effective = calculation.effective;
        let weapon_damage = effective.weapon_damage;
        self.player
            .replace_player_effective_combat_stats_like_cpp(effective);
        let computed_max_health_u32 = max_health_u32_like_cpp(projection.max_health);
        let (health, max_health_for_update) = self
            .player
            .sync_canonical_player_max_health_like_cpp(computed_max_health_u32)?;
        let health = i64::from(health);
        let max_health = i64::from(max_health_for_update);

        // Power for slot 0 (mana/rage/energy/runic). Keep current power from
        // the runtime player and update only the max, like C++ `SetMaxPower`.
        let primary_power_type = primary_power_type_for_player_class_like_cpp(class);
        let computed_max_power0 = primary_max_power_for_class_like_cpp(class, projection.max_mana);
        let base_mana = if primary_power_type == PowerType::Mana {
            projection.base_mana
        } else {
            0
        };
        let (power0, max_power0) = self
            .player
            .sync_canonical_player_primary_power_max_like_cpp(
                primary_power_type,
                computed_max_power0,
                base_mana,
            )
            .or_else(|| {
                self.player
                    .canonical_player_power_snapshot_like_cpp(primary_power_type)
                    .map(|(current, _)| {
                        (current.max(0).min(computed_max_power0), computed_max_power0)
                    })
            })
            .unwrap_or((computed_max_power0, computed_max_power0));

        // The packet adapter consumes the just-published Player snapshot so
        // combat and VALUES publication cannot derive different expertise.
        // Keep the raw calculation only for test/early-login sessions that
        // have no canonical Player owner yet.
        let (mainhand_expertise, offhand_expertise) = self
            .player
            .canonical_player_effective_combat_stats_like_cpp()
            .map(|stats| (stats.mainhand_expertise, stats.offhand_expertise))
            .unwrap_or_else(|| {
                let rating = (gear.combat_ratings[23] as f32
                    * self.player.combat_rating_multiplier_like_cpp(level, 23))
                .trunc()
                .max(0.0);
                (rating, rating)
            });

        // ── Shield block value (from STR, for shield classes) ──
        let mut shield_block_value = match class {
            1 | 2 | 7 => ((projection.stats[0] as f32 * 0.5 - 10.0).max(0.0)) as i32,
            _ => 0,
        };
        shield_block_value = shield_block_value
            .saturating_add(gear.shield_block_base_mod)
            .saturating_add(i32::try_from(gear.shield_block_value).unwrap_or(i32::MAX));
        // C++ `Player::UpdateManaRegen` stores MP5 bonuses as per-second
        // values in both normal and interrupted flat regen fields.
        let represented_mana_regen_per_second = gear.mana_regen_bonus as f32 / 5.0;
        let (mana_regen, mana_regen_combat, mana_regen_mp5) = self
            .player
            .canonical_player_effective_combat_stats_like_cpp()
            .map(|stats| {
                (
                    stats.mana_regen,
                    stats.mana_regen_combat,
                    stats.mana_regen_mp5,
                )
            })
            .unwrap_or_else(|| {
                (
                    self.player
                        .mana_regen_from_stats_like_cpp(level, class, projection.stats)
                        + represented_mana_regen_per_second,
                    represented_mana_regen_per_second,
                    0.0,
                )
            });

        let changes = PlayerStatChanges {
            health,
            max_health,
            min_damage: weapon_damage[0][0],
            max_damage: weapon_damage[0][1],
            base_mana,
            base_health: projection.create_health,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_mod_neg: 0,
            attack_power_multiplier: projection.attack_power_multiplier,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
            min_ranged_damage: weapon_damage[2][0],
            max_ranged_damage: weapon_damage[2][1],
            power0,
            max_power0,
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            armor: projection.armor,
            combat_ratings: gear.combat_ratings,
            mod_damage_done_pos: projection.mod_damage_done_pos,
            mod_damage_done_neg: projection.mod_damage_done_neg,
            mod_healing_done_pos: projection.mod_healing_done_pos,
            mod_damage_done_percent: projection.mod_damage_done_percent,
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            parry_pct: projection.parry_pct,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            mana_regen,
            mana_regen_combat,
            mana_regen_mp5,
            mainhand_expertise,
            offhand_expertise,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            dodge_from_attr: projection.dodge_from_attr,
            parry_from_attr: projection.parry_from_attr,
            offhand_crit_pct: projection.offhand_crit_pct,
            shield_block: shield_block_value,
            shield_block_crit_pct: 0.0,
            mod_healing_pct: 1.0,
            mod_healing_done_pct: projection.mod_healing_done_percent,
            mod_target_resistance: projection.mod_target_resistance,
            mod_target_physical_resistance: projection.mod_target_physical_resistance,
            versatility_bonus: projection.versatility_bonus,
            override_spell_power_by_ap_percent: projection.override_spell_power_by_ap_percent,
            override_ap_by_spell_power_percent: projection.override_ap_by_spell_power_percent,
            mod_periodic_healing_pct: 1.0,
            mod_spell_power_pct: 1.0,
        };

        if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
            tracing::info!(
                guid = ?player_guid,
                power_type = ?primary_power_type,
                current_power0 = power0,
                max_power0,
                base_mana,
                "RUST_STAT_POWER_UPDATE"
            );
        }

        tracing::debug!(
            "Stat update for {:?}: HP={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} SP={} Crit={:.1}% SCrit={:.1}% Dodge={:.1}% Parry={:.1}% Exp={:.1} ManaRegen={:.1}",
            player_guid,
            max_health,
            projection.attack_power,
            projection.stats,
            projection.armor,
            gear.spell_power,
            projection.crit_pct,
            projection.spell_crit_pct[0],
            projection.dodge_pct,
            projection.parry_pct,
            mainhand_expertise,
            mana_regen
        );

        Some((player_guid, changes))
    }

    /// Recalculate stats and publish the C++ `UpdateAllStats` VALUES update.
    pub fn send_stat_update_like_cpp(&mut self) -> bool {
        if self.player.defer_stat_packet_for_current_player() == Some(true) {
            return self
                .apply_stats_for_values(PlayerStatApplicationCause::AllStats)
                .is_some();
        }

        let Some((player_guid, changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return false;
        };
        let update = wow_packet::packets::update::UpdateObject::player_stat_update(
            player_guid,
            self.player.player_map_id_like_cpp(),
            changes,
        );
        self.publication.send_packet(&update)
    }

    /// C++ `AuraEffect::HandleModTotalPercentStat` (`SpellAuraEffects.cpp:3656`).
    /// Ability auras selecting stamina preserve the pre-change health
    /// percentage; other total-stat auras use ordinary `SetMaxHealth` clamping.
    pub fn send_total_stat_percentage_update_like_cpp(&mut self, preserve_health_pct: bool) {
        let Some((health_before, max_health_before, _)) =
            self.player.resolved_player_vitals_like_cpp()
        else {
            return;
        };
        let max_health_before = max_health_before.max(1);
        let zero_health = health_before == 0;
        let Some((player_guid, mut changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return;
        };

        if preserve_health_pct {
            let max_health_after = max_health_u32_like_cpp(changes.max_health);
            let health_pct = health_before as f32 * 100.0 / max_health_before as f32;
            let restored = (max_health_after as f32 * health_pct / 100.0) as u32;
            let restored = restored.max(if zero_health { 0 } else { 1 });
            let _ = self
                .player
                .sync_canonical_player_health_like_cpp(restored, max_health_after);
            changes.health = i64::from(restored);
        }

        let update = wow_packet::packets::update::UpdateObject::player_stat_update(
            player_guid,
            self.player.player_map_id_like_cpp(),
            changes,
        );
        self.publication.send_packet(&update);
    }

    /// C++ `Player::GiveLevel` refills health and powers carrying
    /// `PowerTypeFlags::SetToMaxOnLevelUp` after `UpdateAllStats`.
    /// The represented 3.4.3 path refills mana while rage, energy and runic
    /// power retain their current values.
    pub fn send_level_up_stat_update_like_cpp(&mut self) {
        use wow_constants::PowerType;

        let Some((player_guid, mut changes)) = self.player_stat_changes_like_cpp() else {
            return;
        };
        let max_health = max_health_u32_like_cpp(changes.max_health);
        let _ = self
            .player
            .sync_canonical_player_health_like_cpp(max_health, max_health);
        changes.health = i64::from(max_health);

        if primary_power_type_for_player_class_like_cpp(self.player.player_class_like_cpp())
            == PowerType::Mana
        {
            changes.power0 = changes.max_power0;
            let _ = self.player.sync_canonical_player_primary_power_like_cpp(
                PowerType::Mana,
                changes.power0,
                changes.max_power0,
                changes.base_mana,
            );
        }

        let update = wow_packet::packets::update::UpdateObject::player_stat_update(
            player_guid,
            self.player.player_map_id_like_cpp(),
            changes,
        );
        self.publication.send_packet(&update);
    }

    /// Recalculate and send the login VALUES update after loaded auras and
    /// item modifiers have been installed.
    pub fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self) {
        let Some((player_guid, changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return;
        };
        let update = wow_packet::packets::update::UpdateObject::player_stat_update(
            player_guid,
            self.player.player_map_id_like_cpp(),
            changes,
        );
        self.publication.send_packet(&update);
    }

    fn player_login_combat_projection_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
    ) -> Option<(
        RepresentedPlayerGearStatsLikeCpp,
        PlayerStatSystemProjectionLikeCpp,
        [[f32; 2]; 3],
        [i32; 6],
    )> {
        let request = PlayerStatComputationRequest::new(Some((race, class, level)), None);
        let calculation = self
            .inventory
            .calculate_player_stats_with_access(&self.player, &request)?;
        let gear = calculation.gear;
        let projection = calculation.projection;
        let effective = calculation.effective;
        let weapon_damage = effective.weapon_damage;
        let school_resistances = std::array::from_fn(|index| effective.resistances[index + 1]);
        self.player
            .replace_player_effective_combat_stats_like_cpp(effective);
        Some((gear, projection, weapon_damage, school_resistances))
    }

    pub fn player_login_combat_stats_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        saved_health: Option<u32>,
        saved_power0: i32,
    ) -> Option<(wow_packet::packets::update::PlayerCombatStats, i32, i32)> {
        let (gear, projection, weapon_damage, school_resistances) =
            self.player_login_combat_projection_like_cpp(race, class, level)?;
        let min_damage = weapon_damage[0][0];
        let max_damage = weapon_damage[0][1];
        let min_ranged_damage = weapon_damage[2][0];
        let max_ranged_damage = weapon_damage[2][1];
        let combat = wow_packet::packets::update::PlayerCombatStats {
            health: saved_health
                .map(|health| i64::from(health.min(max_health_u32_like_cpp(projection.max_health))))
                .unwrap_or_else(|| i64::from(max_health_u32_like_cpp(projection.max_health))),
            max_health: projection.max_health,
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_armor: projection.armor,
            school_resistances,
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_multiplier: projection.attack_power_multiplier,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
            min_damage,
            max_damage,
            min_ranged_damage,
            max_ranged_damage,
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            dodge_from_attr: projection.dodge_from_attr,
            parry_pct: projection.parry_pct,
            parry_from_attr: projection.parry_from_attr,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            offhand_crit_pct: projection.offhand_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            combat_ratings: gear.combat_ratings,
            mod_damage_done_pos: projection.mod_damage_done_pos,
            mod_damage_done_neg: projection.mod_damage_done_neg,
            mod_healing_done_pos: projection.mod_healing_done_pos,
            mod_damage_done_percent: projection.mod_damage_done_percent,
            mod_healing_done_pct: projection.mod_healing_done_percent,
            mod_target_resistance: projection.mod_target_resistance,
            mod_target_physical_resistance: projection.mod_target_physical_resistance,
            versatility_bonus: projection.versatility_bonus,
            override_spell_power_by_ap_percent: projection.override_spell_power_by_ap_percent,
            override_ap_by_spell_power_percent: projection.override_ap_by_spell_power_percent,
        };
        let max_power0 = primary_max_power_for_class_like_cpp(class, combat.max_mana);
        Some((
            combat,
            projection.base_mana,
            saved_power0.clamp(0, max_power0.max(0)),
        ))
    }
}

pub fn max_health_u32_like_cpp(max_health: i64) -> u32 {
    max_health.max(1).min(i64::from(u32::MAX)) as u32
}

/// Builds the character-stat application context from the session hub and the
/// session inventory.
///
/// The World shell and the moved character-creation body share this
/// construction so the catalog, config and stat-fixture access is assembled
/// once (#1263 F5).
pub fn stats_application_cx_from_hub_like_cpp<'a>(
    hub: wow_world_core::session::HubMut<'a>,
    inventory: &'a mut InventoryState,
) -> CharacterStatsApplicationCxLikeCpp<'a> {
    let core = &*hub.core;
    let publication = core.packet_publication_access_like_cpp();
    #[cfg(any(test, feature = "test-fixtures"))]
    let player = core.player_stats_access_with_fixture_refs_like_cpp(
        hub.catalogs,
        hub.config,
        &hub.fixtures.identity.player_race,
        &hub.fixtures.identity.player_class,
        &hub.fixtures.identity.player_level,
        wow_world_core::session::StatsFixtureRefs::new_like_cpp(
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut hub.fixtures.combat.player_health_like_cpp,
                &mut hub.fixtures.combat.player_max_health_like_cpp,
                &mut hub.fixtures.combat.player_alive_like_cpp,
                &mut hub.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut hub.fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut hub.fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                &hub.fixtures.auras.represented_shapeshift_form_like_cpp,
                &hub.fixtures.auras.player_aura_authority_complete_like_cpp,
                &hub.fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &hub.fixtures.auras.visible_auras,
                &hub.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            ),
        ),
    );
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let player = core.player_stats_access_like_cpp(hub.catalogs, hub.config);
    CharacterStatsApplicationCxLikeCpp::new(player, &*inventory, publication)
}

pub fn primary_max_power_for_class_like_cpp(class_id: u8, max_mana: i64) -> i32 {
    match class_id {
        1 | 6 => 1_000,
        4 => 100,
        _ => max_mana.max(0).min(i64::from(i32::MAX)) as i32,
    }
}

/// C++ `Player::GiveLevel`'s base-mana and primary-stat row deltas.
///
/// The caller supplies the current identity and levels at the same point it
/// reads them today; this operation only compares the two canonical data rows.
pub fn level_up_stat_deltas_like_cpp(
    store: &PlayerStatsStore,
    race: u8,
    class: u8,
    old_level: u8,
    new_level: u8,
) -> Option<(i32, [i32; 5])> {
    let old = store.get(race, class, old_level)?;
    let new = store.get(race, class, new_level)?;
    let old_stats = old.primary_stats_like_cpp();
    let new_stats = new.primary_stats_like_cpp();
    Some((
        i32::try_from(new.base_mana)
            .unwrap_or(i32::MAX)
            .saturating_sub(i32::try_from(old.base_mana).unwrap_or(i32::MAX)),
        std::array::from_fn(|index| {
            i32::from(new_stats[index]).saturating_sub(i32::from(old_stats[index]))
        }),
    ))
}
