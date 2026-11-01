// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented creature publication at the session boundary (#1363 slice 10a;
//! moved from the World `session/world_entities/{creature,creature_publication}.rs`):
//! the `WorldObject::SendMessageToSet` receiver gates, the creature visible-set
//! broadcast, and the creature auras this session applies, ticks and publishes
//! (`Spell::EffectApplyAura` on a creature, `Creature::Update`'s aura lifetime,
//! `AuraApplication::ClientUpdate`).

use std::time::Instant;

use wow_core::ObjectGuid;

use crate::RepresentedCreatureAuraLikeCpp;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;

pub fn represented_can_receive_creature_message_to_set_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
    creature: &wow_entities::Creature,
    required_3d: bool,
) -> bool {
    if !host.core().client_visible_guids_like_cpp.contains(&guid) {
        return false;
    }

    let Some(player_position) = host.hub_ref().player_position_like_cpp() else {
        return false;
    };
    let player_map_id = u32::from(host.core().player_map_id_like_cpp());
    let player_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let Some(player_phase_shift) = host.represented_player_phase_shift_like_cpp() else {
        return false;
    };

    crate::creature_message_to_set_target_allows_like_cpp(
        creature,
        // The HaveAtClient membership was proven above.
        true,
        player_map_id,
        player_instance_id,
        &player_position,
        &player_phase_shift,
        required_3d,
    )
}

pub fn represented_can_receive_creature_message_to_set_by_guid_like_cpp<
    H: WorldEntitiesHostLikeCpp,
>(
    host: &H,
    guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    required_3d: bool,
) -> Option<bool> {
    let creature = {
        let manager = host.core().canonical_map_manager.as_ref()?.lock().ok()?;
        manager
            .find_map(u32::from(map_id), instance_id)
            .and_then(|map| {
                map.map()
                    .with_creature_like_cpp(guid, |creature| creature.clone())
            })
    }?;
    if creature.map_id() != u32::from(map_id) || creature.instance_id() != instance_id {
        return Some(false);
    }
    Some(represented_can_receive_creature_message_to_set_like_cpp(
        host,
        guid,
        &creature,
        required_3d,
    ))
}

pub fn apply_creature_aura_with_provenance_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    effect_mask: u32,
    duration_ms: u32,
    provenance: wow_entities::AuraCastProvenanceLikeCpp,
) -> Result<(), &'static str> {
    let Ok(spell_key) = u32::try_from(spell_id) else {
        return Err("invalid spell id");
    };
    let spell_store = host
        .hub_ref()
        .catalogs
        .spell_store()
        .cloned()
        .ok_or("Spell store not loaded")?;
    let Some(spell) = spell_store.get(spell_id) else {
        return Err("Spell not found");
    };
    let effects: Vec<(i32, i32, i32, u8)> = spell
        .effects()
        .iter()
        .filter(|effect| {
            1u32.checked_shl(effect.effect_index)
                .is_some_and(|bit| effect_mask & bit != 0)
        })
        .map(|effect| {
            (
                effect.effect_aura,
                effect.calc_value_no_caster_like_cpp(),
                effect.effect_misc_value_1,
                u8::try_from(effect.effect_index).unwrap_or(0),
            )
        })
        .collect();
    if effects.is_empty() {
        return Err("no represented effects for this aura");
    }
    let applied = host
        .core_mut()
        .mutate_canonical_creature_by_guid_like_cpp(target_guid, |creature| {
            let auras = &mut creature.unit_mut().subsystems_mut().auras;
            if auras
                .applied_auras
                .iter()
                .any(|aura| aura.spell_id == spell_key && aura.caster_guid == caster_guid)
            {
                return None;
            }
            let slot = (0..u8::MAX).find(|slot| !auras.visible_auras.contains_key(slot))?;
            auras.add_owned(wow_entities::OwnedAuraRef::new(
                spell_key,
                caster_guid,
                None,
            ));
            // C++ `Aura::Create` builds one `AuraEffect` per applied slot
            // and `GetAuraEffectsByType` reads those effects individually.
            // The per-slot `AppliedAuraRef` keeps each slot's own amount
            // and misc value, the convention the pet-load and
            // threat-snapshot paths already use.
            for (aura_type, amount, misc_value, effect_index) in &effects {
                let effect_ref = wow_entities::AppliedAuraRef::new(
                    spell_key,
                    caster_guid,
                    slot,
                    1_u32 << u32::from(*effect_index),
                );
                auras.register_applied_aura_effect_like_cpp(
                    effect_ref,
                    *aura_type,
                    *amount,
                    *misc_value,
                );
            }
            auras.set_loaded_aura_state_like_cpp(
                wow_entities::AuraRef::new(spell_key, caster_guid),
                wow_entities::LoadedAuraStateLikeCpp::new(
                    i32::try_from(duration_ms).unwrap_or(i32::MAX),
                    i32::try_from(duration_ms).unwrap_or(i32::MAX),
                    0,
                    1,
                    0,
                ),
            );
            auras.set_visible_with_application_like_cpp(
                slot,
                wow_entities::AuraRef::new(spell_key, caster_guid),
                wow_entities::VisibleAuraApplicationLikeCpp::new(
                    0,
                    effects
                        .iter()
                        .map(|(_, amount, _, effect_index)| {
                            wow_entities::VisibleAuraEffectAmountLikeCpp {
                                effect_index: *effect_index,
                                amount: *amount,
                            }
                        })
                        .collect(),
                ),
            );
            auras.set_aura_cast_provenance_like_cpp(slot, provenance);
            Some(())
        })
        .flatten()
        .ok_or("creature target unavailable or aura already applied")?;
    let _ = applied;
    let slot = canonical_creature_aura_slot_like_cpp(host, target_guid, spell_key, caster_guid)
        .ok_or("creature aura slot missing after application")?;
    host.world_entities_mut()
        .record_represented_creature_aura_like_cpp(RepresentedCreatureAuraLikeCpp {
            target_guid,
            spell_id,
            caster_guid,
            slot,
            effect_mask,
            applied_at: Instant::now(),
            duration_ms,
        });
    publish_creature_aura_slot_update_like_cpp(host, target_guid, slot, true);
    Ok(())
}

