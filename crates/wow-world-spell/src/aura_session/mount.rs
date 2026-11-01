// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented mount auras and their capability speed effects, moved from the
//! World `session/spell_state/mount_aura.rs` (#1363 slice 9b).

use std::time::Instant;

use wow_core::ObjectGuid;
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectLikeCpp};

use super::{
    AuraHostLikeCpp, calculate_represented_mounted_aura_amount_like_cpp,
    insert_player_visible_aura_like_cpp, next_player_visible_aura_slot_like_cpp,
    select_represented_mount_aura_display_like_cpp, send_aura_update_applied,
};
use crate::represented_aura_effect_amounts_like_cpp;
use crate::spellbook_session::world_test_consumer_like_cpp;

pub fn apply_represented_mounted_aura_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    effect: &wow_data::SpellEffectInfo,
) -> Result<(), &'static str> {
    let mounted_amount = calculate_represented_mounted_aura_amount_like_cpp(host, spell_id, effect);
    let selected_display_id = u32::try_from(spell_id)
        .ok()
        .and_then(|spell_id| select_represented_mount_aura_display_like_cpp(host, spell_id))
        .unwrap_or(0);
    let creature_entry = u32::try_from(effect.effect_misc_value_1).unwrap_or(0);
    let creature_template_mount = {
        let (s, mut h) = host.split_world_entities_mut();
        s.represented_mount_creature_template_fallback_like_cpp(&mut h, creature_entry)
    };
    let display_id = if selected_display_id != 0 {
        selected_display_id
    } else {
        creature_template_mount
            .map(|(display_id, _)| display_id)
            .unwrap_or(0)
    };
    let vehicle_id = creature_template_mount
        .map(|(_, vehicle_id)| vehicle_id)
        .unwrap_or(0);

    let slot = next_player_visible_aura_slot_like_cpp(host)
        .ok_or("No free aura slots or missing Player aura owner")?;

    let aura = AuraApplication {
        spell_id,
        difficulty_id: host.core().current_map_difficulty_id_like_cpp(),
        caster_guid,
        slot,
        duration_total: -1,
        duration_remaining: -1,
        stack_count: 1,
        aura_flags: 0x0000_0001,
        effect_mask: 1u32 << effect.effect_index,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: Some(RepresentedAuraEffectLikeCpp::Mounted),
        represented_amount: mounted_amount,
        represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
        represented_misc_value: Some(effect.effect_misc_value_1),
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    };

    if !host
        .hub_mut()
        .set_player_mount_presentation_like_cpp(display_id, true)
    {
        return Err("Missing Player presentation owner");
    }
    if !insert_player_visible_aura_like_cpp(host, aura) {
        return Err("Missing Player aura owner");
    }
    if host
        .hub_mut()
        .create_player_mount_vehicle_kit_like_cpp(vehicle_id, creature_entry)
    {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures.vehicles.mount_vehicle_create_requests_like_cpp = fixtures
                .vehicles
                .mount_vehicle_create_requests_like_cpp
                .saturating_add(1);
        }
        host.hub_mut().send_set_vehicle_rec_id_like_cpp(vehicle_id);
        {
            let (s, mut h) = host.split_spell_state_mut();
            s.send_on_cancel_expected_vehicle_ride_aura_like_cpp(&mut h)
        };
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.pets.mount_pet_control_disable_requests_like_cpp = fixtures
            .pets
            .mount_pet_control_disable_requests_like_cpp
            .saturating_add(1);
    }
    host.hub_mut().disable_pet_controls_on_mount_like_cpp(
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .vehicles
            .mount_collision_height_update_requests_like_cpp = fixtures
            .vehicles
            .mount_collision_height_update_requests_like_cpp
            .saturating_add(1);
    }
    crate::update_player_collision_height_like_cpp(host);
    host.hub_mut().send_movement_set_collision_height_like_cpp(
        wow_packet::packets::movement::UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
    );
    apply_represented_mount_capability_speed_aura_like_cpp(host, mounted_amount, caster_guid);

    send_aura_update_applied(
        host,
        spell_id,
        slot,
        caster_guid,
        0,
        0x0000_0001,
        1u32 << effect.effect_index,
    );
    host.hub_mut()
        .send_represented_mount_unit_update_like_cpp(display_id);

    Ok(())
}

