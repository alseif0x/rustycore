// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[allow(dead_code)]
    pub(crate) fn tick_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> bool {
        self.world_entities
            .tick_represented_gameobject_door_or_button_like_cpp(gameobject_guid)
    }
    pub(crate) fn use_represented_gameobject_spell_focus_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        linked_trap_entry: u32,
    ) -> bool {
        self.world_entities
            .use_represented_gameobject_spell_focus_like_cpp(
                gameobject_guid,
                player_guid,
                linked_trap_entry,
            )
    }
    pub(crate) fn use_represented_gameobject_trap_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        source: wow_entities::TrapUseSource,
    ) -> bool {
        self.world_entities
            .use_represented_gameobject_trap_like_cpp(gameobject_guid, user_guid, source)
    }
    pub(crate) fn use_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        user_guid: ObjectGuid,
        restore_time_ms: u32,
    ) -> bool {
        self.world_entities
            .use_represented_gameobject_door_or_button_like_cpp(
                gameobject_guid,
                user_guid,
                restore_time_ms,
            )
    }
}
