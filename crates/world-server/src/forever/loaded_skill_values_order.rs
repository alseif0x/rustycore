//! Temporary numeric replay for Player::_LoadSkills' local loaded-value map.
#[cfg(test)]
#[path = "../../unit_tests/forever/loaded_skill_values_order.rs"]
mod tests;

#[link(name = "rustycore_forever_spell_traversal", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_loaded_skill_values_order(
        first_assignments: *const u32,
        length: usize,
        output: *mut u32,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
}

pub(super) fn order(first_assignments: &[u32], count: usize) -> anyhow::Result<Vec<u32>> {
    let mut output = vec![0; count];
    let mut written = 0;
    // Initialized, disjoint arrays; native retains no pointer or map owner.
    let code = unsafe {
        rustycore_forever_loaded_skill_values_order(
            first_assignments.as_ptr(),
            first_assignments.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    if code != 0 || written != count {
        anyhow::bail!("target loaded-skill-values first-assignment replay rejected its inputs");
    }
    Ok(output)
}