fn apply_represented_mount_capability_speed_aura_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    mount_capability_id: i32,
    caster_guid: ObjectGuid,
) {
    let Some(mod_spell_aura_id) = host
        .hub_ref()
        .catalogs
        .represented_mount_capability_mod_spell_like_cpp(mount_capability_id)
    else {
        return;
    };

    let Some(spell_info) = host
        .hub_ref()
        .catalogs
        .spell_store()
        .and_then(|store| store.get(mod_spell_aura_id))
        .cloned()
    else {
        return;
    };

    for effect in spell_info.effects().iter().filter(|effect| {
        effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
    }) {
        let represented_effect = match effect.effect_aura {
            aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED => {
                RepresentedAuraEffectLikeCpp::MountedSpeed
            }
            aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS => {
                RepresentedAuraEffectLikeCpp::MountedSpeedAlways
            }
            aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK => {
                RepresentedAuraEffectLikeCpp::MountedSpeedNotStack
            }
            aura if aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED =>
            {
                RepresentedAuraEffectLikeCpp::MountedFlightSpeed
            }
            aura if aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS =>
            {
                RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways
            }
            aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK => {
                RepresentedAuraEffectLikeCpp::FlightSpeedNotStack
            }
            _ => continue,
        };

        if host
            .apply_represented_aura_modifier_like_cpp(
                mod_spell_aura_id,
                caster_guid,
                effect,
                represented_effect,
                0,
            )
            .is_ok()
        {
            host.hub_mut()
                .recompute_represented_mounted_speed_rates_like_cpp();
        }
    }
}

pub fn remove_represented_mounted_auras_by_type_like_cpp<H: AuraHostLikeCpp>(host: &mut H) -> bool {
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return false;
    };
    let mounted_slots: Vec<u8> = visible_auras
        .iter()
        .filter_map(|(slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted))
                .then_some(*slot)
        })
        .collect();
    for slot in mounted_slots {
        let _ = host.remove_aura(slot);
    }

    let Some(mounted) = host.hub_ref().resolved_player_mounted_like_cpp() else {
        return false;
    };
    if mounted {
        if !host
            .hub_mut()
            .set_player_mount_presentation_like_cpp(0, false)
        {
            return false;
        }
        let _ = host.hub_mut().clear_player_mount_vehicle_kit_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures.vehicles.player_mount_vehicle_id_like_cpp = 0;
            fixtures
                .vehicles
                .player_mount_vehicle_accessories_like_cpp
                .clear();
            fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp = 0;
            fixtures
                .vehicles
                .player_mount_vehicle_usable_seat_count_like_cpp = 0;
        }
    }
    true
}

pub fn represented_mount_capability_selection_for_type_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
    mount_type_id: u16,
    riding_skill: u32,
    mount_restriction_flags: Option<u8>,
    is_submerged: bool,
    is_in_water: bool,
) -> Result<wow_data::MountCapabilityEntry, wow_data::MountCapabilityRejectLikeCpp> {
    let capability_store = host
        .hub_ref()
        .catalogs
        .mount_capability_store
        .as_ref()
        .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingCapabilityRow)?;
    let type_store = host
        .hub_ref()
        .catalogs
        .mount_type_x_capability_store
        .as_ref()
        .ok_or(wow_data::MountCapabilityRejectLikeCpp::MissingMountTypeCapabilities)?;
    let area_store = host
        .hub_ref()
        .catalogs
        .area_table_store
        .as_ref()
        .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;

    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let map = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(map_id));
    let (_, area_id) = host
        .hub_ref()
        .player_zone_area_like_cpp()
        .ok_or(wow_data::MountCapabilityRejectLikeCpp::Area)?;
    let mount_flags = mount_restriction_flags.unwrap_or_else(|| {
        area_store
            .get(area_id)
            .map(|area| area.mount_flags as u8)
            .unwrap_or_else(|| {
                if area_id == 0 {
                    // C++ reaches this check after TerrainMgr resolved the
                    // player's area. Until Rust has that full terrain bridge,
                    // an area 0 placeholder should not make ordinary ground
                    // mounts fail SPELL_FAILED_NOT_HERE.
                    wow_data::AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS
                } else {
                    0
                }
            })
    });
    let context = wow_data::MountCapabilityContextLikeCpp {
        riding_skill,
        mount_flags,
        is_submerged,
        is_in_water,
        map_id: map_id as i32,
        cosmetic_parent_map_id: map
            .map(|entry| i32::from(entry.cosmetic_parent_map_id))
            .unwrap_or(-1),
        parent_map_id: map
            .map(|entry| i32::from(entry.parent_map_id))
            .unwrap_or(-1),
    };
    let visible_auras = host
        .hub_ref()
        .resolved_player_visible_auras_like_cpp()
        .ok_or(wow_data::MountCapabilityRejectLikeCpp::Aura)?;

    capability_store
        .select_for_mount_type_with_reject_like_cpp(
            type_store,
            mount_type_id,
            &context,
            |required_area_id| area_store.is_in_area_like_cpp(area_id, u32::from(required_area_id)),
            |aura_id| {
                visible_auras
                    .values()
                    .any(|aura| u32::try_from(aura.spell_id).ok() == Some(aura_id))
            },
            |spell_id| crate::known_spells_like_cpp(host).contains(&spell_id),
        )
        .copied()
}

pub fn update_player_collision_height_like_cpp<H: AuraHostLikeCpp>(host: &mut H) {
    let world_test = world_test_consumer_like_cpp(host);
    let mut hub = host.hub_mut();
    let (presentation, mut control) = hub.aura_removal_mount_accesses_like_cpp();
    control.update_player_collision_height_like_cpp(&presentation, world_test);
}