fn canonical_creature_aura_slot_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    target_guid: ObjectGuid,
    spell_id: u32,
    caster_guid: ObjectGuid,
) -> Option<u8> {
    let (state, mut hub) = host.split_world_entities_mut();
    state.canonical_creature_aura_slot_like_cpp(&mut hub, target_guid, spell_id, caster_guid)
}

/// C++ `AuraApplication` publication: the slot's current state when
/// `applied`, or a removal entry (`AuraInfoLikeCpp { aura_data: None }`)
/// when not. The caster's session receives it directly and the creature's
/// nearby observers through the existing creature visibility rail.
fn publish_creature_aura_slot_update_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    target_guid: ObjectGuid,
    slot: u8,
    applied: bool,
) {
    use wow_packet::ServerPacket;
    let Some(map_key) = host
        .core()
        .canonical_object_lookup_map_key_like_cpp(u32::from(host.core().player_map_id_like_cpp()))
    else {
        return;
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref().cloned() else {
        return;
    };
    let Some(aura_info) = ({
        let mut manager = match manager.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        managed
            .map()
            .with_creature_like_cpp(target_guid, |creature| {
                if !applied {
                    return wow_packet::packets::misc::AuraInfoLikeCpp {
                        slot,
                        aura_data: None,
                    };
                }
                let level = creature.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8;
                crate::creature_publication::represented_creature_aura_info_like_cpp(
                    &creature.unit().subsystems().auras,
                    slot,
                    level,
                )
            })
    }) else {
        return;
    };
    let packet = wow_packet::packets::misc::AuraUpdate {
        unit_guid: target_guid,
        update_all: false,
        auras: vec![aura_info],
    };
    host.send_packet(&packet);
    broadcast_creature_packet_to_visible_set_like_cpp(host, target_guid, packet.to_bytes());
}

/// C++ `Creature::Update`'s aura lifetime for the auras this session
/// applied: a duration that elapsed removes the canonical application and
/// publishes the removal.
pub fn tick_represented_creature_auras_like_cpp<H: WorldEntitiesHostLikeCpp>(host: &mut H) {
    if host
        .world_entities()
        .represented_creature_auras_are_empty_like_cpp()
    {
        return;
    }
    let expired: Vec<RepresentedCreatureAuraLikeCpp> = host
        .world_entities()
        .expired_represented_creature_auras_like_cpp();
    for aura in expired {
        let removed = host
            .core_mut()
            .mutate_canonical_creature_by_guid_like_cpp(aura.target_guid, |creature| {
                let auras = &mut creature.unit_mut().subsystems_mut().auras;
                let spell_id = u32::try_from(aura.spell_id).unwrap_or(0);
                // One per-slot `AppliedAuraRef` per applied effect slot;
                // removing every covered slot is the C++
                // `AuraApplication` removal. `unapply_aura` also removes
                // the loaded state and `clear_visible` the published slot.
                let mut removed = false;
                for effect_index in 0..u32::BITS {
                    let bit = 1_u32 << effect_index;
                    if aura.effect_mask & bit == 0 {
                        continue;
                    }
                    let applied = wow_entities::AppliedAuraRef::new(
                        spell_id,
                        aura.caster_guid,
                        aura.slot,
                        bit,
                    );
                    removed |= auras.unapply_aura(applied, 0);
                }
                let _ = auras.clear_visible(aura.slot);
                removed
            })
            .unwrap_or(false);
        host.world_entities_mut()
            .retire_represented_creature_aura_like_cpp(aura);
        if removed {
            publish_creature_aura_slot_update_like_cpp(host, aura.target_guid, aura.slot, false);
        }
    }
}

/// Route a creature-originated packet through the existing
/// `MessageDistDeliverer`-style candidate and per-session visibility gates.
/// The activating session receives its direct packet separately; this
/// queues only nearby observers, matching `WorldObject::SendMessageToSet`.
pub fn broadcast_creature_packet_to_visible_set_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    source_guid: ObjectGuid,
    bytes: Vec<u8>,
) {
    broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
        host,
        source_guid,
        bytes,
        false,
    );
}

fn broadcast_creature_packet_to_visible_set_and_connection_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &H,
    source_guid: ObjectGuid,
    bytes: Vec<u8>,
    realm_connection: bool,
) {
    let Some(source) = ({
        let (state, hub) = host.split_world_entities_ref();
        state.canonical_creature_access_like_cpp(hub, source_guid)
    }) else {
        return;
    };
    {
        let (s, h) = host.split_world_entities_ref();
        s.broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
            h,
            source_guid,
            source.position,
            bytes,
            realm_connection,
        )
    };
}
