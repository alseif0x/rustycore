// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Live RemovePet continuation for states with no pending gameplay callbacks.
//! Source: Player.cpp:20864–20962, Unit.cpp:6028–6215, Pet.cpp:452–546
//! at a5f8da2ebf5424bf0450ca4e08843ecbf72577bd.

use wow_constants::UnitFlags;
use wow_core::ObjectGuid;
use wow_entities::{Pet, Player};
use wow_persistence::PersistenceOutcomeLikeCpp;
use wow_world_lifecycle::PlayerMoneyCommitCancellationFenceLikeCpp;
use wow_world_quest::QuestSessionHostLikeCpp;

use super::save::{PreparedPetSave, prepare_pet_save};

fn validate_owner(
    current_owner: Option<ObjectGuid>,
    owner: ObjectGuid,
) -> Result<(), &'static str> {
    if current_owner != Some(owner) {
        return Err("live pet Player incarnation is unavailable");
    }
    Ok(())
}

fn prepare(player: &Player, pet: &Pet, now: u32) -> Result<PreparedPetSave, &'static str> {
    if player.gameplay_state().pet_guid != Some(pet.creature().guid())
        || pet.owner_guid() != player.guid()
        || pet.is_removed()
    {
        return Err("live pet removal ownership changed");
    }
    if player.gameplay_state().group.is_some() {
        return Err("live pet group-update continuation is unavailable");
    }
    if pet.created_by_spell_id_like_cpp() != 0 {
        return Err("live pet summon-spell continuation is unavailable");
    }
    let control = &player.unit().subsystems().control;
    let guid = pet.creature().guid();
    if control.controlled_guids.iter().any(|other| *other != guid)
        || player
            .unit()
            .minion_guid_like_cpp()
            .is_some_and(|other| other != guid)
        || (!control.pet_guid().is_empty() && control.pet_guid() != guid)
    {
        return Err("live pet secondary-minion continuation is unavailable");
    }
    let unit = pet.creature().unit();
    let subsystems = unit.subsystems();
    let combat = &subsystems.combat;
    if unit.attacking().is_some()
        || !combat.attackers.is_empty()
        || !combat.pve_refs.is_empty()
        || !combat.pvp_refs.is_empty()
        || !combat.threat_refs.is_empty()
        || !combat.threat.is_empty()
        || !combat.threatened_by_me.is_empty()
        || unit.unit_flags_like_cpp().contains(UnitFlags::IN_COMBAT)
        || !subsystems.spells.current_spells.is_empty()
        || !subsystems.control.controlled_guids.is_empty()
        || !subsystems.control.owned_gameobjects.is_empty()
        || subsystems.vehicle.vehicle_guid.is_some()
        || subsystems.vehicle.kit.is_some()
    {
        return Err("live pet combat or control cleanup continuation is unavailable");
    }
    let pets = player.pet_lifecycle_state_like_cpp();
    prepare_pet_save(pet, &pets.stable, pets.temporary_mount_react_state, now)
}

