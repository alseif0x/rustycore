// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player equipment sets and transmog outfits.
//!
//! C++ `Player` owns `EquipmentSetContainer _equipmentSets` (`Player.h:3050`)
//! and performs every transition on it itself: `_LoadEquipmentSets`
//! (`Player.cpp:16907`), `SetEquipmentSet` (`:26376`), `_SaveEquipmentSets`
//! (`:26409`) and `DeleteEquipmentSet` (`:26524`).
//!
//! Separated from the two loose `player_gameplay_state.rs` members under #767,
//! which also closed the storage to this crate's Player module. The update
//! state travels with the set it describes, so a caller can no longer write a
//! row and forget the state C++ assigns in the same statement.
//!
//! The catalog validation, the equipment-set guid generation and the packet
//! construction stay in `wow-world`, where the dependency policy allows them:
//! C++ reaches `sObjectMgr` and the client packet from inside `SetEquipmentSet`,
//! and this owner cannot.

use std::collections::BTreeMap;

use crate::player::{PlayerEquipmentSetLikeCpp, PlayerEquipmentSetUpdateStateLikeCpp};

/// Canonical owner of C++ `Player::_equipmentSets` (`Player.h:3050`) and the
/// load-authority flag that says whether the stored rows were read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerEquipmentSetsLikeCpp {
    sets: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    loaded: bool,
}

impl PlayerEquipmentSetsLikeCpp {
    // ---- reads -------------------------------------------------------------

    /// The stored sets, keyed by equipment-set guid as C++ keys
    /// `_equipmentSets`.
    #[must_use]
    pub fn sets_like_cpp(&self) -> &BTreeMap<u64, PlayerEquipmentSetLikeCpp> {
        &self.sets
    }

    /// One stored set, as C++ `_equipmentSets.find` locates it.
    #[must_use]
    pub fn set_like_cpp(&self, guid: u64) -> Option<&PlayerEquipmentSetLikeCpp> {
        self.sets.get(&guid)
    }

    /// The update state of one stored set, or `None` when it is not stored.
    #[must_use]
    pub fn set_state_like_cpp(&self, guid: u64) -> Option<PlayerEquipmentSetUpdateStateLikeCpp> {
        self.sets
            .get(&guid)
            .map(|equipment_set| equipment_set.state)
    }

    /// Whether the stored rows were read (`_LoadEquipmentSets` ran). An empty
    /// loaded owner is authoritative; an empty unloaded one is unhydrated.
    #[must_use]
    pub fn is_loaded_like_cpp(&self) -> bool {
        self.loaded
    }

    /// Copy the stored sets, for a caller that must own them — the save
    /// projection captures them under the same admitted owner read.
    #[must_use]
    pub fn snapshot_like_cpp(&self) -> BTreeMap<u64, PlayerEquipmentSetLikeCpp> {
        self.sets.clone()
    }

    // ---- transitions -------------------------------------------------------

    /// C++ `Player::_LoadEquipmentSets` (`Player.cpp:16907`) storing one row it
    /// read, keyed by the row's own guid.
    pub fn install_loaded_set_like_cpp(&mut self, equipment_set: PlayerEquipmentSetLikeCpp) {
        self.sets.insert(equipment_set.guid, equipment_set);
    }

    /// Mark the stored rows as read, as the login load completes.
    pub fn mark_loaded_like_cpp(&mut self) {
        self.loaded = true;
    }

    /// Return to the unhydrated state: no rows and no load authority, as the
    /// owner is rebuilt before the character's rows are read again.
    pub fn clear_like_cpp(&mut self) {
        self.sets.clear();
        self.loaded = false;
    }

    /// C++ `Player::SetEquipmentSet` (`Player.cpp:26376`) for a set the client
    /// is creating: C++ reaches `GenerateEquipmentSetGuid` for the guid, so the
    /// caller supplies it here, and the stored row starts as `New`.
    pub fn create_set_like_cpp(&mut self, mut equipment_set: PlayerEquipmentSetLikeCpp) {
        equipment_set.state = PlayerEquipmentSetUpdateStateLikeCpp::New;
        self.sets.insert(equipment_set.guid, equipment_set);
    }

