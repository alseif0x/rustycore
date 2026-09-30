//! Weighted random enchantment selection; ItemEnchantmentMgr::GenerateRandomProperties.
use rand::{Rng, distributions::{Distribution, WeightedIndex}};

pub fn select_random_enchantment<R: Rng + ?Sized>(
    group: impl IntoIterator<Item = (u32, f32)>,
    rng: &mut R,
) -> Option<u32> {
    let valid_rows = group
        .into_iter()
        .filter(|row| (0.000001..=100.0).contains(&row.1))
        .collect::<Vec<_>>();
    let weights = valid_rows.iter().map(|row| row.1).collect::<Vec<_>>();
    let distribution = WeightedIndex::new(weights).ok()?;
    Some(valid_rows[distribution.sample(rng)].0)
}

#[cfg(test)]
mod tests;
