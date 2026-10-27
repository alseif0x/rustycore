// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only native pet-save projection for the bounded empty-aura removal path.

use wow_entities::{
    Pet, PetSpellState, PetSpellType, PetStable, PetStableInfo, ReactState,
    SpellHistoryPetSaveOperationLikeCpp, MAX_ACTIVE_PETS,
};
use wow_persistence::{
    PlayerPetSaveTransactionLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellSaveOperationLikeCpp,
    PlayerPetStableLoadRowLikeCpp,
};

// C++ `UnitDefines.h:104`: UNIT_PET_FLAG_CAN_BE_RENAMED = 0x01 at the pinned SHA below.
const UNIT_PET_FLAG_CAN_BE_RENAMED_LIKE_CPP: u8 = 0x01;

/// Immutable inputs captured for the two ordered native Pet save transactions.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PreparedPetSave {
    pub(super) pet_number: u32,
    pub(super) info: PetStableInfo,
    pub(super) state_transaction: PlayerPetSaveTransactionLikeCpp,
    pub(super) row_transaction: PlayerPetSaveTransactionLikeCpp,
}

/// Prepare the NotInSlot native Pet save phases without changing Pet state.
///
/// The caller must establish that the loaded pet aura and aura-effect row
/// authorities are both known empty. Local-container emptiness below is not
/// persistence authority. C++ anchors: `Pet.cpp:452–536, 545–562, 1129–1176`
/// and `SpellHistory.cpp:182–218` at source SHA
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
pub(super) fn prepare_pet_save(
    pet: &Pet,
    stable: &PetStable,
    temporary_react: Option<u8>,
    now: u32,
) -> Result<PreparedPetSave, &'static str> {
    let creature = pet.creature();
    if creature.entry() == 0 {
        return Err("pet save requires a nonzero creature entry");
    }
    if !pet.is_controlled() {
        return Err("pet save requires a fully controlled pet");
    }

    let owner_guid = pet.owner_guid();
    if !owner_guid.is_player() {
        return Err("pet save requires a Player owner");
    }

    let charm_info = creature
        .unit()
        .subsystems()
        .control
        .charm_info
        .as_ref()
        .ok_or("pet save requires CharmInfo")?;
    let pet_number = charm_info.pet_number;
    if pet_number == 0 {
        return Err("pet save requires a nonzero CharmInfo pet number");
    }

    let active_slot = stable
        .current_pet_index
        .ok_or("pet save current stable pet slot is unavailable")?;
    if active_slot >= MAX_ACTIVE_PETS as u32 {
        return Err("pet save current stable pet slot is invalid");
    }
    let current_stable_pet = stable
        .active_pets
        .get(active_slot as usize)
        .and_then(Option::as_ref)
        .ok_or("pet save current stable pet is unavailable")?;
    if current_stable_pet.pet_number != pet_number {
        return Err("pet save CharmInfo number does not match the current stable pet");
    }

    let forced_react_state = match temporary_react {
        None => None,
        Some(0) => Some(ReactState::Passive),
        Some(1) => Some(ReactState::Defensive),
        Some(2) => Some(ReactState::Aggressive),
        Some(_) => return Err("pet save temporary react state is invalid"),
    };

    let auras = &creature.unit().subsystems().auras;
    if !auras.has_no_local_aura_state_like_cpp()
        || !auras.runtime_applications_like_cpp().is_empty()
    {
        return Err("pet save requires an empty native aura container");
    }

    let mut spells = Vec::new();
    for (&spell_id, spell) in pet.spells() {
        if spell.spell_type == PetSpellType::Family {
            continue;
        }

        match spell.state {
            PetSpellState::Removed => {
                spells.push(PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id });
            }
            PetSpellState::Changed => {
                spells.push(PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id });
                spells.push(PlayerPetSpellSaveOperationLikeCpp::Insert {
                    spell_id,
                    active: spell.active as u8,
                });
            }
            PetSpellState::New => {
                spells.push(PlayerPetSpellSaveOperationLikeCpp::Insert {
                    spell_id,
                    active: spell.active as u8,
                });
            }
            PetSpellState::Unchanged => {}
        }
    }

    let mut cooldowns = Vec::new();
    let mut charges = Vec::new();
    let history_operations = creature
        .unit()
        .subsystems()
        .spells
        .history
        .save_pet_spell_history_plan_like_cpp(pet_number);
    for operation in history_operations {
        match operation {
            SpellHistoryPetSaveOperationLikeCpp::DeleteCooldowns { .. }
            | SpellHistoryPetSaveOperationLikeCpp::DeleteCharges { .. } => {}
            SpellHistoryPetSaveOperationLikeCpp::InsertCooldown {
                spell_id,
                cooldown_end_time_secs,
                category_id,
                category_end_time_secs,
                ..
            } => cooldowns.push(PlayerPetSpellCooldownLoadRowLikeCpp {
                spell_id,
                cooldown_end_unix_secs: cooldown_end_time_secs,
                category_id,
                category_end_unix_secs: category_end_time_secs,
            }),
            SpellHistoryPetSaveOperationLikeCpp::InsertCharge {
                category_id,
                recharge_start_time_secs,
                recharge_end_time_secs,
                ..
            } => charges.push(PlayerPetSpellChargeLoadRowLikeCpp {
                category_id,
                recharge_start_unix_secs: recharge_start_time_secs,
                recharge_end_unix_secs: recharge_end_time_secs,
            }),
        }
    }

    let state_transaction = PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
        pet_number,
        auras: Vec::new(),
        aura_effects: Vec::new(),
        spells,
        cooldowns,
        charges,
    };

    let can_be_renamed =
        creature.unit().pet_flags_like_cpp() & UNIT_PET_FLAG_CAN_BE_RENAMED_LIKE_CPP != 0;
    let info = pet.fill_pet_info_like_cpp(
        pet_number,
        &charm_info.action_bar,
        forced_react_state,
        can_be_renamed,
        now,
        pet.created_by_spell_id_like_cpp(),
    );
    let owner_counter = u64::try_from(owner_guid.counter())
        .map_err(|_| "pet save Player owner counter is invalid")?;
    let slot = i16::try_from(active_slot).map_err(|_| "pet save current stable slot is invalid")?;
    let row_transaction = PlayerPetSaveTransactionLikeCpp::PetRow {
        owner_guid: owner_counter,
        row: PlayerPetStableLoadRowLikeCpp {
            pet_number: info.pet_number,
            creature_id: info.creature_id,
            display_id: info.display_id,
            level: info.level,
            experience: info.experience,
            react_state: info.react_state as u8,
            slot,
            name: info.name.clone(),
            was_renamed: info.was_renamed,
            health: info.health,
            mana: info.mana,
            action_bar: info.action_bar.clone(),
            last_save_time: info.last_save_time,
            created_by_spell_id: info.created_by_spell_id,
            pet_type: info.pet_type as u8,
            specialization_id: info.specialization_id,
        },
    };

    Ok(PreparedPetSave {
        pet_number,
        info,
        state_transaction,
        row_transaction,
    })
}

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;
