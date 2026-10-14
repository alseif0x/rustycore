// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented taunt effect (#1363 slice 9d), from the World
//! `session/spell_effects/effect_combat/damage_and_combat_application.rs`.

use super::{SpellEffectHostLikeCpp, sync_represented_creature_threat_to_canonical_like_cpp};
use std::time::Instant;
use tracing::debug;
use wow_core::ObjectGuid;
use wow_data::spell_duration_ms_like_cpp;
use wow_entities::AuraApplicationLikeCpp as AuraApplication;

/// C++ `Spell::EffectTaunt` / `SPELL_EFFECT_ATTACK_ME`.
pub fn apply_taunt_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    target_guid: ObjectGuid,
    enforce_effect_taunt_current_victim_gate: bool,
    provenance: wow_entities::AuraCastProvenanceLikeCpp,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    let difficulty = host.core().current_map_difficulty_id_like_cpp();
    let taunt_effect_mask = host.hub_ref().catalogs.spell_store().and_then(|store| {
        store
            .effects_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                host.hub_ref()
                    .catalogs
                    .difficulty_store()
                    .map(AsRef::as_ref),
            )
            .and_then(|effects| {
                effects.iter().find_map(|effect| {
                    (effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                        && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT)
                        .then(|| 1u32.checked_shl(effect.effect_index))
                        .flatten()
                })
            })
    });
    let duration_ms = {
        let duration_index = host
            .hub_ref()
            .catalogs
            .spell_catalogs
            .spell_misc_store
            .as_deref()
            .and_then(|store| {
                store.entry_for_spell_difficulty_like_cpp(
                    u32::try_from(spell_id).unwrap_or(0),
                    host.core().current_map_difficulty_id_like_cpp(),
                )
            })
            .map(|entry| u32::from(entry.duration_index))
            .unwrap_or(0);
        spell_duration_ms_like_cpp(
            duration_index,
            host.hub_ref()
                .catalogs
                .spell_catalogs
                .spell_duration_store
                .as_deref(),
        )
    };

    let Some((threat_value, taunt_slot)) = host
        .core_mut()
        .mutate_world_creature(target_guid, |creature| {
            let combat = &mut creature.unit_mut().subsystems_mut().combat;
            if !combat.owner_can_have_threat_list
                || (enforce_effect_taunt_current_victim_gate
                    && combat.current_victim_guid == Some(player_guid))
            {
                return None;
            }
            let threat = (!combat.is_threat_list_empty(false))
                .then(|| combat.match_unit_threat_to_highest_threat_like_cpp(player_guid))
                .flatten();
            if taunt_effect_mask.is_none() {
                combat
                    .set_threat_taunt_state(player_guid, wow_entities::ThreatTauntState::Taunt(1));
                combat.current_victim_guid = Some(player_guid);
            }
            let taunt_slot = taunt_effect_mask.filter(|_| duration_ms != 0).and_then(
                |effect_mask| {
                    wow_world_core::creature_projection::apply_taunt_aura_with_provenance_like_cpp(
                        creature,
                        player_guid,
                        u32::try_from(spell_id).ok()?,
                        effect_mask,
                        duration_ms,
                        provenance,
                    )
                },
            );
            Some((threat, taunt_slot))
        })
        .flatten()
    else {
        return Ok(());
    };

    if let Some(threat_value) = threat_value {
        sync_represented_creature_threat_to_canonical_like_cpp(
            host,
            target_guid,
            player_guid,
            threat_value,
        );
    }
    if let (Some(slot), Some(effect_mask)) = (taunt_slot, taunt_effect_mask) {
        use wow_packet::ServerPacket;

        let visible_duration_ms = u32::try_from(duration_ms).unwrap_or(u32::MAX);
        let aura = AuraApplication {
            spell_id,
            difficulty_id: host.core().current_map_difficulty_id_like_cpp(),
            caster_guid: player_guid,
            slot,
            duration_total: visible_duration_ms,
            duration_remaining: visible_duration_ms,
            stack_count: 1,
            aura_flags: 0,
            effect_mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };
        let packet = wow_packet::packets::misc::AuraUpdate {
            unit_guid: target_guid,
            update_all: false,
            auras: vec![crate::session::player_aura_info_like_cpp(
                &aura,
                host.hub_ref().player_level_like_cpp(),
                host.core().player_map_id_like_cpp(),
            )],
        };
        host.send_packet(&packet);
        host.broadcast_creature_packet_to_visible_set_like_cpp(target_guid, packet.to_bytes());
    }

    // C++ special-cases Hand of Reckoning (62124) by casting 67485 on
    // non-player targets outside the threat-list path. That triggered spell
    // requires broader spell/aura runtime and is intentionally outside this
    // represented threat-list slice.
    if spell_id == 62124 {
        debug!(
            account = host.core().account_id,
            target = ?target_guid,
            "represented EffectTaunt does not yet cast Hand of Reckoning damage spell 67485"
        );
    }

    Ok(())
}
