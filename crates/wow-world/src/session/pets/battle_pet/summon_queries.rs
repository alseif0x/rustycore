//! Battle-pet summon state and query snapshots.

use super::super::*;

impl WorldSession {
    /// C++ `WorldSession::HandleBattlePetSummon` represented toggle.
    ///
    /// `BattlePetMgr::SummonPet` silently ignores unknown pets before casting
    /// the summon spell; `DismissPet` clears the active summoned companion.
    pub(crate) fn battle_pet_summon_toggle_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        if self.represented_summoned_battle_pet_guid_like_cpp() == Some(pet_guid) {
            return self.set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        if self.represented_battle_pet_like_cpp(pet_guid).is_none() {
            return false;
        }

        self.set_represented_summoned_battle_pet_guid_like_cpp(Some(pet_guid))
    }
    pub(crate) fn represented_summoned_battle_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) =
            self.with_owned_player_like_cpp(Player::summoned_battle_pet_guid_like_cpp)
        {
            return guid;
        }
        #[cfg(test)]
        {
            self.battle_pet_test_fixture_like_cpp
                .represented_summoned_battle_pet_guid_like_cpp
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    pub(in crate::session) fn set_represented_summoned_battle_pet_guid_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
    ) -> bool {
        if self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_summoned_battle_pet_guid_like_cpp(
                    pet_guid.unwrap_or(wow_core::ObjectGuid::EMPTY),
                );
            })
            .is_some()
        {
            return true;
        }
        #[cfg(test)]
        {
            self.battle_pet_test_fixture_like_cpp
                .represented_summoned_battle_pet_guid_like_cpp = pet_guid;
            true
        }
        #[cfg(not(test))]
        {
            false
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_cage_items_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetCageItemLikeCpp] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_cage_items_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn set_represented_battle_pet_query_companion_like_cpp(
        &mut self,
        unit_guid: ObjectGuid,
        companion: RepresentedBattlePetQueryCompanionLikeCpp,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_query_companions_like_cpp
            .insert(unit_guid, companion);
    }
    pub(crate) fn represented_battle_pet_query_companion_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetQueryCompanionLikeCpp> {
        if let Some(manager) = self.canonical_map_manager.as_ref()
            && let Ok(manager) = manager.lock()
        {
            let mut snapshot = None;
            manager.do_for_all_maps(|managed| {
                if snapshot.is_some() {
                    return;
                }
                let Some(companion) = managed.map().with_creature_like_cpp(unit_guid, |creature| {
                    let unit = creature.unit();
                    RepresentedBattlePetQueryCompanionLikeCpp {
                        creature_id: i32::try_from(creature.entry()).unwrap_or(i32::MAX),
                        name_timestamp: i64::from(
                            unit.battle_pet_companion_name_timestamp_like_cpp(),
                        ),
                        is_summon: creature.is_summon_like_cpp(),
                        owner_is_player: unit
                            .subsystems()
                            .control
                            .owner_guid
                            .is_some_and(|guid| guid.is_player()),
                        battle_pet_companion_guid: unit.battle_pet_companion_guid_like_cpp(),
                    }
                }) else {
                    return;
                };
                snapshot = Some(companion);
            });
            if snapshot.is_some() {
                return snapshot;
            }
        }
        #[cfg(test)]
        {
            self.battle_pet_test_fixture_like_cpp
                .represented_battle_pet_query_companions_like_cpp
                .get(&unit_guid)
                .copied()
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    pub(crate) fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        if let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp {
            return attachment.owner_like_cpp().pet_snapshot_like_cpp(pet_guid);
        }
        #[cfg(test)]
        return self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
            .cloned();
        #[cfg(not(test))]
        None
    }}
