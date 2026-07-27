#[cfg(feature = "test-fixtures")]
fn main() {
    use std::path::PathBuf;
    use wow_recastdetour::mmap_tile_coords_for_wow_position_like_cpp;
    use wow_recastdetour::test_fixtures::write_obstacle_ring_mmaps_at_height_like_cpp;

    const MAP_ID: u32 = 1;
    const CENTRE_X: f32 = -10_118.333;
    const CENTRE_Y: f32 = 2_681.667;
    const CENTRE_Z: f32 = 218.49;
    const EXPECTED_GRID: (i32, i32) = (50, 26);

    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(output) = args.next() else {
        eprintln!(
            "usage: {} <fixture-directory>",
            PathBuf::from(program).display()
        );
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly one fixture-directory argument");
        std::process::exit(2);
    }

    let output = PathBuf::from(output);
    let grid = mmap_tile_coords_for_wow_position_like_cpp(CENTRE_X, CENTRE_Y);
    assert_eq!(
        grid, EXPECTED_GRID,
        "fixture coordinates must keep the pinned C++ grid identity"
    );
    write_obstacle_ring_mmaps_at_height_like_cpp(
        &output,
        MAP_ID,
        &[(CENTRE_X, CENTRE_Y, CENTRE_Z)],
    );

    println!(
        "generated map {MAP_ID}, grid {},{} at {}",
        grid.0,
        grid.1,
        output.display()
    );
}

#[cfg(not(feature = "test-fixtures"))]
fn main() {
    eprintln!(
        "error: rerun with `--features test-fixtures`; fixture generation is never enabled in production"
    );
    std::process::exit(2);
}
