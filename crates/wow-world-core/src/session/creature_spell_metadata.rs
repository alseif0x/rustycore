// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared creature spell difficulty fallback selection.

use super::LegacyCreatureAggroConfigLikeCpp;

pub fn creature_ai_spell_difficulty_chain_like_cpp(
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> Vec<u8> {
    let mut chain = Vec::new();
    let mut visited = [false; 256];
    let mut current = difficulty_id;
    loop {
        if visited[usize::from(current)] {
            break;
        }
        visited[usize::from(current)] = true;
        chain.push(current);
        if current == 0 {
            break;
        }
        current = config
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(u32::from(current)))
            .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
    }
    chain
}
