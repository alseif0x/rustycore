// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player health, powers, stats and combat ratings.

use super::*;

impl Player {
    pub const fn gameplay_state(&self) -> &PlayerGameplayState {
        &self.gameplay_state
    }

    pub fn gameplay_state_mut(&mut self) -> &mut PlayerGameplayState {
        &mut self.gameplay_state
    }

    pub fn apply_gameplay_state_from_load(&mut self, record: PlayerGameplayLoadRecord) {
        self.gameplay_state = record.state;
    }

    pub fn set_power_index(&mut self, power: PowerType, index: Option<usize>) {
        self.unit.set_power_index(power, index);
    }

    pub fn get_power_index(&self, power: PowerType) -> Option<usize> {
        self.unit.get_power_index(power)
    }

    pub fn get_power(&self, power: PowerType) -> i32 {
        self.unit.get_power(power)
    }

    pub fn get_max_power(&self, power: PowerType) -> i32 {
        self.unit.get_max_power(power)
    }

    pub fn configure_power_indices_for_class(&mut self, resolver: &impl PlayerPowerIndexResolver) {
        let class_id = self.unit.data().class_id;
        for power in representable_power_types() {
            self.unit.set_power_index(power, None);
        }
        for power in representable_power_types() {
            let index = resolver
                .power_index_by_class(power, class_id)
                .filter(|index| *index < MAX_POWERS_PER_CLASS);
            self.unit.set_power_index(power, index);
        }
    }
}
