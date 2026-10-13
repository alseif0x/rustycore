// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The creature loot-release VALUES and creature melee-damage session-command
//! receivers. Neither is loot; they stay here until slice 7 (#1363). The loot
//! command receivers and the `CMSG_LOOT_MONEY` body moved to
//! `wow-world-loot-ops` (`session_loot::handlers`) in slice 5c.

use super::*;

#[cfg(test)]
mod test_shims;

impl WorldSession {
    /// Receiver-owned half of the loot-release VALUES fanout. Applying
    /// `Player::isAllowedToLoot` here preserves session-local pending-bind
    /// state and avoids serialising one player's dynamic flags for another.
    pub(crate) fn handle_send_creature_loot_release_values_update_command_like_cpp(
        &mut self,
        command: SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.core.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if instance_id != command.instance_id
            || !self
                .core
                .client_visible_guids_like_cpp
                .contains(&command.creature_guid)
        {
            return;
        }
        if self.represented_can_receive_creature_message_to_set_by_guid_like_cpp(
            command.creature_guid,
            command.map_id,
            command.instance_id,
            false,
        ) != Some(true)
        {
            return;
        }
        let Some(expected_authority) = command.authority.as_ref() else {
            return;
        };
        // Both non-found facts refuse the queued VALUES update, and neither
        // reconstructs an authority from the other store.
        let OwnedLootAuthorityLookupOutcomeLikeCpp::Found(current_authority) =
            wow_world_loot_ops::represented_owned_loot_authority_outcome_like_cpp(
                self,
                command.creature_guid,
            )
        else {
            return;
        };
        if !current_authority.shares_storage_like_cpp(expected_authority) {
            // The queued update belongs to an older corpse generation. C++
            // publishes synchronously before respawn; Rust must not apply the
            // delayed VALUES delta to a replacement creature with the same GUID.
            return;
        }
        let Some(viewer_guid) = self.player_guid() else {
            return;
        };
        let viewer_update = wow_world_loot_ops::creature_loot_release_values_for_viewer_like_cpp(
            self,
            command.creature_guid,
            viewer_guid,
            self.instances.has_pending_bind_like_cpp(),
            Some(expected_authority),
            command.unit_values_update,
        );
        self.send_packet(&UpdateObject::unit_values_update(
            command.creature_guid,
            command.map_id,
            viewer_update,
        ));
    }

    /// Apply a transitional map-owned creature melee compatibility hit to this
    /// player session.
    ///
    /// C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()`, which
    /// eventually emits `AttackerStateUpdate` from the map update tick and
    /// then applies damage to the victim. This driver preserves the earlier
    /// normal-hit bridge; it does not claim full `CalculateMeleeDamage` parity.
    /// It owns the swing timer/damage/canonical health mutation once, and this
    /// command is only the victim-session delivery rail. Delivery rereads the
    /// current canonical health/death tuple and advances a presentation-only
    /// revision, so neither retries nor a delayed command can write an older
    /// value over a newer heal, hit, death, or resurrection.
    pub(crate) fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if self.core.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let Some(canonical_health) = self.present_committed_creature_melee_health_like_cpp(
            command.victim_health_state_revision_after,
        ) else {
            return;
        };
        // C++ `Unit::Kill` applies the creature-killer durability loss while
        // the lethal damage is applied, before the melee result presentation.
        self.publish_creature_melee_death_durability_loss_like_cpp(command.over_damage);

        // C++ `Unit::CalcAbsorbResist`'s absorb log and shield removal (`Unit.cpp:1876-1889`).
        self.publish_melee_absorb_consumption_like_cpp(
            command.attacker_guid,
            command.victim_guid,
            command.original_damage.min(i32::MAX as u32) as i32,
            command.mana_spent,
            &command.absorb_consumptions,
            &command.split_combat_log_packets,
        );
        use wow_packet::packets::combat::{AttackerStateUpdate, HealthUpdate};
        // Visibility gates only the attacker-facing combat packet, never the
        // authoritative victim health/death reconciliation.
        if self
            .core
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
            self.send_packet(&AttackerStateUpdate {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
                hit_info: command.hit_info,
                damage: command.damage.min(i32::MAX as u32) as i32,
                original_damage: command.original_damage.min(i32::MAX as u32) as i32,
                over_damage: command.over_damage,
                blocked: 0,
                absorbed: command.absorbed.min(i32::MAX as u32) as i32,
                victim_state: command.victim_state,
                school_mask: 1,
                target_level: command.target_level,
                expansion: 2,
            });
        }
        crate::session::hub_ref(self).publish_self_share_health_like_cpp(&command);
        // An avoided swing commits no health transition.
        if command.damage > 0 {
            self.send_packet(&HealthUpdate {
                guid: command.victim_guid,
                health: canonical_health.min(i64::MAX as u64) as i64,
            });
        }
    }
}
