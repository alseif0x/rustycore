//! Trait and override authority transitions on the existing Player spell runtime.
use super::*;

impl PlayerSpellRuntimeState {


    /// Drop the loaded trait-config headers and return them to unhydrated.
    pub fn clear_trait_config_rows_like_cpp(&mut self) {
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }

    /// Drop every trait definition, config header and entry flag, as the
    /// trait-config load does before replacing them.
    pub fn begin_trait_authority_load_like_cpp(&mut self) {
        self.trait_definition_ids.clear();
        self.trait_definition_ids_complete = false;
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }

    /// Set the three trait-config authority facts exactly, for a fixture that
    /// must reproduce a recorded owner state including its incoherent
    /// combinations.
    pub fn set_trait_config_authority_for_fixture_like_cpp(
        &mut self,
        rows_complete: bool,
        entries_complete: bool,
        entries_empty: bool,
    ) {
        self.trait_config_rows_complete = rows_complete;
        self.trait_entry_rows_complete = entries_complete;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Mark the loaded trait-config entries authoritative on their own.
    pub fn mark_trait_entry_rows_complete_like_cpp(&mut self) {
        self.trait_entry_rows_complete = true;
    }

    /// Mark the trait-definition snapshot authoritative on its own.
    pub fn mark_trait_definition_ids_complete_like_cpp(&mut self) {
        self.trait_definition_ids_complete = true;
    }

    /// Mark the loaded trait-config headers and their entries authoritative,
    /// as the trait-config load's completion does.
    pub fn mark_trait_authority_complete_like_cpp(&mut self, entries_empty: bool) {
        self.trait_config_rows_complete = true;
        self.trait_entry_rows_complete = true;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Drop every trait definition id and mark the snapshot stale, as a
    /// rejected load does.
    pub fn clear_trait_definition_ids_like_cpp(&mut self) {
        self.trait_definition_ids.clear();
        self.trait_definition_ids_complete = false;
    }

    /// Install the authoritative trait-config headers and their entry flags.
    pub fn complete_trait_authority_load_like_cpp(
        &mut self,
        rows: BTreeMap<i32, PlayerTraitConfigState>,
        entries_empty: bool,
    ) {
        self.trait_config_rows = rows;
        self.trait_config_rows_complete = true;
        self.trait_entry_rows_complete = true;
        self.trait_entry_rows_empty = entries_empty;
    }

    /// Take one spell's trait definition id, as `Player::RemoveSpell` does
    /// before dropping its override.
    pub fn take_trait_definition_id_like_cpp(&mut self, spell_id: i32) -> Option<i32> {
        self.trait_definition_ids.remove(&spell_id)
    }

    pub fn replace_trait_definition_ids_like_cpp(
        &mut self,
        definition_ids: BTreeMap<i32, i32>,
        complete: bool,
    ) {
        self.trait_definition_ids = definition_ids;
        self.trait_definition_ids_complete = complete;
    }

    pub fn replace_trait_config_rows_like_cpp(
        &mut self,
        rows: BTreeMap<i32, PlayerTraitConfigState>,
        complete: bool,
    ) {
        self.trait_config_rows = rows;
        self.trait_config_rows_complete = complete;
    }

    pub fn insert_trait_config_row_like_cpp(
        &mut self,
        config_id: i32,
        row: PlayerTraitConfigState,
    ) {
        self.trait_config_rows.insert(config_id, row);
    }

    /// Install the loaded detail payload of every trait config at once, as the
    /// login path hands the Player the configs it read (C++
    /// `Player::AddTraitConfig`, `Player.h:1836`, with `GetTraitConfig` at
    /// `:1837` reading them back).
    ///
    /// The hydration is refused whole unless it describes exactly the rows this
    /// owner holds: both row sets must be authoritative, the incoming configs
    /// must match the stored rows one for one with unique ids, and each stored
    /// header — config type, specialization and combat flags — must equal the
    /// incoming one. A partial install would leave details describing rows that
    /// were never loaded.
    pub fn install_loaded_trait_config_details_like_cpp(
        &mut self,
        configs: &[(i32, (i32, i32, i32), PlayerTraitConfigDetails)],
    ) -> bool {
        if !self.trait_config_rows_complete || !self.trait_entry_rows_complete {
            return false;
        }
        if self.trait_config_rows.len() != configs.len() {
            return false;
        }
        let unique_ids = configs
            .iter()
            .map(|(config_id, _, _)| *config_id)
            .collect::<BTreeSet<_>>();
        if unique_ids.len() != configs.len() {
            return false;
        }
        if configs.iter().any(|(config_id, header, _)| {
            self.trait_config_rows
                .get(config_id)
                .is_none_or(|state| state.header != *header)
        }) {
            return false;
        }
        for (config_id, _, details) in configs {
            let Some(state) = self.trait_config_rows.get_mut(config_id) else {
                return false;
            };
            state.details = Some(details.clone());
        }
        true
    }

    pub fn set_trait_entry_rows_state_like_cpp(&mut self, complete: bool, empty: bool) {
        self.trait_entry_rows_complete = complete;
        self.trait_entry_rows_empty = empty;
    }

    /// Login rebuilds a fresh C++ Player: drop the trait and override edges so
    /// they cannot contaminate a coincident spell id on the next character.
    pub fn clear_trait_and_override_state_like_cpp(&mut self) {
        self.override_spells.clear();
        self.trait_definition_ids.clear();
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }
}