pub(super) async fn remove_quiescent_pet<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    pet_guid: ObjectGuid,
) -> Result<(), &'static str> {
    let core = host.core();
    let handle = core
        .player_handle_like_cpp
        .ok_or("live pet owner handle is unavailable")?;
    let owner = core.player_guid().ok_or("live pet owner is unavailable")?;
    let key = core
        .current_canonical_player_map_key_like_cpp()
        .ok_or("live pet Map is unavailable")?;
    let manager = core
        .canonical_map_manager
        .as_ref()
        .ok_or("live pet Map manager is unavailable")?
        .clone();
    let now = wow_world_core::session::connection_identity::unix_now() as u32;
    let captured = {
        let manager = manager.lock().map_err(|_| "live pet Map lock failed")?;
        validate_owner(manager.with_player_like_cpp(handle, Player::guid), owner)?;
        let map = manager
            .find_map(key.map_id, key.instance_id)
            .ok_or("live pet Map disappeared")?
            .map();
        let player = map
            .get_typed_player(owner)
            .ok_or("live pet owner left Map")?;
        let pet = map.get_typed_pet(pet_guid).ok_or("live pet disappeared")?;
        prepare(player, pet, now)?
    };
    let lifecycle = QuestSessionHostLikeCpp::lifecycle(host);
    // Unknown loads must never become destructive empty replacement writes.
    if !lifecycle
        .pet_load_auras_for_pet_number_like_cpp(captured.pet_number)
        .is_some_and(Vec::is_empty)
        || !lifecycle
            .pet_load_aura_effects_for_pet_number_like_cpp(captured.pet_number)
            .is_some_and(Vec::is_empty)
        || lifecycle
            .pet_load_spells_for_pet_number_like_cpp(captured.pet_number)
            .is_none()
        || lifecycle
            .pet_load_spell_cooldowns_for_pet_number_like_cpp(captured.pet_number)
            .is_none()
        || lifecycle
            .pet_load_spell_charges_for_pet_number_like_cpp(captured.pet_number)
            .is_none()
    {
        return Err("live pet persistence input is incomplete or contains auras");
    }
    let port = lifecycle
        .player_lifecycle_port_like_cpp()
        .ok_or("live pet persistence port is unavailable")?
        .clone();
    let tracker = lifecycle
        .durable_loot_money_persistence_tracker_like_cpp()
        .clone();
    if tracker.is_indeterminate_like_cpp() {
        return Err("live pet removal is fenced by unresolved persistence");
    }
    // Register before the cancellation fence so its Drop marks an unknown
    // result before releasing a logout/full-save waiter. No money completion
    // is synthesized: dropping this registration only releases the waiter.
    let _in_flight = tracker
        .begin_like_cpp()
        .map_err(|_| "live pet removal persistence admission is closed")?;
    let mut fence = PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(tracker);
    {
        let mut manager = manager.lock().map_err(|_| "live pet Map lock failed")?;
        validate_owner(manager.with_player_like_cpp(handle, Player::guid), owner)?;
        let map = manager
            .find_map_mut(key.map_id, key.instance_id)
            .ok_or("live pet Map disappeared")?
            .map_mut();
        let player = map
            .get_typed_player(owner)
            .ok_or("live pet owner left Map")?;
        let pet = map.get_typed_pet(pet_guid).ok_or("live pet disappeared")?;
        if prepare(player, pet, now)? != captured {
            return Err("live pet changed before removal");
        }
        fence.arm_like_cpp();
        // CreatedBySpell is zero in this branch, so the refund loop is empty.
        let player = map.get_typed_player_mut(owner).expect("owner prevalidated");
        if player.battleground_instance_id_like_cpp().is_none() {
            player
                .pet_lifecycle_state_mut_like_cpp()
                .temporary_unsummoned_pet_number = 0;
        }
        // All indexed combat participants are empty; no exit callbacks arise.
        let stopped = map.stop_pet_combat_like_cpp(pet_guid)?;
        debug_assert!(stopped.attacks_stopped.is_empty() && stopped.exited_combat.is_empty());
    }
    for transaction in [
        captured.state_transaction.clone(),
        captured.row_transaction.clone(),
    ] {
        if !matches!(
            port.persist_pet_save_transaction_like_cpp(transaction)
                .await,
            PersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            return Err("live pet save failed or has an unknown commit result");
        }
        if host.core().player_handle_like_cpp != Some(handle)
            || host.core().player_guid() != Some(owner)
        {
            return Err("live pet owner changed during save");
        }
        let manager = manager.lock().map_err(|_| "live pet Map lock failed")?;
        validate_owner(manager.with_player_like_cpp(handle, Player::guid), owner)?;
        let map = manager
            .find_map(key.map_id, key.instance_id)
            .ok_or("live pet Map disappeared during save")?
            .map();
        if prepare(
            map.get_typed_player(owner)
                .ok_or("live pet owner left Map")?,
            map.get_typed_pet(pet_guid).ok_or("live pet disappeared")?,
            now,
        )? != captured
        {
            return Err("live pet changed during save");
        }
    }
    {
        let mut manager = manager.lock().map_err(|_| "live pet Map lock failed")?;
        validate_owner(manager.with_player_like_cpp(handle, Player::guid), owner)?;
        let map = manager
            .find_map_mut(key.map_id, key.instance_id)
            .ok_or("live pet Map disappeared after save")?
            .map_mut();
        let player = map
            .get_typed_player(owner)
            .ok_or("live pet owner left Map")?;
        let pet = map.get_typed_pet(pet_guid).ok_or("live pet disappeared")?;
        if prepare(player, pet, now)? != captured {
            return Err("live pet changed before cleanup");
        }
        let player = map.get_typed_player_mut(owner).expect("owner prevalidated");
        let pets = player.pet_lifecycle_state_mut_like_cpp();
        let index = pets
            .stable
            .current_pet_index
            .ok_or("live pet stable selection disappeared")?;
        pets.stable.active_pets[index as usize] = Some(captured.info);
        pets.stable.current_pet_index = None;
        player.set_pet_guid_like_cpp(None);
        let unit = player.unit_mut();
        let clear_minion = unit.minion_guid_like_cpp() == Some(pet_guid);
        {
            let control = &mut unit.subsystems_mut().control;
            control.controlled_guids.remove(&pet_guid);
            control.set_pet_guid(ObjectGuid::EMPTY);
        }
        if clear_minion {
            unit.set_minion_guid_like_cpp(None);
        }
        let mut flags = unit.unit_flags_like_cpp();
        flags.remove(UnitFlags::PET_IN_COMBAT);
        unit.set_unit_flags_like_cpp(flags);
        let removed = map.add_object_to_remove_list_like_cpp(pet_guid);
        if removed.missing_or_stale || removed.unsupported_kind.is_some() {
            return Err("live pet deferred removal failed");
        }
        map.get_typed_pet_mut(pet_guid)
            .expect("pet prevalidated")
            .set_removed(true);
    }
    host.core()
        .send_packet(&wow_packet::packets::pet::ClearPetSpells);
    fence.disarm_like_cpp();
    Ok(())
}
