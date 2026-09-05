// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spells, auras, cooldowns and talents.

use super::*;

impl PlayerSpellRuntimeState {
    /// Replace the represented known-ID projection and prune its dependent
    /// metadata together. Caller performs load-proof invalidation beforehand.
    pub fn replace_known_spell_ids_like_cpp(&mut self, spells: Vec<i32>) {
        self.known_spells = spells;
        self.removed_known_spells.clear();
        self.dependent_known_spells
            .retain(|id| self.known_spells.contains(id));
        self.favorite_known_spells
            .retain(|id| self.known_spells.contains(id));
        self.trait_definition_ids
            .retain(|id, _| self.known_spells.contains(id));
    }

    /// Low-level represented grant, not the full Player::AddSpell closure
    /// (Player.cpp:2741). Keep signed IDs and insertion order unchanged.
    pub fn learn_known_spell_id_like_cpp(&mut self, spell_id: i32) {
        if !self.known_spells.contains(&spell_id) {
            self.known_spells.push(spell_id);
        }
        self.removed_known_spells.remove(&spell_id);
    }

    /// Represented dependent-spell metadata owned by C++ PlayerSpellMap;
    /// Player::AddSpell (Player.cpp:2812-2819) promotes dependent state.
    pub fn mark_known_spell_dependent_like_cpp(&mut self, spell_id: i32) {
        self.dependent_known_spells.insert(spell_id);
        self.favorite_known_spells.remove(&spell_id);
        if self.rows_complete
            && let Some(row) = self.rows.get_mut(&spell_id)
        {
            row.dependent = true;
            row.favorite = false;
        }
    }

    /// Player::_SaveSpells (Player.cpp:20399) retires removed rows and resets
    /// durable rows, but leaves temporary spells untouched. The caller invokes
    /// this only after confirmed commit, preserving the port's durability gate.
    pub fn mark_spell_rows_saved_like_cpp(&mut self) {
        self.rows.retain(|_, spell| {
            if spell.state == PlayerSpellLoadState::Removed {
                return false;
            }
            if spell.state != PlayerSpellLoadState::Temporary {
                spell.state = PlayerSpellLoadState::Unchanged;
            }
            true
        });
        self.removed_known_spells.clear();
        self.trait_definition_ids
            .retain(|spell_id, _| self.rows.contains_key(spell_id));
        self.dependent_known_spells = self
            .rows
            .values()
            .filter(|spell| spell.dependent)
            .map(|spell| spell.spell_id)
            .collect();
        self.favorite_known_spells = self
            .rows
            .values()
            .filter(|spell| spell.favorite)
            .map(|spell| spell.spell_id)
            .collect();
        self.known_spells = self
            .rows
            .values()
            .filter(|spell| !spell.disabled)
            .map(|spell| spell.spell_id)
            .collect();
    }

    /// Reconcile represented base grants with loaded PlayerSpellMap rows.
    /// Player::LearnSpell (Player.cpp:3192-3200) preserves disabled active state
    /// and loaded favorites; the port's fallback rows retain pending grants.
    pub fn replace_loaded_spell_rows_like_cpp(
        &mut self,
        mut rows: BTreeMap<i32, PlayerKnownSpellRecord>,
        complete: bool,
    ) {
        for (&spell_id, fallback) in &self.fallback_rows {
            let reconciled = if let Some(loaded) = rows.get(&spell_id).cloned() {
                let active = if loaded.disabled { loaded.active } else { true };
                let dependent_promoted = fallback.dependent && !loaded.dependent;
                PlayerKnownSpellRecord {
                    active,
                    disabled: false,
                    state: match loaded.state {
                        PlayerSpellLoadState::New => PlayerSpellLoadState::New,
                        PlayerSpellLoadState::Removed => PlayerSpellLoadState::Changed,
                        PlayerSpellLoadState::Temporary => PlayerSpellLoadState::New,
                        _ if loaded.disabled || loaded.active != active || dependent_promoted => {
                            PlayerSpellLoadState::Changed
                        }
                        _ => loaded.state,
                    },
                    dependent: loaded.dependent || fallback.dependent,
                    ..loaded
                }
            } else {
                fallback.clone()
            };
            rows.insert(spell_id, reconciled);
        }
        self.rows = rows;
        self.rows_loaded = true;
        self.rows_complete = complete;
    }

    /// Begin the represented Player::_LoadTraits authority lifecycle
    /// (Player.cpp:26635-26698), without erasing unrelated spell/override state.
    pub fn begin_trait_config_load_like_cpp(&mut self) {
        self.trait_definition_ids.clear();
        self.trait_definition_ids_complete = false;
        self.trait_config_rows.clear();
        self.trait_config_rows_complete = false;
        self.trait_entry_rows_complete = false;
        self.trait_entry_rows_empty = false;
    }

