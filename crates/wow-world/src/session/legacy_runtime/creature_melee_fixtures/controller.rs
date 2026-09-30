//! Explicit attachment rail for the original melee fixtures only.
use super::*;
use crate::session::SessionPlayerController;

pub struct CreatureMeleePlayerController(SessionPlayerController);

impl CreatureMeleePlayerController {
    #[allow(clippy::too_many_arguments)]
    pub fn new(guid: ObjectGuid, name: String, position: Position, map_id: u16,
        race: u8, class: u8, level: u8, gender: u8,
    ) -> Self {
        Self(SessionPlayerController::new(guid, name, position, map_id, race, class, level, gender))
    }
}

impl WorldSession {
    pub fn fixture_melee_attach_player_controller(&mut self, controller: CreatureMeleePlayerController) {
        self.attach_player_controller_for_fixture(controller.0);
    }

    pub fn fixture_melee_ensure_world_map(&mut self) -> Option<wow_map::CreateMapDecision> {
        self.ensure_canonical_world_map_for_current_player_like_cpp()
    }

    pub fn fixture_melee_player_snapshot<R>(&self, read: impl FnOnce(&Player) -> R) -> Option<R> {
        self.canonical_player_snapshot_like_cpp(read)
    }

    pub fn fixture_melee_mutate_auras<R>(&mut self,
        mutate: impl FnOnce(&mut wow_entities::AuraSubsystem) -> R,
    ) -> Option<R> {
        // Original callers already ensured their canonical owner. Do not opt
        // into Gossip's detached reconstruction or repair an invalid handle.
        self.mutate_player_aura_subsystem_like_cpp(mutate)
    }

    pub fn fixture_melee_apply_damage_command(&mut self,
        command: crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        self.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
    }
}
