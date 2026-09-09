//! Represented spell cooldown and recovery state.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Bounded C++ `SpellHistory::HasCooldown`.
    ///
    /// C++ primarily keys `_spellCooldowns` by spell id; item id influences the
    /// duration/category calculation. Category sharing is intentionally outside
    /// this represented slice.
    pub(crate) fn represented_spell_cooldown_remaining_ms_like_cpp(
        &self,
        spell_id: i32,
        cooldown_ms: u32,
    ) -> Option<u32> {
        let last_cast = self.spell_last_cast_time_like_cpp(spell_id)?;
        let Some(last_cast) = last_cast else {
            return Some(0);
        };
        let elapsed_ms = last_cast.elapsed().as_millis() as u32;
        Some(cooldown_ms.saturating_sub(elapsed_ms))
    }
    pub(in crate::session) fn player_spell_history_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::SpellHistory> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.unit().subsystems().spells.history.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let mut history = wow_entities::SpellHistory {
                cooldowns_loaded: self.represented_character_spell_cooldowns_loaded_like_cpp,
                charges_loaded: self.represented_character_spell_charges_loaded_like_cpp,
                ..Default::default()
            };
            history.cooldowns = self
                .represented_character_spell_cooldowns_like_cpp
                .values()
                .map(|row| {
                    (
                        row.spell_id,
                        wow_entities::SpellCooldown {
                            spell_id: row.spell_id,
                            item_id: row.item_id,
                            cooldown_end_ms: u64::try_from(row.cooldown_end_unix_secs)
                                .unwrap_or(0)
                                .saturating_mul(1_000),
                            category_id: row.category_id,
                            category_end_ms: u64::try_from(row.category_end_unix_secs)
                                .unwrap_or(0)
                                .saturating_mul(1_000),
                            on_hold: false,
                        },
                    )
                })
                .collect();
            history.charges = self
                .represented_character_spell_charges_like_cpp
                .iter()
                .map(|(&category_id, rows)| {
                    (
                        category_id,
                        rows.iter()
                            .map(|row| wow_entities::SpellChargeState {
                                recharge_start_ms: u64::try_from(row.recharge_start_unix_secs)
                                    .unwrap_or(0)
                                    .saturating_mul(1_000),
                                recharge_end_ms: u64::try_from(row.recharge_end_unix_secs)
                                    .unwrap_or(0)
                                    .saturating_mul(1_000),
                            })
                            .collect(),
                    )
                })
                .collect();
            return Some(history);
        }
        canonical
    }
    #[cfg(test)]
    fn store_spell_history_fixture_like_cpp(
        &mut self,
        history: wow_entities::SpellHistory,
    ) -> bool {
        if self.player_handle_like_cpp.is_none() {
            self.represented_character_spell_cooldowns_loaded_like_cpp = history.cooldowns_loaded;
            self.represented_character_spell_charges_loaded_like_cpp = history.charges_loaded;
            self.represented_character_spell_cooldowns_like_cpp = history
                .cooldowns
                .values()
                .map(|row| {
                    (
                        row.spell_id,
                        RepresentedCharacterSpellCooldownLikeCpp {
                            spell_id: row.spell_id,
                            item_id: row.item_id,
                            cooldown_end_unix_secs: (row.cooldown_end_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                            category_id: row.category_id,
                            category_end_unix_secs: (row.category_end_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                        },
                    )
                })
                .collect();
            self.represented_character_spell_charges_like_cpp = history
                .charges
                .iter()
                .map(|(&category_id, rows)| {
                    (
                        category_id,
                        rows.iter()
                            .map(|row| RepresentedCharacterSpellChargeLikeCpp {
                                category_id,
                                recharge_start_unix_secs: (row.recharge_start_ms / 1_000)
                                    .min(i64::MAX as u64)
                                    as i64,
                                recharge_end_unix_secs: (row.recharge_end_ms / 1_000)
                                    .min(i64::MAX as u64)
                                    as i64,
                            })
                            .collect(),
                    )
                })
                .collect();
            return true;
        }
        false
    }
    pub(in crate::session) fn mutate_player_spell_history_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::SpellHistory) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut history = self.player_spell_history_snapshot_like_cpp()?;
            let result = f(&mut history);
            return self
                .store_spell_history_fixture_like_cpp(history)
                .then_some(result);
        }
        // C++ Unit owns one SpellHistory (Unit.h:1417-1418,1945).
        // Mutate that value under one generation-checked owner access; a
        // snapshot followed by replacement could overwrite another transition.
        self.with_owned_player_mut_like_cpp(|player| {
            f(&mut player.unit_mut().subsystems_mut().spells.history)
        })
    }
    pub(crate) fn reset_represented_character_spell_cooldowns_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.cooldowns.clear();
            history.cooldowns_loaded = false;
        });
    }
    pub(crate) fn mark_represented_character_spell_cooldowns_loaded_like_cpp(&mut self) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.cooldowns_loaded = true;
        });
    }
    pub(crate) fn record_loaded_character_spell_cooldown_like_cpp(
        &mut self,
        spell_id: u32,
        item_id: u32,
        cooldown_end_unix_secs: i64,
        category_id: u32,
        category_end_unix_secs: i64,
    ) {
        let _ = self.mutate_player_spell_history_like_cpp(|history| {
            history.cooldowns.insert(
                spell_id,
                wow_entities::SpellCooldown {
                    spell_id,
                    item_id,
                    cooldown_end_ms: u64::try_from(cooldown_end_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                    category_id,
                    category_end_ms: u64::try_from(category_end_unix_secs)
                        .unwrap_or(0)
                        .saturating_mul(1_000),
                    on_hold: false,
                },
            );
        });
    }
    /// Check if a spell is on cooldown (global or per-spell).
    ///
    /// Returns true if either the global cooldown (1500ms) or the spell-specific
    /// cooldown from SpellStore is still active.
    pub fn is_spell_on_cooldown(&self, spell_id: i32) -> bool {
        let Some(last_cast) = self.last_spell_cast_time_like_cpp() else {
            return true;
        };
        let Some(last_cast) = last_cast else {
            return false; // Never casted
        };

        let elapsed_ms = last_cast.elapsed().as_millis() as u32;

        // Global cooldown: 1500ms
        if elapsed_ms < 1500 {
            return true;
        }

        // Per-spell cooldown (if exists in SpellStore)
        if let Some(store) = &self.spell_store {
            if let Some(spell_info) = store.get(spell_id) {
                if elapsed_ms < spell_info.cooldown_ms {
                    return true;
                }
            }
        }

        false
    }
    /// Persist the login snapshot of the player's spell history + charge packets so the
    /// before-add init helper can re-send them (e.g. on far teleport). Mirrors C++
    /// `Player::SendInitialPacketsBeforeAddToMap` reading `GetSpellHistory()`.
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_login_spell_history_packets_like_cpp(
        &mut self,
        history: Vec<SpellHistoryEntry>,
        charges: Vec<SpellChargeEntry>,
    ) {
        #[cfg(test)]
        {
            self.represented_spell_history_packets_like_cpp = (history, charges);
        }
    }
    /// Login snapshot of spell-history + charge packet entries
    /// (see `record_login_spell_history_packets_like_cpp`).
    #[cfg(test)]
    pub(crate) fn spell_history_packets_like_cpp(
        &self,
    ) -> (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>) {
        self.represented_spell_history_packets_like_cpp.clone()
    }
}
