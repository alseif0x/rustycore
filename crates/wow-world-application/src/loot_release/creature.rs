// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use tracing::info;
use wow_packet::packets::loot::LOOT_TYPE_SKINNING_LIKE_CPP;

/// The DynamicFlags VALUES a fully-looted creature release publishes.
///
/// C++ `LootHandler::DoLootRelease` forces the DynamicFlags field before it
/// runs the corpse lifecycle, so the field reaches every viewer whatever that
/// transition does. The lifecycle's VALUES (lootable flag removed) win; a
/// refused or absent transition still publishes the forced VALUES, whose
/// map-publication mark the force already handed to the fanout (#1263 F6-8E,
/// review D-1).
pub(super) fn fully_looted_release_values_to_publish_like_cpp<'a, M>(
    lifecycle_update: Option<&'a (M, wow_entities::UnitValuesUpdate)>,
    forced_values_update: Option<&'a wow_entities::UnitValuesUpdate>,
) -> Option<&'a wow_entities::UnitValuesUpdate> {
    lifecycle_update
        .map(|(_, values_update)| values_update)
        .or(forced_values_update)
}

impl LootReleaseCxLikeCpp<'_> {
    pub(super) fn release_looted_creature_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        whole_object_fully_looted: bool,
        represented_loot_type: u8,
        authoritative_release: Option<&AuthoritativeLootReleaseLikeCpp>,
    ) -> bool {
        // C++ forces the viewer-dependent DynamicFlags field after every
        // creature release, including a selected personal pool that completed
        // while another pool remains.
        let forced_values_update = self
            .owner
            .force_creature_loot_release_dynamic_flags_like_cpp(
                owner_guid,
                authoritative_release.is_some(),
            );

        if !whole_object_fully_looted {
            if let Some(values_update) = forced_values_update.as_ref() {
                self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                    owner_guid,
                    values_update,
                    authoritative_release
                        .as_ref()
                        .map(|release| &release.authority),
                );
            }
            if authoritative_release.is_some() {
                self.loot
                    .discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
            }
            return true;
        }

        let corpse_decay_looted_rate = self.stats_inputs.corpse_decay_looted_rate_like_cpp();

        // Start corpse despawn timer if fully looted.
        let whole_object_fully_skinned = authoritative_release.map_or(
            represented_loot_type == LOOT_TYPE_SKINNING_LIKE_CPP,
            |release| release.whole_object_fully_skinned,
        );
        let lifecycle_update = self.owner.finish_looted_creature_like_cpp(
            owner_guid,
            whole_object_fully_skinned,
            corpse_decay_looted_rate,
            authoritative_release.map(|release| {
                (
                    &release.authority,
                    release.object_generation,
                    release.lifecycle_revision,
                )
            }),
        );

        // #1263 F6-8E: the forced field is published either way. A refused
        // or absent lifecycle transition publishes the forced VALUES, whose
        // map-publication mark was already handed to this fanout.
        if let Some(values_update) = fully_looted_release_values_to_publish_like_cpp(
            lifecycle_update.as_ref(),
            forced_values_update.as_ref(),
        ) {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                owner_guid,
                values_update,
                authoritative_release
                    .as_ref()
                    .map(|release| &release.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);

        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }

        if authoritative_release.is_some() {
            self.loot
                .discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
        }

        true
    }
}
