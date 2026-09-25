// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hydrate the selected character's represented glyphs during login.

use super::*;

impl WorldSession {
    pub(super) async fn load_character_glyphs_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        guid: ObjectGuid,
    ) {
        // ── Load glyphs from character_glyphs ──
        // C++ `Player::_LoadGlyphs`: skip invalid talent group/slot and glyph ids
        // missing from GlyphProperties.db2.
        self.reset_represented_glyphs_like_cpp();
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
                self.mark_represented_glyphs_loaded_like_cpp();
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
}
