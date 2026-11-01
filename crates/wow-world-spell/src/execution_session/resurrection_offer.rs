//! Player resurrection offers from C++ `Spell::EffectResurrect*` and
//! `Spell::SendResurrectRequest` (`SpellEffects.cpp:416-442,3599-3627,5328-5358`,
//! `Spell.cpp:5288-5310`).

use super::SpellExecutionHostLikeCpp;
use wow_core::ObjectGuid;
use wow_entities::{PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp};
use wow_map::{Map, MapKey, PlayerHandle};
use wow_packet::{ServerPacket, packets::spell::ResurrectRequest};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

pub(super) const SPELL_EFFECT_RESURRECT: u32 = 18;
pub(super) const SPELL_EFFECT_RESURRECT_NEW: u32 = 113;
pub(super) const SPELL_EFFECT_RESURRECT_WITH_AURA: u32 = 172;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Admission {
    handle: PlayerHandle,
    residence_revision: u64,
    key: MapKey,
}

#[derive(Clone, Debug, PartialEq)]
struct OffererSnapshot {
    guid: ObjectGuid,
    map_id: u32,
    position: wow_core::Position,
    actual_name: String,
    creature_entry: Option<u32>,
    sickness: bool,
}

fn recipient_guid_from_hit(
    map: &Map,
    effect: u32,
    hit_guid: ObjectGuid,
) -> Option<(ObjectGuid, bool)> {
    if effect == SPELL_EFFECT_RESURRECT_WITH_AURA {
        return (hit_guid.is_player() && map.get_typed_player(hit_guid).is_some())
            .then_some((hit_guid, false));
    }
    if hit_guid.is_player() && map.get_typed_player(hit_guid).is_some() {
        return Some((hit_guid, false));
    }
    map.get_typed_corpse(hit_guid)
        .map(|corpse| corpse.data().owner)
        .filter(|owner| !owner.is_empty())
        .map(|owner| (owner, true))
}

fn admission_for_handle(manager: &wow_map::MapManager, handle: PlayerHandle) -> Option<Admission> {
    let (current, owner_revision) = manager.current_player_admission_like_cpp(handle.guid())?;
    if current != handle {
        return None;
    }
    let (key, residence_revision) = manager.player_active_residence_revision_like_cpp(handle)?;
    (residence_revision == owner_revision).then_some(Admission {
        handle,
        residence_revision,
        key,
    })
}

fn admission_for_guid(manager: &wow_map::MapManager, guid: ObjectGuid) -> Option<Admission> {
    let (handle, owner_revision) = manager.current_player_admission_like_cpp(guid)?;
    let (key, residence_revision) = manager.player_active_residence_revision_like_cpp(handle)?;
    (residence_revision == owner_revision).then_some(Admission {
        handle,
        residence_revision,
        key,
    })
}

fn is_dead_in_world(player: &wow_entities::Player) -> bool {
    player.unit().world().object().is_in_world() && !player.unit().is_alive()
}

fn offerer_snapshot_on_map(map: &Map, caster_guid: ObjectGuid) -> Option<OffererSnapshot> {
    let from_unit = |unit: &wow_entities::Unit, actual_name, creature_entry| OffererSnapshot {
        guid: caster_guid,
        map_id: unit.world().map_id(),
        position: unit.world().position(),
        actual_name,
        creature_entry,
        sickness: unit.npc_flags_like_cpp()[0]
            & wow_constants::unit::NPCFlags1::SPIRIT_HEALER.bits()
            != 0,
    };

    if let Some(player) = map.get_typed_player(caster_guid) {
        return Some(from_unit(player.unit(), String::new(), None));
    }
    if let Some(creature) = map.get_typed_creature(caster_guid) {
        return Some(from_unit(
            creature.unit(),
            creature.unit().world().name().to_owned(),
            Some(creature.entry()),
        ));
    }
    let pet = map.get_typed_pet(caster_guid)?;
    let creature = pet.creature();
    Some(from_unit(
        creature.unit(),
        creature.unit().world().name().to_owned(),
        Some(creature.entry()),
    ))
}

fn packet_name(
    offerer: &OffererSnapshot,
    locale: &str,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
) -> String {
    // C++ `Creature::GetNameForLocaleIdx` (`Creature.cpp:3109-3118`) uses the
    // live creature's actual name when its locale row is absent or empty.
    offerer
        .creature_entry
        .and_then(|entry| {
            creature_spawn_catalogs
                .localized_names
                .localized_name(entry, locale)
        })
        .unwrap_or(&offerer.actual_name)
        .to_owned()
}

