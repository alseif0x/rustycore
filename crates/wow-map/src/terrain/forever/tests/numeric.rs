// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: boundary checks and explicit pinned-native selector replay.
use super::super::{Error, NO_HEIGHT, query::select_height};

#[test]
fn static_height_precedence_and_native_fuzzy_tolerance() {
    for (grid, vmap, z, expected) in [
        (7., 0., 8., 7.),
        (7., 9., 10., 9.),
        (7., 0., 6., 0.),
        (7., NO_HEIGHT, 6., NO_HEIGHT),
        (7., NO_HEIGHT, 7. - 0.05, 7.),
        (NO_HEIGHT, 0., 5., 0.),
        (NO_HEIGHT, NO_HEIGHT, 5., NO_HEIGHT),
        (0., -1., -0.02, 0.),
        (7., NO_HEIGHT, 6.94, NO_HEIGHT),
    ] {
        assert_eq!(select_height(grid, vmap, z), Ok(expected));
    }
    assert_eq!(select_height(f32::NAN, 0., 0.), Err(Error::NonFinite));
    assert_eq!(select_height(0., f32::INFINITY, 0.), Err(Error::NonFinite));
}

/// Opt-in native acceptance: missing oracle is an ignored test, never a pass.
/// Build it with tools/qa/forever_height_oracle.py, then supply its explicit path.
#[test]
#[ignore = "requires explicitly built pinned native oracle"]
fn pinned_native_height_selector_1210_cases() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let executable =
        std::env::var_os("RCE_FOREVER_HEIGHT_ORACLE").expect("explicit native oracle path");
    let values: [f32; 11] = [
        NO_HEIGHT, -100001., -100000., -99999., -1., -0., 0., 1., 7., 10000., 100000.,
    ];
    let mut expected = Vec::new();
    let mut input = Vec::new();
    for grid in values {
        for vmap in values {
            let threshold = grid - 0.05_f32;
            for z in [
                grid - 10.,
                grid,
                grid + 10.,
                threshold.next_down(),
                threshold,
                threshold.next_up(),
                -0.,
                0.,
                NO_HEIGHT,
                200000.,
            ] {
                for v in [grid, vmap, z] {
                    input.extend(v.to_bits().to_le_bytes());
                }
                expected.extend(
                    select_height(grid, vmap, z)
                        .unwrap()
                        .to_bits()
                        .to_le_bytes(),
                );
            }
        }
    }
    assert_eq!(expected.len(), 1210 * 4);
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Drain child stdout concurrently via wait_with_output; even an expanded
    // corpus cannot deadlock the two finite pipe buffers.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let result = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert_eq!(result.stdout, expected);
}
