// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the player combat bodies moved to
//! `wow_world_entities::combat_session` and the pet bodies moved to
//! `wow_world_lifecycle::pet_session` (#1363 slice 7a); the pet row-loader
//! entry points came from `unit_tests/session/pet_loading/f3_shims.rs`. Each
//! forwards to the moved body with this session as its host, so the scenarios
//! keep their method syntax and the session and character hotspots' test lines
//! do not grow. The loot-money tracker read is here because its `LootCxRef`
//! facade retired when the battle-pet purchase saga, its last production
//! reader, moved.

#![cfg(test)]

use std::sync::Arc;

use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_world_core::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand;
use wow_world_core::session_policy::PlayerRegenerationRatesLikeCpp;
use wow_world_lifecycle::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetSpellChargeRowLikeCpp, CharacterPetSpellCooldownRowLikeCpp,
    CharacterPetSpellRowLikeCpp,
};

use crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;
use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn tick_player_regeneration_like_cpp(
        &mut self,
        diff_ms: u32,
        power_types: &wow_data::character_progression::PowerTypeStore,
        regen_game_tables: Option<&wow_data::RegenGameTablesLikeCpp>,
        rates: &PlayerRegenerationRatesLikeCpp,
    ) {
        wow_world_entities::tick_player_regeneration_like_cpp(
            self,
            diff_ms,
            power_types,
            regen_game_tables,
            rates,
        )
    }

    pub(crate) fn send_player_power_update_like_cpp(
        &self,
        guid: ObjectGuid,
        power: PowerType,
        value: i32,
    ) {
        wow_world_entities::send_player_power_update_like_cpp(self, guid, power, value)
    }

    pub(crate) fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        wow_world_entities::handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
            self, command,
        )
    }

    pub(crate) fn durable_loot_money_persistence_tracker_like_cpp(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        Arc::clone(
            self.lifecycle
                .durable_loot_money_persistence_tracker_like_cpp(),
        )
    }

    pub(crate) fn set_represented_pet_mode_state_with_spell_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
        react_state: u8,
        command_state: u8,
        created_by_spell: u32,
    ) {
        wow_world_lifecycle::set_represented_pet_mode_state_with_spell_like_cpp(
            self,
            pet_guid,
            react_state,
            command_state,
            created_by_spell,
        )
    }

    pub(crate) fn load_represented_pet_aura_rows_with_timediff_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_aura_rows_with_timediff_like_cpp(
            self,
            pet_number,
            rows,
            timediff_secs,
        )
    }
    pub(crate) fn load_represented_pet_spell_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellRowLikeCpp>,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_spell_rows_like_cpp(self, pet_number, rows)
    }
    pub(crate) fn load_represented_pet_aura_effect_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraEffectRowLikeCpp>,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_aura_effect_rows_like_cpp(self, pet_number, rows)
    }
    pub(crate) fn load_represented_pet_spell_cooldown_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellCooldownRowLikeCpp>,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_spell_cooldown_rows_like_cpp(
            self, pet_number, rows,
        )
    }
    pub(crate) fn load_represented_pet_aura_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_aura_rows_like_cpp(self, pet_number, rows)
    }
    pub(crate) fn load_represented_pet_spell_charge_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellChargeRowLikeCpp>,
    ) -> usize {
        wow_world_lifecycle::load_represented_pet_spell_charge_rows_like_cpp(self, pet_number, rows)
    }
}
