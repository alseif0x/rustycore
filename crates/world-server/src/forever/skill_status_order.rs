//! Transient insertion-only numeric replay; Entities owns every status payload.
#[cfg(test)]
#[path = "../../unit_tests/forever/skill_status_order.rs"]
mod tests;

#[link(name = "rustycore_forever_spell_traversal", kind = "static")]
unsafe extern "C" {
    fn rustycore_forever_skill_status_order(
        insertions: *const u32,
        length: usize,
        output: *mut u32,
        capacity: usize,
        written: *mut usize,
    ) -> i32;
}

pub(super) fn order(history: &[u32], count: usize) -> anyhow::Result<Vec<u32>> {
    let mut output = vec![0; count];
    let mut written = 0;
    // Initialized, disjoint arrays with exact lengths; native retains no pointer
    // and catches exceptions. Entities separately admits the canonical key set.
    let code = unsafe {
        rustycore_forever_skill_status_order(
            history.as_ptr(),
            history.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written,
        )
    };
    if code != 0 || written != count {
        anyhow::bail!("target skill status insertion replay rejected its inputs");
    }
    Ok(output)
}
