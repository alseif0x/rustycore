// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation consumer tests.

use super::*;

#[test]
fn tilebuilder_coordinates_are_projected_with_producer_float_order() {
    let coordinates = [
        Coordinate {
            row: 31,
            column: 31,
        },
        Coordinate {
            row: 32,
            column: 31,
        },
        Coordinate {
            row: 32,
            column: 32,
        },
    ];
    let source = tilebuilder::global_params(&coordinates).unwrap();
    let params = DetourNavMeshParams {
        origin: source.origin,
        tile_width: source.tile_width,
        tile_height: source.tile_height,
        max_tiles: source.max_tiles,
        max_polys: source.max_polys,
    };

    assert_eq!(native_tile_coordinates(coordinates[0], params), (1, 1));
    assert_eq!(native_tile_coordinates(coordinates[2], params), (0, 0));

    let asymmetric = [
        Coordinate {
            row: 31,
            column: 31,
        },
        Coordinate {
            row: 32,
            column: 33,
        },
    ];
    let source = tilebuilder::global_params(&asymmetric).unwrap();
    let params = DetourNavMeshParams {
        origin: source.origin,
        tile_width: source.tile_width,
        tile_height: source.tile_height,
        max_tiles: source.max_tiles,
        max_polys: source.max_polys,
    };
    assert_eq!(native_tile_coordinates(asymmetric[0], params), (2, 1));
}

#[test]
fn global_params_require_the_selected_profile_and_grid_origin() {
    let coordinates = [Coordinate {
        row: 31,
        column: 31,
    }];
    let mut params = tilebuilder::global_params(&coordinates).unwrap();
    assert!(validate_global_params(params).is_ok());

    // This one-ULP offset used to disappear when dividing by GRID_SIZE and
    // passing the broad quotient-based alignment tolerance.
    params.origin[0] = f32::from_bits(1);
    assert!(matches!(
        validate_global_params(params),
        Err(ForeverNavigationError::NonCanonicalGlobalParams)
    ));
}
