// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The acquisition snapshot completeness marks, moved from the World
//! `session/spell_state/acquisition.rs` (#1363 slice 9a).

use super::*;

pub fn invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp<
    H: SpellbookHostLikeCpp,
>(
    host: &mut H,
) {
    host.core()
        .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);
    });
}
/// Authorizes the current post-login spell, trait and override mirrors as one
/// coherent acquisition snapshot. Call only after all represented `AddSpell`
/// work for character login has completed.
pub fn mark_represented_spell_acquisition_snapshot_complete_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
) {
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.mark_override_spells_complete_like_cpp();
    });
}
