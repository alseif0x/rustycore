// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: synthetic selected Forever navigation consumer tests.
//! Synthetic package and valid Detour fixtures for the public consumer path.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use forever_casc::{
    Locale, integrity,
    package::{self, Component, Context, Run, Summary},
};
use forever_navigation::{
    mmap::{self, Params},
    package_v23::{
        self, ClosureMap, Coverage, Expected, LiquidSnapshot, MapSnapshot, PackageIdentity,
        SourceIdentity, TerrainMap, TileOutput,
    },
    tilebuilder::{self, Coordinate},
};
use wow_recastdetour::test_fixtures::obstacle_ring_tile_blob_at;

pub const SELECTED_MAP: u32 = 48;
const PLAN_SHA256: [u8; 32] = [0x34; 32];
const PRODUCER_SHA256: [u8; 32] = [0x35; 32];
const SOURCE_SHA256: [u8; 32] = [0x46; 32];
pub const WRITTEN_A: Coordinate = Coordinate {
    row: 31,
    column: 31,
};
pub const WRITTEN_C: Coordinate = Coordinate {
    row: 31,
    column: 32,
};
pub const EMPTY: Coordinate = Coordinate {
    row: 32,
    column: 31,
};
pub const WRITTEN_B: Coordinate = Coordinate {
    row: 32,
    column: 32,
};

static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

pub struct SyntheticNavigationPackage {
    root: PathBuf,
    pub output: PathBuf,
    pub expected: Expected,
}

impl SyntheticNavigationPackage {
    pub fn create() -> Self {
        Self::create_with_options(false, false, false, 9)
    }

    pub fn create_with_bad_placement(bad_placement: bool) -> Self {
        Self::create_with_options(bad_placement, false, false, 9)
    }

    pub fn create_with_bad_bounds(bad_bounds: bool) -> Self {
        Self::create_with_options(false, bad_bounds, false, 9)
    }

    pub fn create_with_bad_global_origin(bad_global_origin: bool) -> Self {
        Self::create_with_options(false, false, bad_global_origin, 9)
    }

    pub fn create_with_metadata_calls(metadata_calls: u64) -> Self {
        Self::create_with_options(false, false, false, metadata_calls)
    }

