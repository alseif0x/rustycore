// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Effective stat snapshot reconciliation and VALUES publication adapters.

use super::*;

impl WorldSession {
    /// Recalculate all stats from base + gear while preserving current power.
    pub(super) fn player_stat_changes_like_cpp(
        &mut self,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
    }

    pub(super) fn player_stat_changes_with_represented_item_bonuses_like_cpp(
        &mut self,
        _include_represented_item_bonuses: bool,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return None,
        };

        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let level = self.player_level_like_cpp();

        if race == 0 || class == 0 || level == 0 {
            return None; // Not fully logged in yet
        }

        let gear = self.represented_player_gear_stats_like_cpp(true)?;
        let projection = self.player_stat_system_projection_like_cpp(race, class, level, &gear)?;
        self.publish_effective_stats_like_cpp(level, true, projection, &gear);
        let computed_max_health_u32 = max_health_u32_like_cpp(projection.max_health);
        let (health, max_health_for_update) =
            self.sync_canonical_player_max_health_like_cpp(computed_max_health_u32)?;
        let health = i64::from(health);
        let max_health = i64::from(max_health_for_update);

        let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
            projection,
            gear.weapon_damage,
            gear.base_attack_time,
            self.represented_shapeshift_combat_round_time_like_cpp(),
        );

        // Power for slot 0 (mana/rage/energy/runic). Keep current power from
        // the runtime player and update only the max, like C++ `SetMaxPower`.
        let primary_power_type = primary_power_type_for_class_like_cpp(class);
        let computed_max_power0 = primary_max_power_for_class_like_cpp(class, projection.max_mana);
        let base_mana = if primary_power_type == PowerType::Mana {
            projection.base_mana
        } else {
            0
        };
        let (power0, max_power0) = self
            .sync_canonical_player_primary_power_max_like_cpp(
                primary_power_type,
                computed_max_power0,
                base_mana,
            )
            .or_else(|| {
                self.canonical_player_power_snapshot_like_cpp(primary_power_type)
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
            .canonical_player_effective_combat_stats_like_cpp()
            .map(|stats| (stats.mainhand_expertise, stats.offhand_expertise))
            .unwrap_or_else(|| {
                let rating = (gear.combat_ratings[23] as f32
                    * self.combat_rating_multiplier_like_cpp(level, 23))
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
                    self.mana_regen_from_stats_like_cpp(level, class, projection.stats)
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
            // Mana regen
            mana_regen,
            mana_regen_combat,
            mana_regen_mp5,
            // Expertise
            mainhand_expertise,
            offhand_expertise,
            // Extended parent 38 fields. C++ never writes
            // `RangedExpertise`/`CombatRatingExpertise`, so they keep their
            // zero create value.
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
            info!(
                guid = ?player_guid,
                power_type = ?primary_power_type,
                current_power0 = power0,
                max_power0,
                base_mana,
                "RUST_STAT_POWER_UPDATE"
            );
        }

        debug!(
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

    /// Recalculate all stats from base + gear and send a VALUES update to the client.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    pub(crate) fn send_stat_update(&mut self) -> bool {
        let Some((player_guid, changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return false;
        };

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update)
    }

    /// Recalculate stats for C++ `HandleModTotalPercentStat`.
    ///
    /// Ability auras that select stamina preserve the pre-change health
    /// percentage after max health is recalculated. Other total-stat auras use
    /// ordinary `SetMaxHealth` clamping.
    pub(crate) fn send_total_stat_percentage_update_like_cpp(&mut self, preserve_health_pct: bool) {
        let Some((health_before, max_health_before, _)) = self.resolved_player_vitals_like_cpp()
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
            let _ = self.sync_canonical_player_health_like_cpp(restored, max_health_after);
            changes.health = i64::from(restored);
        }

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    /// C++ `Player::GiveLevel` refills health and powers carrying
    /// `PowerTypeFlags::SetToMaxOnLevelUp` after `UpdateAllStats`.
    ///
    /// The 3.4.3 data path represented here has mana as the refillable
    /// primary power; rage, energy and runic power preserve their current
    /// values.
    pub(crate) fn send_level_up_stat_update_like_cpp(&mut self) {
        let Some((player_guid, mut changes)) = self.player_stat_changes_like_cpp() else {
            return;
        };

        let max_health = max_health_u32_like_cpp(changes.max_health);
        let _ = self.sync_canonical_player_health_like_cpp(max_health, max_health);
        changes.health = i64::from(max_health);

        if primary_power_type_for_class_like_cpp(self.player_class_like_cpp()) == PowerType::Mana {
            changes.power0 = changes.max_power0;
            let _ = self.sync_canonical_player_primary_power_like_cpp(
                PowerType::Mana,
                changes.power0,
                changes.max_power0,
                changes.base_mana,
            );
        }

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    pub(super) fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self) {
        let Some((player_guid, changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return;
        };

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }
}
