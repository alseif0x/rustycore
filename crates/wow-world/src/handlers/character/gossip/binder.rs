// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl WorldSession {
    /// CMSG_BINDER_ACTIVATE — player sets hearthstone at innkeeper.
    /// C++ refs: `WorldSession::HandleBinderActivateOpcode` /
    /// `WorldSession::SendBindPoint` (`Handlers/NPCHandler.cpp:373-402`).
    pub async fn handle_binder_activate_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        hello: Hello,
    ) {
        info!(
            "BinderActivate {:?} account {}",
            hello.unit, self.account_id
        );
        if !self.player_is_strictly_in_world_like_cpp()
            || self.resolved_player_is_alive_like_cpp() != Some(true)
        {
            return;
        }
        let Some(_innkeeper) = self.represented_npc_can_interact_with_like_cpp(
            hello.unit,
            NPCFlags1::INNKEEPER.bits(),
            0,
        ) else {
            debug!(
                innkeeper_guid = ?hello.unit,
                account = self.account_id,
                "BinderActivate rejected: NPC missing, out of range, dead, or lacks INNKEEPER flag"
            );
            return;
        };
        // C++ HandleBinderActivateOpcode removes feign death before
        // SendBindPoint performs its instanceable-map rejection.
        self.remove_represented_feign_death_if_needed_like_cpp();
        if self.player_current_map_instanceable_like_cpp() {
            debug!(
                innkeeper_guid = ?hello.unit,
                map_id = self.player_map_id_like_cpp(),
                "BinderActivate rejected: current map is instanceable like C++ SendBindPoint"
            );
            return;
        }

        // C++ SendBindPoint calls innkeeper->CastSpell(player, 3286, true).
        // Route the triggered creature cast through the represented spell
        // pipeline so SpellGo, EffectBind, persistence, and bind packets keep
        // their C++ ordering and caster identity.
        const BIND_SPELL_ID_LIKE_CPP: i32 = 3286;
        const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;
        const CAST_FLAG_UNKNOWN_9_LIKE_CPP: u32 = 0x0000_0100;
        const CAST_FLAG_NO_GCD_LIKE_CPP: u32 = 0x0004_0000;
        const BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP: u32 =
            CAST_FLAG_UNKNOWN_9_LIKE_CPP | CAST_FLAG_PENDING_LIKE_CPP | CAST_FLAG_NO_GCD_LIKE_CPP;
        if let Some(player_guid) = self.player_guid() {
            let Some(cast_id) =
                self.next_represented_spell_cast_guid_like_cpp(BIND_SPELL_ID_LIKE_CPP)
            else {
                return;
            };
            if let Err(error) = self
                .execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    BIND_SPELL_ID_LIKE_CPP,
                    player_guid,
                    cast_id,
                    SpellCastVisual::default(),
                    SpellTargetData {
                        flags: 0x2,
                        unit: player_guid,
                        item: ObjectGuid::EMPTY,
                        ..SpellTargetData::default()
                    },
                    SpellCastMetadata {
                        caster_guid_override: Some(hello.unit),
                        // C++ Spell::SendSpellGo starts with UNKNOWN_9, adds
                        // PENDING for this non-client triggered cast, and
                        // adds NO_GCD because spell 3286 has no
                        // StartRecoveryTime row.
                        cast_flags: BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP,
                        ..SpellCastMetadata::default()
                    },
                )
                .await
            {
                warn!(
                    innkeeper_guid = ?hello.unit,
                    account = self.account_id,
                    error,
                    "BinderActivate bind spell failed"
                );
            }
        }
        // C++ closes gossip after attempting the triggered cast, even if the
        // spell execution itself cannot complete.
        self.send_close_gossip_like_cpp();
    }

    #[cfg(test)]
    pub async fn handle_binder_activate(&mut self, hello: Hello) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_binder_activate_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            hello,
        )
        .await;
    }
}