    fn create_with_options(
        bad_placement: bool,
        bad_bounds: bool,
        bad_global_origin: bool,
        metadata_calls: u64,
    ) -> Self {
        let mut params = tilebuilder::global_params(&[WRITTEN_A, WRITTEN_B])
            .expect("synthetic ancestor inventory");
        if bad_global_origin {
            params.origin[0] += 0.125;
        }
        let global = mmap::encode_global(SELECTED_MAP, params, &[], mmap::Limits::default())
            .expect("synthetic global params");
        let mut written_tiles = Vec::new();
        for coordinate in [WRITTEN_A, WRITTEN_C, WRITTEN_B] {
            let bytes = tile_container(
                coordinate,
                params,
                bad_placement && coordinate == WRITTEN_A,
                bad_bounds && coordinate == WRITTEN_A,
            );
            written_tiles.push((coordinate, bytes));
        }
        let mut coverage = synthetic_coverage(&global, &written_tiles, metadata_calls);
        let coverage_bytes = coverage.render().expect("valid synthetic coverage");

        let root = std::env::temp_dir().join(format!(
            "rustycore-selected-navigation-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create synthetic package parent");
        let output = root.join("package");
        let mut run = Run::create(&output, coverage.context, &[], package::Limits::default())
            .expect("create synthetic package run");
        run.put(
            Component::Navigation,
            package_v23::COVERAGE_PATH,
            &coverage_bytes,
        )
        .expect("write synthetic coverage");
        run.put(
            Component::Navigation,
            &mmap::global_path(SELECTED_MAP),
            &global,
        )
        .expect("write synthetic global");
        for (coordinate, bytes) in written_tiles {
            let path = mmap::tile_path(SELECTED_MAP, coordinate.row, coordinate.column)
                .expect("synthetic tile path");
            run.put(Component::Navigation, &path, &bytes)
                .expect("write synthetic tile");
        }
        run.finish_component(Component::Navigation)
            .expect("seal synthetic navigation component");
        run.finish().expect("finish synthetic package");

        let expected = Expected {
            selected_map: SELECTED_MAP,
            locale: Locale::EnUs,
            plan_sha256: PLAN_SHA256,
            context: Some(coverage.context),
        };
        Self {
            root,
            output,
            expected,
        }
    }

    pub fn tile_path(&self, coordinate: Coordinate) -> PathBuf {
        self.output.join(
            mmap::tile_path(SELECTED_MAP, coordinate.row, coordinate.column)
                .expect("synthetic tile path"),
        )
    }

    pub fn replace_output_with(&self, replacement: &Self) -> std::io::Result<()> {
        let displaced = self.root.join("displaced-package");
        fs::rename(&self.output, displaced)?;
        fs::rename(&replacement.output, &self.output)
    }
}

impl Drop for SyntheticNavigationPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn tile_container(
    coordinate: Coordinate,
    params: Params,
    bad_placement: bool,
    bad_bounds: bool,
) -> Vec<u8> {
    let max_x = (32 - i32::from(coordinate.column)) as f32 * tilebuilder::GRID_SIZE;
    let min_x = max_x - tilebuilder::GRID_SIZE;
    let max_z = (32 - i32::from(coordinate.row)) as f32 * tilebuilder::GRID_SIZE;
    let min_z = max_z - tilebuilder::GRID_SIZE;
    let tile_x = (((min_x + max_x) / 2.0 - params.origin[0]) / params.tile_width) as i32;
    let tile_y = (((min_z + max_z) / 2.0 - params.origin[2]) / params.tile_height) as i32;
    let mut blob = obstacle_ring_tile_blob_at(tile_x, tile_y, min_x, min_z);
    // Keep the generated ring geometry while giving the native header the
    // producer's nominal horizontal bounds for the whole WoW tile.
    put_f32(&mut blob.data, 72, min_x);
    put_f32(&mut blob.data, 84, max_x);
    put_f32(&mut blob.data, 80, min_z);
    put_f32(&mut blob.data, 92, max_z);
    if bad_placement {
        let wrong_x = tile_x + 1;
        blob.data[8..12].copy_from_slice(&wrong_x.to_le_bytes());
    }
    if bad_bounds {
        let min_x = f32::from_le_bytes(blob.data[72..76].try_into().unwrap());
        blob.data[72..76].copy_from_slice(&(min_x + 0.25).to_le_bytes());
    }

    let mut bytes = vec![0; mmap::TILE_HEADER_BYTES + blob.data.len()];
    put_u32(&mut bytes, 0, blob.header.mmap_magic);
    put_u32(&mut bytes, 4, blob.header.dt_version);
    put_u32(&mut bytes, 8, mmap::VERSION);
    put_u32(&mut bytes, 12, blob.data.len() as u32);
    bytes[16] = u8::from(blob.header.uses_liquids);
    bytes[17..20].copy_from_slice(&blob.header.padding);
    bytes[mmap::TILE_HEADER_BYTES..].copy_from_slice(&blob.data);
    bytes
}

fn synthetic_coverage(
    global: &[u8],
    tiles: &[(Coordinate, Vec<u8>)],
    metadata_calls: u64,
) -> Coverage {
    let locale = Locale::EnUs;
    let identity = SourceIdentity {
        selected_map: SELECTED_MAP,
        build: 70_450,
        locale: locale.name().to_owned(),
        plan_sha256: PLAN_SHA256,
        producer_sha256: PRODUCER_SHA256,
        current_source_sha256: SOURCE_SHA256,
        map: MapSnapshot {
            file_data_id: 1_349_477,
            declared_bytes: 1_000,
            available_bytes: 800,
            full_ckey_verified: false,
            encoded_key_authenticated: true,
            range_count: 2,
            section_count: 1,
            declared_direct_rows: 1,
            declared_copy_pairs: 1,
            direct_rows: 1,
            copy_pairs: 0,
            terrain_materialized_rows: 1,
            vmap_materialized_rows: 1,
            decoded_sha256: [0x51; 32],
            snapshot_sha256: [0x52; 32],
        },
        closure: vec![
            ClosureMap {
                id: 1,
                root: 1,
                parent: None,
                cosmetic_parent: None,
                wdt_file_data_id: 1_199,
                output: false,
                is_parent: true,
                tiles: 0,
            },
            ClosureMap {
                id: SELECTED_MAP,
                root: 1,
                parent: Some(1),
                cosmetic_parent: None,
                wdt_file_data_id: 1_200,
                output: true,
                is_parent: false,
                tiles: 4,
            },
        ],
        liquid: LiquidSnapshot {
            file_data_id: 1_371_380,
            bytes: 256,
            direct_rows: 3,
            copy_pairs: 1,
            sha256: [0x53; 32],
        },
        terrain: package_identity(Component::Terrain.bit(), [0x61; 32], 0x71),
        terrain_maps: vec![
            TerrainMap {
                id: 1,
                loaded: true,
                known_present: 0,
                converted: 0,
                missing_root_adts: 0,
            },
            TerrainMap {
                id: SELECTED_MAP,
                loaded: true,
                known_present: 0,
                converted: 0,
                missing_root_adts: 0,
            },
        ],
        vmap: package_identity(Component::Collision.bit(), [0x62; 32], 0x72),
    };

    let context = Context {
        build: 70_450,
        locale,
        source_sha256: [0x11; 32],
        plan_sha256: PLAN_SHA256,
        producer_sha256: PRODUCER_SHA256,
        components: Component::Navigation.bit(),
    };
    let mut output_tiles = Vec::new();
    for coordinate in [WRITTEN_A, WRITTEN_C, EMPTY, WRITTEN_B] {
        if let Some((_, bytes)) = tiles.iter().find(|(tile, _)| *tile == coordinate) {
            output_tiles.push(TileOutput {
                tile: coordinate,
                written: true,
                bytes: bytes.len() as u64,
                sha256: integrity::digest(bytes),
            });
        } else {
            output_tiles.push(TileOutput {
                tile: coordinate,
                written: false,
                bytes: 0,
                sha256: [0; 32],
            });
        }
    }
    let written_bytes = output_tiles.iter().map(|tile| tile.bytes).sum::<u64>();
    let written_tiles = output_tiles.iter().filter(|tile| tile.written).count();
    let empty_tiles = output_tiles.len() - written_tiles;
    let mut coverage = Coverage {
        context,
        identity,
        global_bytes: global.len() as u64,
        global_sha256: integrity::digest(global),
        tiles: output_tiles,
        maps: 1,
        written_tiles,
        empty_tiles,
        output_bytes: global.len() as u64 + written_bytes,
        builder_work: 7,
        save_work: 4,
        metadata_calls,
    };
    coverage.context.source_sha256 = integrity::digest(
        coverage
            .identity
            .render()
            .expect("synthetic identity")
            .as_bytes(),
    );
    coverage
}

fn package_identity(component: u8, plan_sha256: [u8; 32], inventory: u8) -> PackageIdentity {
    let context = Context {
        build: 70_450,
        locale: Locale::EnUs,
        source_sha256: SOURCE_SHA256,
        plan_sha256,
        producer_sha256: PRODUCER_SHA256,
        components: component,
    };
    PackageIdentity {
        context,
        plan_sha256,
        files: 1,
        bytes: 128,
        inventory_sha256: [inventory; 32],
        coverage_sha256: [inventory + 1; 32],
    }
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub struct SyntheticNavigation56Package {
    root: PathBuf,
    pub output: PathBuf,
    pub expected: forever_navigation::package_v56::Expected,
}

impl SyntheticNavigation56Package {
    pub fn create() -> Self {
        use forever_navigation::package_v56::{self, Coverage56, MapFact, Output};

        let params =
            tilebuilder::global_params(&[WRITTEN_A, WRITTEN_B]).expect("synthetic v56 inventory");
        let global = mmap::encode_global(SELECTED_MAP, params, &[], mmap::Limits::default())
            .expect("synthetic v56 global");
        let written_tiles: Vec<_> = [WRITTEN_A, WRITTEN_C, WRITTEN_B]
            .into_iter()
            .map(|coordinate| (coordinate, tile_container(coordinate, params, false, false)))
            .collect();
        let output_tiles: Vec<_> = [WRITTEN_A, WRITTEN_C, EMPTY, WRITTEN_B]
            .into_iter()
            .map(|coordinate| {
                if let Some((_, bytes)) = written_tiles.iter().find(|(tile, _)| *tile == coordinate)
                {
                    TileOutput {
                        tile: coordinate,
                        written: true,
                        bytes: bytes.len() as u64,
                        sha256: integrity::digest(bytes),
                    }
                } else {
                    TileOutput {
                        tile: coordinate,
                        written: false,
                        bytes: 0,
                        sha256: [0; 32],
                    }
                }
            })
            .collect();
        let source = Summary {
            context: Context {
                build: 70_450,
                locale: Locale::EnUs,
                source_sha256: SOURCE_SHA256,
                plan_sha256: [0x51; 32],
                producer_sha256: PRODUCER_SHA256,
                components: 25,
            },
            files: 1,
            bytes: 128,
            inventory_sha256: [0x53; 32],
        };
        let context = Context {
            build: 70_450,
            locale: Locale::EnUs,
            source_sha256: integrity::digest(
                format!("rustycore-navigation-v56-sealed-v51\n{}\n", source.json()).as_bytes(),
            ),
            plan_sha256: PLAN_SHA256,
            producer_sha256: PRODUCER_SHA256,
            components: Component::Navigation.bit(),
        };
        let coverage = Coverage56 {
            context,
            source,
            maps: vec![
                MapFact {
                    id: SELECTED_MAP,
                    root: SELECTED_MAP,
                    state: "planned",
                    tiles: vec![WRITTEN_A, WRITTEN_C, EMPTY, WRITTEN_B],
                },
                MapFact {
                    id: 49,
                    root: 49,
                    state: "empty",
                    tiles: Vec::new(),
                },
                MapFact {
                    id: 50,
                    root: 50,
                    state: "root-absent",
                    tiles: Vec::new(),
                },
            ],
            outputs: vec![
                Output {
                    map: SELECTED_MAP,
                    global: Some(TileOutput {
                        tile: Coordinate { row: 0, column: 0 },
                        written: true,
                        bytes: global.len() as u64,
                        sha256: integrity::digest(&global),
                    }),
                    tiles: output_tiles,
                    partial_triangulations: 0,
                    detail_center_fallbacks: 0,
                    unset_height_samples: 0,
                },
                Output {
                    map: 49,
                    global: None,
                    tiles: Vec::new(),
                    partial_triangulations: 0,
                    detail_center_fallbacks: 0,
                    unset_height_samples: 0,
                },
                Output {
                    map: 50,
                    global: None,
                    tiles: Vec::new(),
                    partial_triangulations: 0,
                    detail_center_fallbacks: 0,
                    unset_height_samples: 0,
                },
            ],
            builder: 0,
            save: 0,
        };
        let coverage_bytes = coverage.render().expect("valid synthetic v56 coverage");
        let root = std::env::temp_dir().join(format!(
            "rustycore-navigation56-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create synthetic v56 parent");
        let output = root.join("package");
        let mut run = Run::create(&output, context, &[], package::Limits::default())
            .expect("create synthetic v56 run");
        run.put(
            Component::Navigation,
            package_v56::COVERAGE_PATH,
            &coverage_bytes,
        )
        .expect("write synthetic v56 coverage");
        run.put(
            Component::Navigation,
            &mmap::global_path(SELECTED_MAP),
            &global,
        )
        .expect("write synthetic v56 global");
        for (coordinate, bytes) in written_tiles {
            run.put(
                Component::Navigation,
                &mmap::tile_path(SELECTED_MAP, coordinate.row, coordinate.column).unwrap(),
                &bytes,
            )
            .expect("write synthetic v56 tile");
        }
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        Self {
            root,
            output,
            expected: package_v56::Expected {
                selected_map: SELECTED_MAP,
                locale: Locale::EnUs,
                plan_sha256: PLAN_SHA256,
                context: Some(context),
            },
        }
    }
}

impl Drop for SyntheticNavigation56Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct SyntheticNavigation57Package {
    root: PathBuf,
    pub output: PathBuf,
    pub expected: forever_navigation::package_v57::Expected,
}

impl SyntheticNavigation57Package {
    pub fn create() -> Self {
        use forever_navigation::package_v57::{self, Coverage57, MapFact, Output};

        let params = tilebuilder::global_params(&[WRITTEN_A]).expect("synthetic v57 inventory");
        let global = mmap::encode_global(SELECTED_MAP, params, &[], mmap::Limits::default())
            .expect("synthetic v57 global");
        let tile = tile_container(WRITTEN_A, params, false, false);
        let source = Summary {
            context: Context {
                build: 70_450,
                locale: Locale::EnUs,
                source_sha256: SOURCE_SHA256,
                plan_sha256: [0x51; 32],
                producer_sha256: PRODUCER_SHA256,
                components: 25,
            },
            files: 1,
            bytes: 128,
            inventory_sha256: [0x53; 32],
        };
        let context = Context {
            build: 70_450,
            locale: Locale::EnUs,
            source_sha256: integrity::digest(
                format!("rustycore-navigation-v57-sealed-v51\n{}\n", source.json()).as_bytes(),
            ),
            plan_sha256: PLAN_SHA256,
            producer_sha256: PRODUCER_SHA256,
            components: Component::Navigation.bit(),
        };
        let coverage = Coverage57 {
            context,
            source,
            maps: vec![MapFact {
                id: SELECTED_MAP,
                root: SELECTED_MAP,
                state: "planned",
                tiles: vec![WRITTEN_A],
            }],
            outputs: vec![Output {
                map: SELECTED_MAP,
                global: Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: global.len() as u64,
                    sha256: integrity::digest(&global),
                }),
                tiles: vec![TileOutput {
                    tile: WRITTEN_A,
                    written: true,
                    bytes: tile.len() as u64,
                    sha256: integrity::digest(&tile),
                }],
                partial_triangulations: 0,
                detail_center_fallbacks: 0,
                unset_height_samples: 0,
            }],
            builder: 0,
            save: 0,
        };
        let root = std::env::temp_dir().join(format!(
            "rustycore-navigation57-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create synthetic v57 parent");
        let output = root.join("package");
        let mut run = Run::create(&output, context, &[], package::Limits::default())
            .expect("create synthetic v57 run");
        run.put(
            Component::Navigation,
            package_v57::COVERAGE_PATH,
            &coverage.render().unwrap(),
        )
        .expect("write synthetic v57 coverage");
        run.put(
            Component::Navigation,
            &mmap::global_path(SELECTED_MAP),
            &global,
        )
        .expect("write synthetic v57 global");
        run.put(
            Component::Navigation,
            &mmap::tile_path(SELECTED_MAP, WRITTEN_A.row, WRITTEN_A.column).unwrap(),
            &tile,
        )
        .expect("write synthetic v57 tile");
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        Self {
            root,
            output,
            expected: package_v57::Expected {
                selected_map: SELECTED_MAP,
                locale: Locale::EnUs,
                plan_sha256: PLAN_SHA256,
                context: Some(context),
            },
        }
    }
}

impl Drop for SyntheticNavigation57Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
