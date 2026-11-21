//! Binding-time save reconstruction for the Icecrown Citadel instance script.
//!
//! C++ references:
//! - `scripts/Northrend/IcecrownCitadel/icecrown_citadel.h`: script name,
//!   `ICCv1` header and encounter ids.
//! - `scripts/Northrend/IcecrownCitadel/instance_icecrown_citadel.cpp:1074-1205`:
//!   required-boss prerequisites.
//! - `game/Instances/InstanceScript.cpp:490-520`: load normalization,
//!   prerequisite repair and `AfterDataLoad` ordering.

use super::*;

pub const ICC_MAP_ID_LIKE_CPP: u32 = 631;
pub const ICC_INSTANCE_SCRIPT_NAME_LIKE_CPP: &str = "instance_icecrown_citadel";
/// C++ `DIFFICULTY_10_N`, `DIFFICULTY_25_N`, `DIFFICULTY_10_HC`,
/// `DIFFICULTY_25_HC` from `game/DataStores/DBCEnums.h`.
pub const ICC_DIFFICULTY_IDS_LIKE_CPP: [u8; 4] = [3, 4, 5, 6];
const ICC_SAVE_HEADER: &str = "ICCv1";
const ICC_BOSS_COUNT: usize = 13;

const BOSS_LORD_MARROWGAR: u32 = 0;
const BOSS_LADY_DEATHWHISPER: u32 = 1;
const BOSS_ICECROWN_GUNSHIP_BATTLE: u32 = 2;
const BOSS_DEATHBRINGER_SAURFANG: u32 = 3;
const BOSS_FESTERGUT: u32 = 4;
const BOSS_ROTFACE: u32 = 5;
const BOSS_PROFESSOR_PUTRICIDE: u32 = 6;
const BOSS_BLOOD_PRINCE_COUNCIL: u32 = 7;
const BOSS_BLOOD_QUEEN_LANA_THEL: u32 = 8;
const BOSS_VALITHRIA_DREAMWALKER: u32 = 9;
const BOSS_SINDRAGOSA: u32 = 10;
const BOSS_THE_LICH_KING: u32 = 11;

/// ICC-specific portion of C++ `instance_icecrown_citadel_InstanceMapScript`
/// needed when `InstanceMap::CreateInstanceData` reconstructs a bound lock.
#[derive(Debug, Clone, PartialEq)]
pub struct IccInstanceScriptLikeCpp {
    base: InstanceScriptBase,
    coldflame_jets_state: Option<u32>,
    upper_spire_teleporter_active_state: Option<u32>,
}

impl IccInstanceScriptLikeCpp {
    /// Create ICC script data for an already-selected effective registration.
    ///
    /// `Some(lock_data)` denotes an attached, non-new lock. Its entrance is
    /// restored even if its save payload is empty; an empty payload follows
    /// C++ `Create()`. `None` follows the no-lock/new-lock `Create()` path.
    pub fn initialize_for_binding_like_cpp(
        difficulty_id: u8,
        lock_data: Option<&InstanceLockData>,
    ) -> Self {
        let mut base = InstanceScriptBase::new(u32::from(difficulty_id), ICC_BOSS_COUNT);
        base.set_header(ICC_SAVE_HEADER);

        let mut script = Self {
            base,
            coldflame_jets_state: None,
            upper_spire_teleporter_active_state: None,
        };

        let Some(lock_data) = lock_data else {
            script.base.create_like_cpp();
            return script;
        };

        script
            .base
            .set_entrance_location_like_cpp(lock_data.entrance_world_safe_loc_id);
        if lock_data.data.is_empty() {
            script.base.create_like_cpp();
            return script;
        }

        if script.base.load_save_data_like_cpp(&lock_data.data).is_ok() {
            script.repair_loaded_boss_prerequisites_like_cpp();
            script.after_data_load_like_cpp();
        }

        script
    }

    /// C++ ICC `CheckRequiredBosses` without its player-only permission bypass.
    pub fn check_required_bosses_like_cpp(&self, boss_id: u32) -> bool {
        match boss_id {
            BOSS_THE_LICH_KING => {
                if !self.check_plagueworks_like_cpp(boss_id)
                    || !self.check_crimson_halls_like_cpp(boss_id)
                    || !self.check_frostwing_halls_like_cpp(boss_id)
                {
                    return false;
                }
            }
            BOSS_SINDRAGOSA | BOSS_VALITHRIA_DREAMWALKER => {
                if !self.check_frostwing_halls_like_cpp(boss_id) {
                    return false;
                }
            }
            BOSS_BLOOD_QUEEN_LANA_THEL | BOSS_BLOOD_PRINCE_COUNCIL => {
                if !self.check_crimson_halls_like_cpp(boss_id) {
                    return false;
                }
            }
            BOSS_FESTERGUT | BOSS_ROTFACE | BOSS_PROFESSOR_PUTRICIDE => {
                if !self.check_plagueworks_like_cpp(boss_id) {
                    return false;
                }
            }
            _ => {}
        }

        self.check_lower_spire_like_cpp(boss_id)
    }

    pub fn boss_state_like_cpp(&self, boss_id: u32) -> EncounterState {
        self.base.boss_state(boss_id)
    }

    pub fn entrance_location_like_cpp(&self) -> u32 {
        self.base.entrance_location_like_cpp()
    }

    pub fn coldflame_jets_state_like_cpp(&self) -> Option<u32> {
        self.coldflame_jets_state
    }

    pub fn upper_spire_teleporter_active_state_like_cpp(&self) -> Option<u32> {
        self.upper_spire_teleporter_active_state
    }

    /// Current generic InstanceScript save bytes used by instance-lock binds.
    pub fn get_save_data_like_cpp(&self) -> String {
        self.base.get_save_data_like_cpp()
    }

