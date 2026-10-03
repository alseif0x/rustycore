//! Linux reference-container replay; all domain rules remain in wow-world.
//! Inputs/outputs are temporary IDs, never a native SpellInfo/raw-record owner.
#[cfg(test)]
mod tests;
use wow_world::forever::spells::{SpellDefinitionSeeds, SpellTraversal};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Key {
    id: u32,
    difficulty: i16,
}
impl From<(u32, i16)> for Key {
    fn from((id, difficulty): (u32, i16)) -> Self {
        Self { id, difficulty }
    }
}
impl From<Key> for (u32, i16) {
    fn from(key: Key) -> Self {
        (key.id, key.difficulty)
    }
}
#[link(name = "rustycore_forever_spell_traversal", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_spell_traversal(
        helpers: *const Key,
        helpers_length: usize,
        clients: *const Key,
        clients_length: usize,
        server: *const Key,
        server_length: usize,
        primary: *mut Key,
        secondary: *mut Key,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
}
struct Orders {
    primary: Vec<Key>,
    secondary: Vec<Key>,
}

pub(super) fn correct(seeds: SpellDefinitionSeeds) -> anyhow::Result<SpellDefinitionSeeds> {
    let inputs = seeds
        .traversal_inputs()
        .ok_or_else(|| anyhow::anyhow!("target spell traversal inputs already retired"))?;
    let orders = replay(
        inputs.helper_insertions(),
        inputs.client_keys(),
        inputs.server_requests(),
        seeds.len(),
    )?;
    // Native containers are gone. Admission and mutation belong to the same
    // canonical Rust owner; it rechecks both exact sets and raw exclusivity.
    seeds
        .with_global_corrections(SpellTraversal::new(
            orders.primary.into_iter().map(Into::into).collect(),
            orders.secondary.into_iter().map(Into::into).collect(),
        ))
        .map_err(|error| anyhow::anyhow!("target spell global corrections rejected: {error:?}"))
}

fn replay(
    helpers: &[(u32, i16)],
    clients: impl IntoIterator<Item = (u32, i16)>,
    server: &[(u32, i16)],
    count: usize,
) -> anyhow::Result<Orders> {
    let helpers: Vec<Key> = helpers.iter().copied().map(Into::into).collect();
    let clients: Vec<Key> = clients.into_iter().map(Into::into).collect();
    let server: Vec<Key> = server.iter().copied().map(Into::into).collect();
    let mut primary = vec![Key::default(); count];
    let mut secondary = vec![Key::default(); count];
    let mut written = 0;
    // Each vector owns a valid array with this exact length; output buffers are
    // initialized and disjoint. C++ catches exceptions and retains no pointers.
    let result = unsafe {
        rustycore_forever_spell_traversal(
            helpers.as_ptr(),
            helpers.len(),
            clients.as_ptr(),
            clients.len(),
            server.as_ptr(),
            server.len(),
            primary.as_mut_ptr(),
            secondary.as_mut_ptr(),
            count,
            &mut written,
        )
    };
    if result != 0 || written != count {
        anyhow::bail!("target spell container replay rejected its inputs");
    }
    Ok(Orders { primary, secondary })
}
