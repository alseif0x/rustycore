// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Live RemovePet continuation for bounded native pet states.
//! Source: Player.cpp:20864–20962, Unit.cpp:6028–6215, Pet.cpp:452–546
//! at a5f8da2ebf5424bf0450ca4e08843ecbf72577bd. Forever aura-save review:
//! Player.cpp:22322–22408, Pet.cpp:452–546,1301–1345 and Unit.cpp:4401–4436
//! at 02245dcd245e7433e524577656177723d3e4992e.

use wow_constants::UnitFlags;
use wow_core::ObjectGuid;
use wow_entities::{GROUP_UPDATE_FLAG_PET_LIKE_CPP, Pet, Player};
use wow_persistence::{
    PersistenceOutcomeLikeCpp, PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetSaveTransactionLikeCpp,
};
use wow_world_entities::ResurrectionOperationCatalogs;
use wow_world_inventory::VendorSessionHostLikeCpp;
use wow_world_lifecycle::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp,
};
use wow_world_quest::{QuestItemStorageHostLikeCpp, QuestSessionHostLikeCpp};
use wow_world_spell::PlayerCastHostLikeCpp;

use crate::TrainerSessionHostLikeCpp;

use super::{
    pet_spell_reagents,
    save::{PreparedPetSave, prepare_pet_save},
};

fn validate_owner(
    current_owner: Option<ObjectGuid>,
    owner: ObjectGuid,
) -> Result<(), &'static str> {
    if current_owner != Some(owner) {
        return Err("live pet Player incarnation is unavailable");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
struct PreparedLivePetRemoval {
    save: PreparedPetSave,
    created_by_spell: u32,
    in_battleground: bool,
}

impl PreparedLivePetRemoval {
    fn after_aura_cleanup(&self) -> Self {
        let mut cleaned = self.clone();
        let PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
            auras,
            aura_effects,
            ..
        } = &mut cleaned.save.state_transaction
        else {
            unreachable!("live pet state save always includes auras")
        };
        auras.clear();
        aura_effects.clear();
        cleaned
    }
}

fn persisted_aura_input_matches(
    save: &PreparedPetSave,
    loaded_auras: Option<&Vec<CharacterPetAuraRowLikeCpp>>,
    loaded_effects: Option<&Vec<CharacterPetAuraEffectRowLikeCpp>>,
) -> bool {
    let PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
        auras: expected_auras,
        aura_effects: expected_effects,
        ..
    } = &save.state_transaction
    else {
        return false;
    };
    let (Some(loaded_auras), Some(loaded_effects)) = (loaded_auras, loaded_effects) else {
        return false;
    };
    let mut loaded_auras: Vec<_> = loaded_auras
        .iter()
        .map(|aura| PlayerPetAuraLoadRowLikeCpp {
            caster_guid_binary: aura.caster_guid.to_raw_bytes().to_vec(),
            spell_id: aura.spell_id,
            effect_mask: aura.effect_mask,
            recalculate_mask: aura.recalculate_mask,
            difficulty: aura.difficulty,
            stack_count: aura.stack_count,
            max_duration_ms: aura.max_duration_ms,
            remain_time_ms: aura.remain_time_ms,
            remain_charges: aura.remain_charges,
        })
        .collect();
    loaded_auras.sort_by_key(|aura| {
        (
            aura.caster_guid_binary.clone(),
            aura.spell_id,
            aura.effect_mask,
        )
    });
    let mut loaded_effects: Vec<_> = loaded_effects
        .iter()
        .map(|effect| PlayerPetAuraEffectLoadRowLikeCpp {
            caster_guid_binary: effect.caster_guid.to_raw_bytes().to_vec(),
            spell_id: effect.spell_id,
            effect_mask: effect.effect_mask,
            effect_index: effect.effect_index,
            amount: effect.amount,
            base_amount: effect.base_amount,
        })
        .collect();
    loaded_effects.sort_by_key(|effect| {
        (
            effect.caster_guid_binary.clone(),
            effect.spell_id,
            effect.effect_mask,
            effect.effect_index,
        )
    });
    expected_auras == &loaded_auras && expected_effects == &loaded_effects
}

fn prepare(player: &Player, pet: &Pet, now: u32) -> Result<PreparedLivePetRemoval, &'static str> {
    if player.gameplay_state().pet_guid != Some(pet.creature().guid())
        || pet.owner_guid() != player.guid()
        || pet.is_removed()
    {
        return Err("live pet removal ownership changed");
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
    Ok(PreparedLivePetRemoval {
        save: prepare_pet_save(pet, &pets.stable, pets.temporary_mount_react_state, now)?,
        created_by_spell: pet.created_by_spell_id_like_cpp(),
        in_battleground: player.battleground_instance_id_like_cpp().is_some(),
    })
}