fn active_pet_number(map: &Map, player: &wow_entities::Player) -> u32 {
    let Some(pet_guid) = player.gameplay_state().pet_guid else {
        return 0;
    };
    map.get_typed_pet(pet_guid)
        .and_then(|pet| {
            pet.creature()
                .unit()
                .subsystems()
                .control
                .charm_info
                .as_ref()
                .map(|charm| charm.pet_number)
        })
        .unwrap_or_default()
}

fn percent_of_max_health(max_health: u64, percent: i32) -> u32 {
    // `Unit::CountPctFromMaxHealth` uses CalculatePct's float arithmetic and
    // returns uint64; SetResurrectRequestData then narrows it to uint32.
    ((max_health as f32 * percent as f32 / 100.0) as u64) as u32
}

fn percent_of_max_mana(max_mana: i32, percent: i32) -> u32 {
    // `GetMaxPower(POWER_MANA)` and CalculatePct return int32 here; the
    // request setter's uint32 parameter applies C++'s signed-to-unsigned wrap.
    ((max_mana as f32 * percent as f32 / 100.0) as i32) as u32
}

fn request_state_is_clear(state: &PlayerResurrectionStateLikeCpp) -> bool {
    state.request.is_none() && state.delayed_after_teleport.is_none()
}

/// Resolve the session registration outside the MapManager lock, then verify
/// its generation-bound canonical owner and both Player residence stamps under
/// that lock. The packet is sent only after the request transition and unlock.
pub(super) fn apply_resurrection_offer<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    spell_info: &wow_data::SpellInfo,
    effect_info: &wow_data::SpellEffectInfo,
    caster_guid: ObjectGuid,
    hit_guid: ObjectGuid,
) {
    let effect = effect_info.effect;
    if !matches!(
        effect,
        SPELL_EFFECT_RESURRECT | SPELL_EFFECT_RESURRECT_NEW | SPELL_EFFECT_RESURRECT_WITH_AURA
    ) {
        return;
    }
    let Some(registry) = host.core().player_registry().cloned() else {
        return;
    };
    let Some(manager_arc) = host.core().canonical_map_manager.as_ref().cloned() else {
        return;
    };
    let Some(session_handle) = host.core().player_handle_like_cpp else {
        return;
    };

    // Corpse lookup is local to the casting map, but its owner is looked up
    // globally through canonical Player admission just as FindPlayer is.
    let (recipient_guid, from_corpse, session_admission, recipient_admission, offerer_before) = {
        let Ok(manager) = manager_arc.lock() else {
            return;
        };
        let Some(session_admission) = admission_for_handle(&manager, session_handle) else {
            return;
        };
        let Some(source_map) = manager
            .find_map(
                session_admission.key.map_id,
                session_admission.key.instance_id,
            )
            .map(|managed| managed.map())
        else {
            return;
        };
        let Some((recipient_guid, from_corpse)) =
            recipient_guid_from_hit(source_map, effect, hit_guid)
        else {
            return;
        };
        let Some(recipient_admission) = admission_for_guid(&manager, recipient_guid) else {
            return;
        };
        if (!from_corpse && recipient_admission.key != session_admission.key)
            || !manager
                .with_player_like_cpp(recipient_admission.handle, is_dead_in_world)
                .unwrap_or(false)
        {
            return;
        }
        let Some(offerer_before) = offerer_snapshot_on_map(source_map, caster_guid) else {
            return;
        };
        (
            recipient_guid,
            from_corpse,
            session_admission,
            recipient_admission,
            offerer_before,
        )
    };

    let Some(registration) = registry
        .control_address(recipient_guid)
        .map(|address| address.registration())
    else {
        return;
    };
    let virtual_realm_address = host.core().virtual_realm_address();
    let difficulty_id = host.core().current_map_difficulty_id_like_cpp();
    let use_timer = {
        let catalogs = &host.hub_ref().catalogs;
        catalogs
            .spell_store()
            .and_then(|store| {
                store.misc_attributes_for_difficulty_like_cpp(
                    spell_info.spell_id,
                    difficulty_id,
                    catalogs.difficulty_store().map(AsRef::as_ref),
                )
            })
            .is_none_or(|attributes| attributes[3] & 0x0000_0010 == 0) // SharedDefines.h:546
    };

    let trigger_aura = if effect == SPELL_EFFECT_RESURRECT_WITH_AURA {
        let trigger_id = effect_info.effect_trigger_spell;
        (trigger_id > 0
            && host
                .hub_ref()
                .catalogs
                .spell_store()
                .is_some_and(|store| store.get(trigger_id).is_some()))
        .then_some(trigger_id as u32)
        .unwrap_or_default()
    } else {
        0
    };
    // Interim fixed-value/die-side calculation only. This uses the selected
    // effect's existing no-caster helper and does not establish full caster
    // SpellInfo::CalcValue parity; DB2 scaling metadata is retained for a later unit.
    let amount = effect_info.calc_value_no_caster_like_cpp();

    let packet = {
        let Ok(mut manager) = manager_arc.lock() else {
            return;
        };
        if admission_for_handle(&manager, session_handle) != Some(session_admission)
            || admission_for_guid(&manager, recipient_guid) != Some(recipient_admission)
            || (!from_corpse && recipient_admission.key != session_admission.key)
        {
            return;
        }
        let Some(recipient_locale) =
            registry.current_player_locale(registration, recipient_admission.handle)
        else {
            return;
        };
        if !manager
            .with_player_like_cpp(recipient_admission.handle, is_dead_in_world)
            .unwrap_or(false)
        {
            return;
        }

        let (offerer, pet_number) = {
            let Some(source_map) = manager
                .find_map(
                    session_admission.key.map_id,
                    session_admission.key.instance_id,
                )
                .map(|managed| managed.map())
            else {
                return;
            };
            let Some(current_offerer) = offerer_snapshot_on_map(source_map, caster_guid) else {
                return;
            };
            if current_offerer != offerer_before {
                return;
            }
            let Some(target_map) = manager
                .find_map(
                    recipient_admission.key.map_id,
                    recipient_admission.key.instance_id,
                )
                .map(|managed| managed.map())
            else {
                return;
            };
            let pet_number = manager
                .with_player_like_cpp(recipient_admission.handle, |player| {
                    active_pet_number(target_map, player)
                })
                .unwrap_or_default();
            let mut offerer = current_offerer;
            offerer.actual_name = packet_name(&offerer, &recipient_locale, creature_spawn_catalogs);
            (offerer, pet_number)
        };

        let Some(Some(())) =
            manager.with_player_mut_like_cpp(recipient_admission.handle, |player| {
                if !is_dead_in_world(player)
                    || !request_state_is_clear(player.resurrection_state_like_cpp())
                {
                    return None;
                }
                if trigger_aura != 0
                    && player
                        .unit()
                        .subsystems()
                        .auras
                        .has_aura_spell_like_cpp(trigger_aura)
                {
                    return None;
                }

                let (health, mana) = match effect {
                    SPELL_EFFECT_RESURRECT_NEW => {
                        (amount as u32, effect_info.effect_misc_value_1 as u32)
                    }
                    SPELL_EFFECT_RESURRECT | SPELL_EFFECT_RESURRECT_WITH_AURA => (
                        percent_of_max_health(player.unit().data().max_health, amount),
                        percent_of_max_mana(
                            player.get_max_power(wow_constants::PowerType::Mana),
                            amount,
                        ),
                    ),
                    _ => return None,
                };
                player.set_resurrection_request_like_cpp(PlayerResurrectionRequestLikeCpp {
                    resurrecter: offerer.guid,
                    map_id: offerer.map_id,
                    position: offerer.position,
                    health,
                    mana,
                    aura: trigger_aura,
                });
                Some(())
            })
        else {
            return;
        };

        ResurrectRequest {
            offerer_guid: offerer.guid,
            offerer_virtual_realm_address: virtual_realm_address,
            pet_number,
            spell_id: spell_info.spell_id,
            use_timer,
            sickness: offerer.sickness,
            name: offerer.actual_name,
        }
    };

    host.split_spell_state_mut()
        .0
        .represented_spell_execute_log_effect_like_cpp(effect as i32)
        .generic_victim_targets
        .push(
            wow_packet::packets::combat::SpellLogEffectGenericVictimParams {
                victim: recipient_guid,
            },
        );
    let _ = registry.send_current_realm_packet(registration, packet.to_bytes());
}
