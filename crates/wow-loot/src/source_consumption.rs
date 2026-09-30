//! Source-stack consumption on loot release (Player::DestroyItemCount, LootHandler.cpp).

pub fn remaining_source_item_count(current_count: u32, maximum_destroy_count: Option<u32>) -> u32 {
    let destroy_count = maximum_destroy_count
        .unwrap_or(current_count)
        .min(current_count);
    current_count.saturating_sub(destroy_count)
}

#[cfg(test)]
mod tests;