pub(super) async fn remove_quiescent_pet<H>(
    host: &mut H,
    pet_guid: ObjectGuid,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> Result<(), &'static str>
where
    H: QuestItemStorageHostLikeCpp
        + TrainerSessionHostLikeCpp
        + PlayerCastHostLikeCpp
        + VendorSessionHostLikeCpp,
{
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
    // Unknown or divergent loads must never become destructive replacement writes.
    if !persisted_aura_input_matches(
        &captured.save,
        lifecycle.pet_load_auras_for_pet_number_like_cpp(captured.save.pet_number),
        lifecycle.pet_load_aura_effects_for_pet_number_like_cpp(captured.save.pet_number),
    ) || lifecycle
        .pet_load_spells_for_pet_number_like_cpp(captured.save.pet_number)
        .is_none()
        || lifecycle
            .pet_load_spell_cooldowns_for_pet_number_like_cpp(captured.save.pet_number)
            .is_none()
        || lifecycle
            .pet_load_spell_charges_for_pet_number_like_cpp(captured.save.pet_number)
            .is_none()
    {
        return Err("live pet persistence input is incomplete or does not match native auras");
    }
    let cleaned = captured.after_aura_cleanup();
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
    if !captured.in_battleground {
        // C++ returns summon reagents before CombatStop and SavePetToDB. The
        // reagent operation arms this same fence before any quest/item callback.
        let difficulty = host.core().current_map_difficulty_id_like_cpp();
        let reagents = pet_spell_reagents(
            host.hub_ref().catalogs,
            captured.created_by_spell,
            difficulty,
        )?;
        for (entry, count) in reagents {
            crate::return_pet_reagent_like_cpp(
                host,
                &mut fence,
                catalogs.item_guid_generator,
                catalogs.creature_spawns,
                entry,
                count,
            )
            .await?;
        }
    }
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
        let player = map.get_typed_player_mut(owner).expect("owner prevalidated");
        if !captured.in_battleground {
            player
                .pet_lifecycle_state_mut_like_cpp()
                .temporary_unsummoned_pet_number = 0;
        }
        // All indexed combat participants are empty; no exit callbacks arise.
        let stopped = map.stop_pet_combat_like_cpp(pet_guid)?;
        debug_assert!(stopped.attacks_stopped.is_empty() && stopped.exited_combat.is_empty());
        if !map
            .get_typed_pet_mut(pet_guid)
            .expect("pet prevalidated")
            .creature_mut()
            .unit_mut()
            .subsystems_mut()
            .auras
            .remove_all_persisted_pet_auras_after_save_like_cpp()
        {
            return Err("live pet aura cleanup left unsupported state");
        }
        let player = map
            .get_typed_player(owner)
            .ok_or("live pet owner left Map")?;
        let pet = map.get_typed_pet(pet_guid).ok_or("live pet disappeared")?;
        if prepare(player, pet, now)? != cleaned {
            return Err("live pet aura cleanup changed another saved field");
        }
    }
    for transaction in [
        captured.save.state_transaction.clone(),
        captured.save.row_transaction.clone(),
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
        )? != cleaned
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
        if prepare(player, pet, now)? != cleaned {
            return Err("live pet changed before cleanup");
        }
        let player = map.get_typed_player_mut(owner).expect("owner prevalidated");
        let pets = player.pet_lifecycle_state_mut_like_cpp();
        let index = pets
            .stable
            .current_pet_index
            .ok_or("live pet stable selection disappeared")?;
        pets.stable.active_pets[index as usize] = Some(captured.save.info);
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
    {
        let mut manager = manager.lock().map_err(|_| "live pet Map lock failed")?;
        validate_owner(manager.with_player_like_cpp(handle, Player::guid), owner)?;
        let map = manager
            .find_map_mut(key.map_id, key.instance_id)
            .ok_or("live pet Map disappeared during group update")?
            .map_mut();
        let player = map
            .get_typed_player_mut(owner)
            .ok_or("live pet owner left Map during group update")?;
        if player.gameplay_state().group.is_some() {
            player.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_LIKE_CPP);
        }
    }
    fence.disarm_like_cpp();
    Ok(())
}