    /// Preserve the port's complete-header proof, not full TraitMgr validation.
    /// Duplicate/nonpositive IDs reset both header and trait-spell authority.
    pub fn complete_trait_config_load_like_cpp(
        &mut self,
        configs: Vec<(i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        let mut exact = BTreeMap::new();
        for (id, kind, specialization, flags) in configs {
            if id <= 0 || exact.insert(id, (kind, specialization, flags)).is_some() {
                self.begin_trait_config_load_like_cpp();
                return false;
            }
        }
        self.trait_config_rows = exact;
        self.trait_config_rows_complete = true;
        self.trait_entry_rows_complete = true;
        self.trait_entry_rows_empty = entries_empty;
        true
    }

    /// Represented PlayerSpell::TraitDefinitionId authority (Player.h:191).
    /// Invalid or duplicate input clears the previous proof, preserving the
    /// port's fail-closed load contract independently of packet/catalog code.
    pub fn replace_complete_trait_definition_ids_like_cpp(
        &mut self,
        traits: Vec<(i32, i32)>,
    ) -> bool {
        let mut exact = BTreeMap::new();
        for (spell, definition) in traits {
            if spell <= 0 || definition <= 0 || exact.insert(spell, definition).is_some() {
                self.trait_definition_ids.clear();
                self.trait_definition_ids_complete = false;
                return false;
            }
        }
        self.trait_definition_ids = exact;
        self.trait_definition_ids_complete = true;
        true
    }

    /// C++ Player::AddOverrideSpell, Player.cpp:28581-28584, retaining the
    /// represented signed-ID admission gate before accessing the native map.
    pub fn add_override_spell_like_cpp(&mut self, overridden: i32, replacement: i32) {
        if overridden > 0 && replacement > 0 {
            self.override_spells
                .entry(overridden)
                .or_default()
                .insert(replacement);
        }
    }

    /// C++ Player::RemoveOverrideSpell, Player.cpp:28586-28596.
    pub fn remove_override_spell_like_cpp(&mut self, overridden: i32, replacement: i32) {
        if let Some(overrides) = self.override_spells.get_mut(&overridden) {
            overrides.remove(&replacement);
            if overrides.is_empty() {
                self.override_spells.remove(&overridden);
            }
        }
    }
}

impl Player {
    pub const fn titan_grip_penalty_spell_id(&self) -> u32 {
        self.titan_grip_penalty_spell_id
    }

    /// C++ `MAX_ACTION_BUTTONS` for the 3.4.3 client.
    pub const ACTION_BUTTON_COUNT_LIKE_CPP: usize = 180;

    /// C++ `Player::AddActionButton` storage mutation after the caller has
    /// performed the store-backed validation that still lives above the
    /// entity boundary.
    pub fn set_action_button_like_cpp(
        &mut self,
        button: u8,
        action_id: u32,
        action_type: u8,
    ) -> bool {
        if usize::from(button) >= Self::ACTION_BUTTON_COUNT_LIKE_CPP {
            return false;
        }

        self.gameplay_state
            .action_buttons
            .retain(|record| record.button != button);
        if action_id != 0 {
            self.gameplay_state
                .action_buttons
                .push(PlayerActionButtonRecord {
                    button,
                    action_id,
                    action_type,
                });
            self.gameplay_state
                .action_buttons
                .sort_unstable_by_key(|record| record.button);
        }
        true
    }

    /// C++ `_LoadActionButtons` starts from an empty `m_actionButtons` map.
    pub fn reset_action_buttons_for_load_like_cpp(&mut self) {
        self.gameplay_state.action_buttons.clear();
        self.gameplay_state.action_buttons_loaded = false;
    }

    pub fn mark_action_buttons_loaded_like_cpp(&mut self) {
        self.gameplay_state.action_buttons_loaded = true;
    }

    pub fn action_buttons_loaded_like_cpp(&self) -> bool {
        self.gameplay_state.action_buttons_loaded
    }

    pub fn action_buttons_snapshot_like_cpp(&self) -> [u32; Self::ACTION_BUTTON_COUNT_LIKE_CPP] {
        let mut buttons = [0; Self::ACTION_BUTTON_COUNT_LIKE_CPP];
        for record in &self.gameplay_state.action_buttons {
            let Some(slot) = buttons.get_mut(usize::from(record.button)) else {
                continue;
            };
            *slot = record.action_id | (u32::from(record.action_type) << 24);
        }
        buttons
    }

    pub fn action_button_like_cpp(&self, button: u8) -> Option<u32> {
        (usize::from(button) < Self::ACTION_BUTTON_COUNT_LIKE_CPP).then(|| {
            self.gameplay_state
                .action_buttons
                .iter()
                .find(|record| record.button == button)
                .map(|record| record.action_id | (u32::from(record.action_type) << 24))
                .unwrap_or(0)
        })
    }
}
