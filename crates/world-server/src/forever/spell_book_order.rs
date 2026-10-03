//! Transient numeric-only target container replay; Player owns all book state.
#[cfg(test)]
mod override_tests;
#[cfg(test)]
mod tests;
use wow_world::forever::player::SpellBookMutation;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mutation {
    spell: u32,
    action: u32,
}
#[link(name = "rustycore_forever_spell_traversal", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_spell_book_order(
        mutations: *const Mutation,
        length: usize,
        order: *mut u32,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
    fn rustycore_forever_spell_override_order(
        mutations: *const Mutation,
        length: usize,
        order: *mut u32,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
}

/// The inner override SET has a different source container from the book MAP.
pub(super) fn override_order(
    history: &[SpellBookMutation],
    count: usize,
) -> anyhow::Result<Vec<u32>> {
    let mutations = history
        .iter()
        .map(|event| match *event {
            SpellBookMutation::Insert(spell) => Mutation { spell, action: 1 },
            SpellBookMutation::Erase(spell) => Mutation { spell, action: 2 },
        })
        .collect::<Vec<_>>();
    let mut output = vec![0; count];
    let mut written = 0;
    // Same initialized/disjoint arrays and no-retained-pointer contract.
    let code = unsafe {
        rustycore_forever_spell_override_order(
            mutations.as_ptr(),
            mutations.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    if code != 0 || written != count {
        anyhow::bail!("target spell override membership replay rejected its inputs");
    }
    Ok(output)
}

pub(super) fn order(history: &[SpellBookMutation], count: usize) -> anyhow::Result<Vec<u32>> {
    let mutations = history
        .iter()
        .map(|event| match *event {
            SpellBookMutation::Insert(spell) => Mutation { spell, action: 1 },
            SpellBookMutation::Erase(spell) => Mutation { spell, action: 2 },
        })
        .collect::<Vec<_>>();
    replay(&mutations, count)
}

fn replay(history: &[Mutation], count: usize) -> anyhow::Result<Vec<u32>> {
    let mut output = vec![0; count];
    let mut written = 0;
    // Exact initialized arrays and disjoint output, no retained pointer. Native
    // catches exceptions. Domain separately admits the exact canonical key set.
    let code = unsafe {
        rustycore_forever_spell_book_order(
            history.as_ptr(),
            history.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    if code != 0 || written != count {
        anyhow::bail!("target spell book membership replay rejected its inputs");
    }
    Ok(output)
}
