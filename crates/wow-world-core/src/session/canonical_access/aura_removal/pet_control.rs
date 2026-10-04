// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::SessionCore;
use wow_core::ObjectGuid;
use wow_entities::PlayerPetLifecycleStateLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct AuraDismountPetFixtureRefsLikeCpp<'a> {
    guid: &'a Option<ObjectGuid>,
    react: &'a mut u8,
    command: &'a mut u8,
    stable: &'a mut wow_entities::PetStable,
    empty_authority: &'a mut bool,
    temporary_number: &'a mut u32,
    old_spell: &'a mut u32,
    temporary_react: &'a mut Option<u8>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> AuraDismountPetFixtureRefsLikeCpp<'a> {
    pub fn new(
        guid: &'a Option<ObjectGuid>, react: &'a mut u8, command: &'a mut u8,
        stable: &'a mut wow_entities::PetStable, empty_authority: &'a mut bool,
        temporary_number: &'a mut u32, old_spell: &'a mut u32,
        temporary_react: &'a mut Option<u8>,
    ) -> Self {
        Self { guid, react, command, stable, empty_authority, temporary_number, old_spell, temporary_react }
    }
}

struct DismountPetAccessLikeCpp<'a, 'b> {
    core: &'a SessionCore,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: &'a mut AuraDismountPetFixtureRefsLikeCpp<'b>,
    #[cfg(not(any(test, feature = "test-fixtures")))]
    lifetime: std::marker::PhantomData<&'b ()>,
}

impl SessionCore {
    pub fn enable_pet_controls_on_dismount_with_fixture_refs_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut AuraDismountPetFixtureRefsLikeCpp<'_>,
    ) {
        let mut access = DismountPetAccessLikeCpp {
            core: self,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
            #[cfg(not(any(test, feature = "test-fixtures")))]
            lifetime: std::marker::PhantomData,
        };
        if let Some(pet_guid) = access.player_pet_guid_state_like_cpp().flatten() {
            if let Some((current_react_state, command_state)) = access.canonical_pet_mode_state_like_cpp() {
                let react_state = access.player_pet_lifecycle_state_snapshot_like_cpp()
                    .and_then(|state| state.temporary_mount_react_state)
                    .unwrap_or(current_react_state);
                if access.update_canonical_pet_mode_state_like_cpp(react_state, command_state) {
                    self.send_packet(&wow_packet::packets::pet::PetMode {
                        pet_guid, react_state, command_state, flag: 0,
                    });
                }
            }
        }
        let _ = access.clear_temporary_mount_react_state_like_cpp();
    }
}

impl DismountPetAccessLikeCpp<'_, '_> {
    fn player_pet_guid_state_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.gameplay_state().pet_guid);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.fixtures.guid);
        }
        canonical
    }

    fn canonical_pet_mode_state_like_cpp(&self) -> Option<(u8, u8)> {
        let pet_guid = self.player_pet_guid_state_like_cpp().flatten()?;
        let canonical = (|| {
            let manager = self.core.canonical_map_manager.as_ref()?.lock().ok()?;
            let mut inspect = Some(|pet: &wow_entities::Pet| {
                let react_state = pet.creature().react_state() as u8;
                let command_state = pet.creature().unit().subsystems().control.charm_info.as_ref()
                    .map_or(wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP, |info| info.command_state);
                (react_state, command_state)
            });
            let mut result = None;
            manager.do_for_all_maps(|managed| {
                if result.is_some() { return; }
                if let Some(pet) = managed.map().get_typed_pet(pet_guid) {
                    result = Some(inspect.take().expect("pet inspector consumed once")(pet));
                }
            });
            result
        })();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((*self.fixtures.react, *self.fixtures.command));
        }
        canonical
    }

    fn player_pet_lifecycle_state_snapshot_like_cpp(&self) -> Option<PlayerPetLifecycleStateLikeCpp> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.pet_lifecycle_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(PlayerPetLifecycleStateLikeCpp {
                stable: self.fixtures.stable.clone(),
                character_rows_empty_authority_complete: *self.fixtures.empty_authority,
                temporary_unsummoned_pet_number: *self.fixtures.temporary_number,
                old_pet_spell: *self.fixtures.old_spell,
                temporary_mount_react_state: *self.fixtures.temporary_react,
            });
        }
        canonical
    }

    fn update_canonical_pet_mode_state_like_cpp(&mut self, react_state: u8, command_state: u8) -> bool {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else { return false; };
        let canonical = self.core.with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
            pet.creature_mut().set_react_state(crate::session::react_state_from_db_like_cpp(react_state));
            pet.creature_mut().unit_mut().subsystems_mut().control.init_charm_info().command_state = command_state;
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            *self.fixtures.react = react_state;
            *self.fixtures.command = command_state;
            return true;
        }
        canonical
    }

    fn clear_temporary_mount_react_state_like_cpp(&mut self) -> bool {
        if self.core.player_handle_like_cpp.is_some() {
            return self.core.with_owned_player_mut_like_cpp(|player| {
                player.pet_lifecycle_state_mut_like_cpp().temporary_mount_react_state = None;
            }).is_some();
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let mut state = self.player_pet_lifecycle_state_snapshot_like_cpp().unwrap_or_default();
            state.temporary_mount_react_state = None;
            *self.fixtures.stable = state.stable;
            *self.fixtures.empty_authority = state.character_rows_empty_authority_complete;
            *self.fixtures.temporary_number = state.temporary_unsummoned_pet_number;
            *self.fixtures.old_spell = state.old_pet_spell;
            *self.fixtures.temporary_react = state.temporary_mount_react_state;
            true
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        { false }
    }
}
