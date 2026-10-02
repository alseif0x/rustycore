//! Session-owned battle-pet data transfer objects shared with the world shell.

use wow_core::ObjectGuid;

pub const BATTLE_PET_SLOT_COUNT_LIKE_CPP: usize = 3;
pub const DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP: u8 = 3;
pub const BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP: u16 = 0x01;
pub const BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlePetSaveInfoLikeCpp {
    New,
    Changed,
    #[allow(dead_code)]
    Unchanged,
    Removed,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlePetCageItemLikeCpp {
    pub item_id: u32,
    pub species_id: u32,
    pub breed_data: u32,
    pub level: u16,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlePetCalculatedStatsLikeCpp {
    pub max_health: u32,
    pub power: u32,
    pub speed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlePetLevelCriteriaLikeCpp {
    pub species: u32,
    pub level: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedBattlePetDataLikeCpp {
    pub species: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub breed: u16,
    pub level: u16,
    pub exp: u16,
    pub flags: u16,
    pub power: u32,
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub quality: u8,
    pub owner_info: Option<wow_packet::packets::misc::BattlePetJournalPetOwnerInfo>,
    pub name: String,
    pub name_timestamp: i64,
    pub declined_names: Option<wow_packet::packets::misc::DeclinedNamesLikeCpp>,
    pub save_info: RepresentedBattlePetSaveInfoLikeCpp,
}

/// Represented ObjectAccessor/TempSummon facts needed by
/// `WorldSession::HandleQueryBattlePetName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlePetQueryCompanionLikeCpp {
    pub creature_id: i32,
    pub name_timestamp: i64,
    pub is_summon: bool,
    pub owner_is_player: bool,
    pub battle_pet_companion_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedBattlePetSlotLikeCpp {
    pub pet_guid: Option<ObjectGuid>,
    pub collar_id: u32,
    pub index: u8,
    pub locked: bool,
}

impl RepresentedBattlePetSlotLikeCpp {
    pub fn locked_empty(index: u8) -> Self {
        Self {
            pet_guid: None,
            collar_id: 0,
            index,
            locked: true,
        }
    }

    pub fn packet_slot_like_cpp(&self) -> wow_packet::packets::misc::BattlePetJournalSlot {
        wow_packet::packets::misc::BattlePetJournalSlot {
            pet_guid: self
                .pet_guid
                .unwrap_or_else(wow_packet::packets::misc::empty_battle_pet_guid_like_cpp),
            collar_id: self.collar_id,
            index: self.index,
            locked: self.locked,
        }
    }
}

impl RepresentedBattlePetDataLikeCpp {
    pub fn minimal_like_cpp(flags: u16, save_info: RepresentedBattlePetSaveInfoLikeCpp) -> Self {
        Self {
            species: 0,
            creature_id: 0,
            display_id: 0,
            breed: 0,
            level: 0,
            exp: 0,
            flags,
            power: 0,
            health: 0,
            max_health: 0,
            speed: 0,
            quality: 0,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info,
        }
    }

    pub fn packet_info_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> wow_packet::packets::misc::BattlePetJournalPet {
        wow_packet::packets::misc::BattlePetJournalPet {
            guid,
            species: self.species,
            creature_id: self.creature_id,
            display_id: self.display_id,
            breed: self.breed,
            level: self.level,
            exp: self.exp,
            flags: self.flags,
            power: self.power,
            health: self.health,
            max_health: self.max_health,
            speed: self.speed,
            quality: self.quality,
            owner_info: self.owner_info,
            name: self.name.clone(),
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_dismissed_critter_guids_like_cpp(&self) -> &[ObjectGuid] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_dismissed_critter_guids_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn represented_critter_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().critter_guid_like_cpp())
        {
            return guid;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_critter_guid_like_cpp
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            None
        }
    }
}

impl crate::session::HubMut<'_> {
    pub fn set_represented_critter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        if self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_critter_guid_like_cpp(guid);
            })
            .is_some()
        {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_critter_guid_like_cpp = guid;
        }
    }

    /// C++ `WorldSession::HandleDismissCritter`, represented at the ownership gate.
    ///
    /// Full `ObjectAccessor::GetCreatureOrPetOrVehicle`, `Unit::IsSummon` and
    /// `TempSummon::UnSummon` remain part of the live companion runtime. This
    /// represented path preserves the C++ no-response semantics and only acts
    /// when the requested GUID is the player's active critter.
    pub fn represented_dismiss_critter_like_cpp(&mut self, critter_guid: ObjectGuid) -> bool {
        if self.shared().represented_critter_guid_like_cpp() != Some(critter_guid) {
            return false;
        }

        if let Some(companion) = self
            .shared()
            .represented_battle_pet_query_companion_like_cpp(critter_guid)
            && let Some(battle_pet_guid) = companion.battle_pet_companion_guid
            && self
                .shared()
                .represented_summoned_battle_pet_guid_like_cpp()
                == Some(battle_pet_guid)
        {
            let _ = self.set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        self.set_represented_critter_guid_like_cpp(None);
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_dismissed_critter_guids_like_cpp
            .push(critter_guid);
        true
    }
}
