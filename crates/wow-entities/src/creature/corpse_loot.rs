//! Loot-driven corpse decay duration (Creature::AllLootRemovedFromCorpse).

pub fn looted_corpse_decay_seconds(
    is_fully_skinned: bool,
    corpse_delay_secs: u32,
    ignore_decay_ratio: bool,
    corpse_decay_looted_rate: f32,
) -> u32 {
    if is_fully_skinned {
        return 0;
    }

    let rate = if ignore_decay_ratio {
        1.0
    } else {
        corpse_decay_looted_rate.max(0.0)
    };
    ((corpse_delay_secs as f32) * rate) as u32
}

#[cfg(test)]
mod tests;
