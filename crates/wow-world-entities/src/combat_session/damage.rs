// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Melee absorb publication and the `SMSG_POWER_UPDATE` owner copy (#1363
//! slice 7a; moved from the World `session/combat/damage.rs` and
//! `session/combat/vitals.rs`). Behaviour is preserved; the canonical owner of
//! this state is unchanged.

use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_packet::ServerPacket;

use super::{
    CombatSessionHostLikeCpp, broadcast_player_combat_packet_to_visible_set_instance_like_cpp,
};

/// C++ `Unit::CalcAbsorbResist`'s absorb publication for one melee hit
/// (`Unit.cpp:1876-1889`): per shield that consumed part of the hit, send the
/// victim `SMSG_SPELL_ABSORB_LOG` and then remove the aura C++
/// left at zero.
///
/// The map-owned swing already committed the absorb arithmetic and the
/// shield amounts; this transition reads the shield's caster and spell
/// before the removal and keeps C++'s per-shield order (log, then removal).
pub(super) fn publish_melee_absorb_consumption_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &mut H,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    original_damage: i32,
    mana_spent: u32,
    consumptions: &[wow_world_core::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp],
    split_combat_log_packets: &[Vec<u8>],
) -> Result<(), &'static str> {
    for consumption in consumptions {
        let shield = host
            .core()
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
            host.send_packet(&packet);
            broadcast_player_combat_packet_to_visible_set_instance_like_cpp(
                host,
                packet.to_bytes(),
            );
        }
        if consumption.removed {
            // C++ `Remove(AURA_REMOVE_BY_ENEMY_SPELL)`; the session's aura
            // transition owns the removal publication and its side effects.
            host.remove_aura(consumption.slot)?;
        }
    }
    if mana_spent > 0 {
        // C++ `Unit::ModifyPower(POWER_MANA, -manaReduction)` sends
        // `SMSG_POWER_UPDATE` from the same call that drains the mana
        // (`Unit.cpp:1913-1918`, `Unit.cpp:9287-9312`). The map-owned stage
        // committed the drain; this session publishes the resulting value.
        if let Some(mana) = host.core().canonical_player_snapshot_like_cpp(|player| {
            player.unit().get_power(wow_constants::PowerType::Mana)
        }) {
            send_player_power_update_like_cpp(
                host,
                victim_guid,
                wow_constants::PowerType::Mana,
                mana,
            );
        }
    }
    for packet in split_combat_log_packets {
        host.send_raw_packet(packet);
        broadcast_player_combat_packet_to_visible_set_instance_like_cpp(host, packet.clone());
    }
    Ok(())
}

/// C++ `Unit::SetPower`'s `SMSG_POWER_UPDATE` publication
/// (`Unit.cpp:9287-9312`). C++ also marks the changed `UnitData::Power`
/// field, which the canonical Player setter already did, and sends the same
/// packet to the nearby observers through `SendMessageToSet(packet, true)`.
/// Both the owner copy and observer copies use the REALM connection, matching
/// `Opcodes.cpp:1911`.
pub fn send_player_power_update_like_cpp<H: CombatSessionHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
    power: PowerType,
    value: i32,
) {
    use wow_packet::ServerPacket;
    let packet = wow_packet::packets::combat::PowerUpdate {
        guid,
        powers: vec![(value, power as u8)],
    };
    host.core().send_packet_realm(&packet);
    host.broadcast_player_packet_to_visible_set_realm_like_cpp(packet.to_bytes());
}
