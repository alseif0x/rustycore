//! Represented damage application, absorption and resistance.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
use wow_packet::ServerPacket;

impl WorldSession {
    /// C++ `Unit::CalcAbsorbResist`'s absorb publication for one melee hit
    /// (`Unit.cpp:1876-1889`): per shield that consumed part of the hit, send the
    /// victim `SMSG_SPELL_ABSORB_LOG` and then remove the aura C++
    /// left at zero.
    ///
    /// The map-owned swing already committed the absorb arithmetic and the
    /// shield amounts; this transition reads the shield's caster and spell
    /// before the removal and keeps C++'s per-shield order (log, then removal).
    pub(crate) fn publish_melee_absorb_consumption_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        original_damage: i32,
        mana_spent: u32,
        consumptions: &[crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp],
        split_combat_log_packets: &[Vec<u8>],
    ) {
        for consumption in consumptions {
            let shield = self
                .core
                .canonical_player_snapshot_like_cpp(|player| {
                    player
                        .unit()
                        .subsystems()
                        .auras
                        .runtime_application_like_cpp(consumption.slot)
                        .map(|aura| (aura.caster_guid, aura.spell_id))
                })
                .flatten();
            if let Some((caster, absorb_spell_id)) = shield
                && consumption.consumed > 0
            {
                // A white melee swing carries no spell of its own, so C++
                // publishes `AbsorbedSpellID == 0` (`Unit.cpp:1876-1882`).
                let packet = wow_packet::packets::combat::SpellAbsorbLog {
                    attacker: attacker_guid,
                    victim: victim_guid,
                    absorbed_spell_id: 0,
                    absorb_spell_id,
                    caster,
                    absorbed: consumption.consumed,
                    original_damage,
                };
                // C++ `WorldObject::SendCombatLogMessage` sends the victim's
                // direct copy and then distributes the same combat-log frame
                // to nearby visible players (`Object.cpp:1785-1794`).
                self.send_packet(&packet);
                self.broadcast_player_packet_to_visible_set_realm_like_cpp(packet.to_bytes());
            }
            if consumption.removed {
                // C++ `Remove(AURA_REMOVE_BY_ENEMY_SPELL)`; the session's aura
                // transition owns the removal publication and its side effects.
                let _ = self.remove_aura(consumption.slot);
            }
        }
        if mana_spent > 0 {
            // C++ `Unit::ModifyPower(POWER_MANA, -manaReduction)` sends
            // `SMSG_POWER_UPDATE` from the same call that drains the mana
            // (`Unit.cpp:1913-1918`, `Unit.cpp:9287-9312`). The map-owned stage
            // committed the drain; this session publishes the resulting value.
            if let Some(mana) = self.core.canonical_player_snapshot_like_cpp(|player| {
                player.unit().get_power(wow_constants::PowerType::Mana)
            }) {
                self.send_player_power_update_like_cpp(
                    victim_guid,
                    wow_constants::PowerType::Mana,
                    mana,
                );
            }
        }
        for packet in split_combat_log_packets {
            self.send_raw_packet(packet);
            self.broadcast_player_packet_to_visible_set_realm_like_cpp(packet.clone());
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/damage/f3_shims.rs"]
mod f3_shims;