    /// C++ `Player::SetEquipmentSet` (`Player.cpp:26376`) for a set the client
    /// is editing.
    ///
    /// A guid that is not stored is refused, exactly as C++ logs and returns
    /// without writing. The stored state then follows the C++ rule at `:26406`:
    /// a set that is still `New` stays `New`, anything else becomes `Changed`,
    /// so a set created and edited before its first save is still inserted once.
    pub fn update_set_like_cpp(&mut self, mut equipment_set: PlayerEquipmentSetLikeCpp) -> bool {
        let Some(existing_state) = self.set_state_like_cpp(equipment_set.guid) else {
            return false;
        };
        equipment_set.state = if existing_state == PlayerEquipmentSetUpdateStateLikeCpp::New {
            PlayerEquipmentSetUpdateStateLikeCpp::New
        } else {
            PlayerEquipmentSetUpdateStateLikeCpp::Changed
        };
        self.sets.insert(equipment_set.guid, equipment_set);
        true
    }

    /// Assign one equipment set to a specialization, as the client's assign
    /// request reaches `SetEquipmentSet`'s state rule for an existing row.
    /// A transmog outfit is not an equipment set and is not matched.
    pub fn assign_set_to_spec_like_cpp(&mut self, set_id: u32, assigned_spec_index: i32) -> bool {
        let Some(equipment_set) = self.sets.values_mut().find(|equipment_set| {
            equipment_set.set_id == set_id
                && equipment_set.set_type == crate::player::PlayerEquipmentSetTypeLikeCpp::Equipment
        }) else {
            return false;
        };
        equipment_set.assigned_spec_index = assigned_spec_index;
        if equipment_set.state != PlayerEquipmentSetUpdateStateLikeCpp::New {
            equipment_set.state = PlayerEquipmentSetUpdateStateLikeCpp::Changed;
        }
        true
    }

    /// C++ `Player::DeleteEquipmentSet` (`Player.cpp:26524`): a set that was
    /// never saved is erased, and a stored one is tombstoned so the save can
    /// delete its row.
    pub fn delete_set_like_cpp(&mut self, guid: u64) -> bool {
        let Some(equipment_set) = self.sets.get_mut(&guid) else {
            return false;
        };
        if equipment_set.state == PlayerEquipmentSetUpdateStateLikeCpp::New {
            self.sets.remove(&guid);
        } else {
            equipment_set.state = PlayerEquipmentSetUpdateStateLikeCpp::Deleted;
        }
        true
    }

    /// C++ `Player::_SaveEquipmentSets` (`Player.cpp:26409`) as it leaves the
    /// container: deleted rows are erased once their statement is queued, and
    /// every surviving row returns to `Unchanged`.
    pub fn mark_sets_saved_like_cpp(&mut self) {
        self.sets.retain(|_, equipment_set| {
            if equipment_set.state == PlayerEquipmentSetUpdateStateLikeCpp::Deleted {
                return false;
            }
            equipment_set.state = PlayerEquipmentSetUpdateStateLikeCpp::Unchanged;
            true
        });
    }

    /// Settle one completed save projection against the rows that exist now.
    ///
    /// This is RustyCore's deferred-save acknowledgement, not a C++ function:
    /// C++ writes its statements inside `_SaveEquipmentSets` while holding the
    /// Player, so it has nothing to reconcile afterwards. A row that is still
    /// byte-for-byte what was saved settles; a row that changed since keeps a
    /// state that describes the difference; and a row that was saved and has
    /// since disappeared is tombstoned so its stored row is deleted.
    pub fn acknowledge_saved_sets_like_cpp(
        &mut self,
        saved: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    ) {
        use PlayerEquipmentSetUpdateStateLikeCpp::{Changed, Deleted, New, Unchanged};
        for (guid, row) in saved {
            match self.sets.get_mut(&guid) {
                Some(current) if *current == row => {
                    if row.state == Deleted {
                        self.sets.remove(&guid);
                    } else {
                        current.state = Unchanged;
                    }
                }
                Some(current) => {
                    current.state = match (row.state, current.state) {
                        (Deleted, Deleted) => Deleted,
                        (Deleted, _) => New,
                        (_, Deleted) => Deleted,
                        (New | Changed | Unchanged, _) => Changed,
                    };
                }
                None if row.state != Deleted => {
                    self.sets.insert(
                        guid,
                        PlayerEquipmentSetLikeCpp {
                            state: Deleted,
                            ..row
                        },
                    );
                }
                None => {}
            }
        }
    }
}
