// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only native pet-save projection for the bounded live removal path.

use wow_entities::{
    AppliedAuraRef, MAX_ACTIVE_PETS, Pet, PetAuraSaveEffectLikeCpp, PetAuraSaveOperationLikeCpp,
    PetAuraSaveRefLikeCpp, PetSpellState, PetSpellType, PetStable, PetStableInfo, ReactState,
    SpellHistoryPetSaveOperationLikeCpp,
};
use wow_persistence::{
    PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetSaveTransactionLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellSaveOperationLikeCpp,
    PlayerPetStableLoadRowLikeCpp,
};

// C++ `UnitDefines.h:104`: UNIT_PET_FLAG_CAN_BE_RENAMED = 0x01 at the pinned SHA below.
const UNIT_PET_FLAG_CAN_BE_RENAMED_LIKE_CPP: u8 = 0x01;

fn prepare_pet_aura_save(
    pet: &Pet,
    pet_number: u32,
) -> Result<
    (
        Vec<PlayerPetAuraLoadRowLikeCpp>,
        Vec<PlayerPetAuraEffectLoadRowLikeCpp>,
    ),
    &'static str,
> {
    let auras = &pet.creature().unit().subsystems().auras;
    let mut represented = Vec::new();
    for owned in &auras.owned_auras {
        if owned.item_caster_guid.is_some() {
            return Err("pet save requires complete persisted native aura state");
        }
        let aura_ref = owned.aura_ref();
        let loaded = auras
            .loaded_aura_states_like_cpp
            .get(&aura_ref)
            .ok_or("pet save requires complete persisted native aura state")?;
        let difficulty = loaded
            .cast_difficulty
            .ok_or("pet save requires complete persisted native aura state")?;
        let mut applications = auras
            .applied_auras
            .iter()
            .filter(|applied| applied.aura_ref() == aura_ref);
        let applied = applications
            .next()
            .ok_or("pet save requires complete persisted native aura state")?;
        if applications.next().is_some()
            || auras.visible_auras.get(&applied.slot) != Some(&aura_ref)
        {
            return Err("pet save requires complete persisted native aura state");
        }
        let mut effects = Vec::new();
        for effect_index in 0..wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP {
            let effect_mask = 1_u32 << effect_index;
            if applied.effect_mask & effect_mask == 0 {
                continue;
            }
            let effect_index = u8::try_from(effect_index)
                .map_err(|_| "pet save requires complete persisted native aura state")?;
            let effect_ref =
                AppliedAuraRef::new(owned.spell_id, owned.caster_guid, applied.slot, effect_mask);
            let amount = *auras
                .applied_aura_amounts
                .get(&effect_ref)
                .ok_or("pet save requires complete persisted native aura state")?;
            let base_amount = loaded
                .base_amounts
                .iter()
                .find_map(|(index, amount)| (*index == effect_index).then_some(*amount))
                .ok_or("pet save requires complete persisted native aura state")?;
            effects.push(PetAuraSaveEffectLikeCpp {
                effect_index,
                amount,
                base_amount,
            });
        }
        represented.push(PetAuraSaveRefLikeCpp {
            caster_guid: owned.caster_guid,
            spell_id: owned.spell_id,
            effect_mask: applied.effect_mask,
            recalculate_mask: loaded.recalculate_mask,
            difficulty,
            stack_count: loaded.stack_amount,
            max_duration_ms: loaded.max_duration_ms,
            duration_ms: loaded.duration_ms,
            charges: loaded.charges,
            can_be_saved: true,
            is_pet_aura: false,
            effects,
        });
    }
    represented.sort_by_key(|aura| {
        (
            aura.caster_guid.to_raw_bytes(),
            aura.spell_id,
            aura.effect_mask,
        )
    });
    let mut cleaned = auras.clone();
    if !cleaned.remove_all_persisted_pet_auras_after_save_like_cpp() {
        return Err("pet save requires complete persisted native aura state");
    }

    let mut aura_rows = Vec::new();
    let mut effect_rows = Vec::new();
    for operation in Pet::save_auras_plan_like_cpp(pet_number, pet.creature().guid(), &represented)
    {
        match operation {
            PetAuraSaveOperationLikeCpp::InsertAura {
                caster_guid,
                spell_id,
                effect_mask,
                recalculate_mask,
                difficulty,
                stack_count,
                max_duration_ms,
                duration_ms,
                charges,
                ..
            } => aura_rows.push(PlayerPetAuraLoadRowLikeCpp {
                caster_guid_binary: caster_guid.to_raw_bytes().to_vec(),
                spell_id,
                effect_mask,
                recalculate_mask,
                difficulty,
                stack_count,
                max_duration_ms,
                remain_time_ms: duration_ms,
                remain_charges: charges,
            }),
            PetAuraSaveOperationLikeCpp::InsertAuraEffect {
                caster_guid,
                spell_id,
                effect_mask,
                effect_index,
                amount,
                base_amount,
                ..
            } => effect_rows.push(PlayerPetAuraEffectLoadRowLikeCpp {
                caster_guid_binary: caster_guid.to_raw_bytes().to_vec(),
                spell_id,
                effect_mask,
                effect_index,
                amount,
                base_amount,
            }),
            PetAuraSaveOperationLikeCpp::DeleteAuraEffects { .. }
            | PetAuraSaveOperationLikeCpp::DeleteAuras { .. } => {}
        }
    }
    Ok((aura_rows, effect_rows))
}

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
/// The caller must establish that the loaded pet aura and aura-effect rows
/// match this projection. Only persisted native auras with complete provenance
/// are admitted; other runtime aura state fails closed. C++ anchors:
/// `Pet.cpp:452–536, 1129–1176` and `SpellHistory.cpp:182–218` at source SHA
/// `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`; Forever `Pet.cpp:452–546,
/// 1301–1345` at `02245dcd245e7433e524577656177723d3e4992e`.
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

    let (auras, aura_effects) = prepare_pet_aura_save(pet, pet_number)?;

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
        auras,
        aura_effects,
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
