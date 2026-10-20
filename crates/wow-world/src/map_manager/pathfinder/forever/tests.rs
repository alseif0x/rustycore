// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation consumer tests.

use super::*;

#[test]
fn public_limits_only_allow_downward_reductions() {
    let defaults = ForeverNavigationLimits::default();
    let reduced = ForeverNavigationLimits::new(
        defaults.max_loaded_tiles() - 1,
        defaults.max_loaded_bytes() - 1,
        defaults.max_tiles_per_batch() - 1,
        defaults.max_batch_bytes() - 1,
        defaults.max_file_bytes() - 1,
        defaults.max_instance_queries() - 1,
    );
    assert!(reduced.is_ok());
    assert!(matches!(
        ForeverNavigationLimits::new(
            defaults.max_loaded_tiles() + 1,
            defaults.max_loaded_bytes(),
            defaults.max_tiles_per_batch(),
            defaults.max_batch_bytes(),
            defaults.max_file_bytes(),
            defaults.max_instance_queries(),
        ),
        Err(ForeverNavigationError::InvalidLimit {
            field: "max_loaded_tiles"
        })
    ));
}