    fn repair_loaded_boss_prerequisites_like_cpp(&mut self) {
        for boss_id in 0..ICC_BOSS_COUNT as u32 {
            if self.boss_state_like_cpp(boss_id) == EncounterState::Done
                && !self.check_required_bosses_like_cpp(boss_id)
            {
                self.base
                    .set_boss_state_like_cpp(boss_id, EncounterState::NotStarted);
            }
        }
    }

    fn after_data_load_like_cpp(&mut self) {
        if self.boss_state_like_cpp(BOSS_DEATHBRINGER_SAURFANG) == EncounterState::Done {
            let done = u32::from(EncounterState::Done as u8);
            self.coldflame_jets_state = Some(done);
            self.upper_spire_teleporter_active_state = Some(done);
        }
    }

    fn check_plagueworks_like_cpp(&self, boss_id: u32) -> bool {
        match boss_id {
            BOSS_THE_LICH_KING => {
                if self.boss_state_like_cpp(BOSS_PROFESSOR_PUTRICIDE) != EncounterState::Done {
                    return false;
                }
                self.boss_state_like_cpp(BOSS_FESTERGUT) == EncounterState::Done
                    && self.boss_state_like_cpp(BOSS_ROTFACE) == EncounterState::Done
            }
            BOSS_PROFESSOR_PUTRICIDE => {
                self.boss_state_like_cpp(BOSS_FESTERGUT) == EncounterState::Done
                    && self.boss_state_like_cpp(BOSS_ROTFACE) == EncounterState::Done
            }
            _ => true,
        }
    }

    fn check_crimson_halls_like_cpp(&self, boss_id: u32) -> bool {
        match boss_id {
            BOSS_THE_LICH_KING => {
                self.boss_state_like_cpp(BOSS_BLOOD_QUEEN_LANA_THEL) == EncounterState::Done
                    && self.boss_state_like_cpp(BOSS_BLOOD_PRINCE_COUNCIL) == EncounterState::Done
            }
            BOSS_BLOOD_QUEEN_LANA_THEL => {
                self.boss_state_like_cpp(BOSS_BLOOD_PRINCE_COUNCIL) == EncounterState::Done
            }
            _ => true,
        }
    }

    fn check_frostwing_halls_like_cpp(&self, boss_id: u32) -> bool {
        match boss_id {
            BOSS_THE_LICH_KING => {
                self.boss_state_like_cpp(BOSS_SINDRAGOSA) == EncounterState::Done
                    && self.boss_state_like_cpp(BOSS_VALITHRIA_DREAMWALKER) == EncounterState::Done
            }
            BOSS_SINDRAGOSA => {
                self.boss_state_like_cpp(BOSS_VALITHRIA_DREAMWALKER) == EncounterState::Done
            }
            _ => true,
        }
    }

    fn check_lower_spire_like_cpp(&self, boss_id: u32) -> bool {
        if matches!(
            boss_id,
            BOSS_THE_LICH_KING
                | BOSS_SINDRAGOSA
                | BOSS_BLOOD_QUEEN_LANA_THEL
                | BOSS_PROFESSOR_PUTRICIDE
                | BOSS_VALITHRIA_DREAMWALKER
                | BOSS_BLOOD_PRINCE_COUNCIL
                | BOSS_ROTFACE
                | BOSS_FESTERGUT
        ) && self.boss_state_like_cpp(BOSS_DEATHBRINGER_SAURFANG) != EncounterState::Done
        {
            return false;
        }

        if matches!(
            boss_id,
            BOSS_THE_LICH_KING
                | BOSS_SINDRAGOSA
                | BOSS_BLOOD_QUEEN_LANA_THEL
                | BOSS_PROFESSOR_PUTRICIDE
                | BOSS_VALITHRIA_DREAMWALKER
                | BOSS_BLOOD_PRINCE_COUNCIL
                | BOSS_ROTFACE
                | BOSS_FESTERGUT
                | BOSS_DEATHBRINGER_SAURFANG
        ) && self.boss_state_like_cpp(BOSS_ICECROWN_GUNSHIP_BATTLE) != EncounterState::Done
        {
            return false;
        }

        if matches!(
            boss_id,
            BOSS_THE_LICH_KING
                | BOSS_SINDRAGOSA
                | BOSS_BLOOD_QUEEN_LANA_THEL
                | BOSS_PROFESSOR_PUTRICIDE
                | BOSS_VALITHRIA_DREAMWALKER
                | BOSS_BLOOD_PRINCE_COUNCIL
                | BOSS_ROTFACE
                | BOSS_FESTERGUT
                | BOSS_DEATHBRINGER_SAURFANG
                | BOSS_ICECROWN_GUNSHIP_BATTLE
        ) && self.boss_state_like_cpp(BOSS_LADY_DEATHWHISPER) != EncounterState::Done
        {
            return false;
        }

        if matches!(
            boss_id,
            BOSS_THE_LICH_KING
                | BOSS_SINDRAGOSA
                | BOSS_BLOOD_QUEEN_LANA_THEL
                | BOSS_PROFESSOR_PUTRICIDE
                | BOSS_VALITHRIA_DREAMWALKER
                | BOSS_BLOOD_PRINCE_COUNCIL
                | BOSS_ROTFACE
                | BOSS_FESTERGUT
                | BOSS_DEATHBRINGER_SAURFANG
                | BOSS_ICECROWN_GUNSHIP_BATTLE
                | BOSS_LADY_DEATHWHISPER
        ) && self.boss_state_like_cpp(BOSS_LORD_MARROWGAR) != EncounterState::Done
        {
            return false;
        }

        true
    }
}
