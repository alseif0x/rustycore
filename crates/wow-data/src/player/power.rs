// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! DB2/record-backed bridge for TrinityCore `Player::GetPowerIndexByClass`.
//!
//! `wow-entities` owns only the `PlayerPowerIndexResolver` trait so entities stay independent
//! from DB2 stores. This module provides a small injectable row store that can be populated from
//! future `ChrClasses`/`ChrPowerTypes` DB2 readers without hardcoding class power layouts in
//! `Player`.

use std::collections::HashMap;

use wow_constants::PowerType;
use wow_entities::PlayerPowerIndexResolver;

/// Minimal row shape needed to answer `GetPowerIndexByClass(power, class)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassPowerIndexRecord {
    pub class_id: u8,
    pub power: PowerType,
    pub power_index: usize,
}

impl ClassPowerIndexRecord {
    pub const fn new(class_id: u8, power: PowerType, power_index: usize) -> Self {
        Self {
            class_id,
            power,
            power_index,
        }
    }
}

/// In-memory class/power index store populated from DB2-equivalent records.
#[derive(Debug, Clone, Default)]
pub struct PlayerClassPowerIndexStore {
    indices: HashMap<(u8, PowerType), usize>,
}

impl PlayerClassPowerIndexStore {
    pub fn from_records(records: impl IntoIterator<Item = ClassPowerIndexRecord>) -> Self {
        let mut store = Self::default();
        for record in records {
            store.insert(record);
        }
        store
    }

    pub fn insert(&mut self, record: ClassPowerIndexRecord) {
        self.indices
            .insert((record.class_id, record.power), record.power_index);
    }

    pub fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize> {
        self.indices.get(&(class_id, power)).copied()
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Resolver adapter passed to `wow_entities::Player` lifecycle/configuration code.
#[derive(Debug, Clone, Default)]
pub struct Db2PlayerPowerIndexResolver {
    store: PlayerClassPowerIndexStore,
}

impl Db2PlayerPowerIndexResolver {
    pub fn new(store: PlayerClassPowerIndexStore) -> Self {
        Self { store }
    }

    pub fn from_records(records: impl IntoIterator<Item = ClassPowerIndexRecord>) -> Self {
        Self::new(PlayerClassPowerIndexStore::from_records(records))
    }

    pub const fn store(&self) -> &PlayerClassPowerIndexStore {
        &self.store
    }
}

impl PlayerPowerIndexResolver for Db2PlayerPowerIndexResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize> {
        self.store.power_index_by_class(power, class_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::Gender;
    use wow_entities::{CLASS_PALADIN, MAX_POWERS_PER_CLASS, Player};

    #[test]
    fn db2_player_power_index_resolver_returns_record_backed_indices() {
        let resolver = Db2PlayerPowerIndexResolver::from_records([
            ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::Mana, 0),
            ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::Energy, 3),
        ]);

        assert_eq!(
            resolver.power_index_by_class(PowerType::Mana, CLASS_PALADIN),
            Some(0)
        );
        assert_eq!(
            resolver.power_index_by_class(PowerType::Energy, CLASS_PALADIN),
            Some(3)
        );
        assert_eq!(
            resolver.power_index_by_class(PowerType::Focus, CLASS_PALADIN),
            None
        );
        assert_eq!(resolver.store().len(), 2);
    }

    #[test]
    fn db2_player_power_index_resolver_feeds_player_and_entity_ignores_out_of_range_indices() {
        let resolver = Db2PlayerPowerIndexResolver::from_records([
            ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::Mana, 0),
            ClassPowerIndexRecord::new(CLASS_PALADIN, PowerType::ComboPoints, 9),
            ClassPowerIndexRecord::new(
                CLASS_PALADIN,
                PowerType::AlternateMount,
                MAX_POWERS_PER_CLASS,
            ),
        ]);
        let mut player = Player::new(None, false);
        player.set_race_class_gender(1, CLASS_PALADIN, Gender::Male);
        player.clear_data_changes();

        player.configure_power_indices_for_class(&resolver);

        assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
        assert_eq!(player.get_power_index(PowerType::ComboPoints), Some(9));
        assert_eq!(player.get_power_index(PowerType::AlternateMount), None);
        assert!(!player.unit().unit_data_changes_mask().is_any_set());
    }
}
