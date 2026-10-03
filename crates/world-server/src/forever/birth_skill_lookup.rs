//! Source GNU unordered_multimap equal-key replay; domain owns matching.
//! Records and temporary native containers never become a second data owner.
#[cfg(test)]
mod tests;
use wow_data::forever_birth::BirthCatalog;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Key {
    skill: u32,
    record: u32,
}
#[link(name = "rustycore_forever_spell_traversal", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_birth_skill_lookup(
        records: *const Key,
        length: usize,
        order: *mut u32,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
}

pub(super) fn order(birth: &BirthCatalog) -> anyhow::Result<Vec<u32>> {
    let inputs = birth
        .race_class_records()
        .filter(|row| birth.skill_line(u32::from(row.skill)).is_some())
        .map(|row| Key {
            skill: u32::from(row.skill),
            record: row.id,
        })
        .collect::<Vec<_>>();
    replay(&inputs)
}
fn replay(inputs: &[Key]) -> anyhow::Result<Vec<u32>> {
    let mut output = vec![0; inputs.len()];
    let mut written = 0;
    // Exact initialized arrays, disjoint output, no retained pointer. Native
    // catches allocation exceptions; domain rechecks the exact admitted set.
    let code = unsafe {
        rustycore_forever_birth_skill_lookup(
            inputs.as_ptr(),
            inputs.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    if code != 0 || written != output.len() {
        anyhow::bail!("target birth skill lookup replay rejected its inputs");
    }
    Ok(output)
}
