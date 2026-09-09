//! Pet lifecycle, aura and persistence state regression scenarios.
//!
//! Separated from the pet.rs root under #636.

use super::*;

fn owner_guid() -> ObjectGuid {
    ObjectGuid::create_global(wow_core::guid::HighGuid::Player, 0, 1)
}

fn pet_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::new((wow_core::guid::HighGuid::Pet as i64) << 58, counter)
}

fn pet_info(pet_number: u32, creature_id: u32) -> PetStableInfo {
    PetStableInfo {
        pet_number,
        creature_id,
        ..PetStableInfo::default()
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
