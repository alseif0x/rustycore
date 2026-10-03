// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bounded GroupRegistry difficulty operations for the application layer.

use crate::session::{
    SessionCore,
    mailbox::{ApplyGroupDifficultyLikeCppCommand, SessionCommand},
};
use wow_core::ObjectGuid;
use wow_social::group::{
    GroupAuthorityErrorLikeCpp, GroupDifficultyKindLikeCpp, GroupInfo,
    GroupTransitionOutcomeLikeCpp,
};

pub struct GroupDifficultyAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn group_difficulty_access_like_cpp(&self) -> GroupDifficultyAccessLikeCpp<'_> {
        GroupDifficultyAccessLikeCpp { core: self }
    }
}

impl GroupDifficultyAccessLikeCpp<'_> {
    pub fn group_snapshot_like_cpp(&self, group_guid: u64) -> Option<GroupInfo> {
        self.core.directory.group_registry.as_ref()?.get(&group_guid)
    }

    pub fn set_difficulty_transition_like_cpp(
        &self,
        group_guid: u64,
        actor_guid: ObjectGuid,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    ) -> Option<Result<GroupTransitionOutcomeLikeCpp<u32>, GroupAuthorityErrorLikeCpp>> {
        Some(
            self.core
                .directory
                .group_registry
                .as_ref()?
                .set_difficulty_transition_like_cpp(group_guid, actor_guid, difficulty_id, kind),
        )
    }

    pub fn apply_member_transition_like_cpp(
        &self,
        member_guid: ObjectGuid,
        group_guid: u64,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    ) {
        let Some(player_registry) = self.core.player_registry.as_ref() else {
            return;
        };
        if let Some(member) = player_registry.group_presence(member_guid) {
            let _ = player_registry.deliver_group_state_command_like_cpp(
                member.registration,
                SessionCommand::ApplyGroupDifficultyLikeCpp(
                    ApplyGroupDifficultyLikeCppCommand {
                        group_guid,
                        difficulty_id,
                        kind,
                    },
                ),
            );
        } else {
            player_registry.mark_group_state_reconciliation_like_cpp(member_guid);
        }
    }
}
