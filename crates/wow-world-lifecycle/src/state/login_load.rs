use std::sync::Arc;

use tracing::{info, warn};
use wow_world_core::session::{HubMut, PlayerBootstrapCatalogsLikeCpp};
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp,
};

use super::SessionLifecycleState;

impl SessionLifecycleState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn load_completed_achievement_rows_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        rows: impl IntoIterator<Item = u32>,
    ) {
        let _ = hub.replace_completed_achievement_ids_like_cpp(rows);
    }

    pub async fn load_completed_achievements_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let _ = hub.replace_completed_achievement_ids_like_cpp([]);

        let Some(player_guid) = hub.core.player_guid() else {
            warn!(
                account = hub.core.account_id,
                "LoadCompletedAchievements skipped: player guid unavailable"
            );
            return;
        };
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            warn!(
                account = hub.core.account_id,
                guid = player_guid.counter(),
                "LoadCompletedAchievements skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements {
                    player_guid: player_guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
                    guid = player_guid.counter(),
                    "LoadCompletedAchievements query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = hub.core.account_id,
                    guid = player_guid.counter(),
                    "Player lifecycle port returned the wrong auxiliary login data for completed achievements"
                );
                return;
            }
        };

        let _ = hub.replace_completed_achievement_ids_like_cpp(rows);
    }

    pub async fn load_account_toys_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_toys_like_cpp(hub, []);
            return;
        };

        let bnet_account_id = hub.core.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
                rows,
            )) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.is_favorite, row.has_fanfare))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account toys: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = hub.core.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for toys"
                );
                Vec::new()
            }
        };

        self.load_represented_account_toys_like_cpp(hub, rows);
    }

    pub async fn load_account_heirlooms_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_heirlooms_like_cpp(hub, []);
            return;
        };

        let bnet_account_id = hub.core.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
            ) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.flags))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account heirlooms: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = hub.core.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for heirlooms"
                );
                Vec::new()
            }
        };

        self.load_represented_account_heirlooms_like_cpp(hub, rows);
    }

    pub async fn load_character_glyphs_for_login_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        guid: wow_core::ObjectGuid,
    ) {
        // ── Load glyphs from character_glyphs ──
        // C++ `Player::_LoadGlyphs`: skip invalid talent group/slot and glyph ids
        // missing from GlyphProperties.db2.
        hub.reset_represented_glyphs_like_cpp();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Glyphs(rows),
            ) => {
                let mut loaded = 0usize;
                let mut skipped = 0usize;
                for row in rows {
                    if self.load_represented_glyph_row_like_cpp(
                        hub,
                        player_bootstrap.glyph_properties.as_ref(),
                        row.talent_group,
                        row.glyph_slot,
                        row.glyph_id,
                    ) {
                        loaded += 1;
                    } else {
                        skipped += 1;
                    }
                }
                self.mark_represented_glyphs_loaded_like_cpp(hub);
                info!(
                    loaded,
                    skipped,
                    player_guid = guid.counter(),
                    "Loaded represented character glyphs like C++ Player::_LoadGlyphs"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character glyphs for {:?}: {}", guid, reason);
            }
            _ => unreachable!("glyph request returned a different row family"),
        }
    }

    pub async fn load_action_buttons_for_login_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: wow_core::ObjectGuid,
    ) -> Option<[i64; 180]> {
        // Column types: button=tinyint unsigned, action=int unsigned, type=tinyint unsigned
        let mut action_buttons = [0i64; 180];
        let mut action_count = 0u32;
        hub.reset_represented_action_buttons_like_cpp();
        // C++ loads the action-button map for GetActiveTalentGroup(), not always spec 0.
        let Some((active_spec, trait_config_id)) =
            hub.shared().represented_action_button_db_context_like_cpp()
        else {
            hub.core.kick(
                "canonical Player specialization owner unavailable while loading action buttons",
            );
            return None;
        };
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
                    player_guid: guid.counter() as u64,
                    active_spec,
                    trait_config_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::ActionButtons(rows),
            ) => {
                for row in rows {
                    if (row.button as usize) < 180 && row.action > 0 {
                        self.record_loaded_action_button_like_cpp(
                            hub,
                            row.button,
                            row.action,
                            row.button_type,
                        );
                        action_buttons[row.button as usize] =
                            wow_packet::packets::misc::UpdateActionButtons::pack_button(
                                row.action as i32,
                                row.button_type,
                            );
                        action_count += 1;
                    }
                }
                self.mark_represented_action_buttons_loaded_like_cpp(hub);
                info!("Loaded {} action buttons for {:?}", action_count, guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load action buttons for {:?}: {}", guid, reason);
            }
            _ => unreachable!("action-button request returned a different row family"),
        }
        Some(action_buttons)
    }
}
