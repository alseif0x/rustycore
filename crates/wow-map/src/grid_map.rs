//! GridMap — parser and height query for a single TrinityCore `.map` terrain tile.
//!
//! Faithful port of `GridMap::loadHeightData` + `getHeightFrom{Float,Uint16,Uint8,Flat}`
//! and `isHole` from the legacy C++ (`src/server/game/Maps/GridMap.cpp`) for WoW 3.4.3.
//! Tile files are `maps/{mapId:04}_{gridX:02}_{gridY:02}.map`, magic `"MAPS"` v10
//! (`src/common/Collision/Maps/MapDefines.{h,cpp}`).
//!
//! Scope (issue [03]/#15, slice A): **grid terrain height only**. VMap, liquid,
//! flight bounds, and the area map are intentionally not parsed here; only the
//! `MHGT` height chunk (+ holes, which `getHeight` consults) is loaded.

// C++ GridDefines.h
const MAP_RESOLUTION: i32 = 128;
const SIZE_OF_GRIDS: f32 = 533.333_3;
const CENTER_GRID_ID: f32 = 32.0;

/// C++ `INVALID_HEIGHT` sentinel (GridDefines.h) — "no terrain height here".
pub const INVALID_HEIGHT: f32 = -100_000.0;

const MAP_MAGIC: [u8; 4] = *b"MAPS";
const MAP_VERSION_MAGIC: u32 = 10;
const HEIGHT_MAGIC: [u8; 4] = *b"MHGT";

// map_heightHeaderFlags (MapDefines.h)
const FLAG_NO_HEIGHT: u32 = 0x0001;
const FLAG_HEIGHT_AS_INT16: u32 = 0x0002;
const FLAG_HEIGHT_AS_INT8: u32 = 0x0004;

// V9 = 129x129 corner grid, V8 = 128x128 cell-center grid.
const V9_SIZE: usize = 129 * 129;
const V8_SIZE: usize = 128 * 128;
const HOLES_SIZE: usize = 16 * 16 * 8;

/// Packed height storage, mirroring the three C++ on-disk encodings + the flat case.
#[derive(Debug)]
enum Heights {
    /// `NoHeight` flag: every query returns `grid_height`.
    Flat,
    Float {
        v9: Vec<f32>,
        v8: Vec<f32>,
    },
    Int16 {
        v9: Vec<u16>,
        v8: Vec<u16>,
        mult: f32,
    },
    Int8 {
        v9: Vec<u8>,
        v8: Vec<u8>,
        mult: f32,
    },
}

/// One parsed `.map` terrain tile's height data.
#[derive(Debug)]
pub struct GridMap {
    grid_height: f32,
    heights: Heights,
    /// 16*16*8 hole bitmask, when the tile has holes.
    holes: Option<Vec<u8>>,
}

/// Minimal little-endian reader over the `.map` bytes.
struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    fn u32_at(&self, off: usize) -> Option<u32> {
        let b = self.buf.get(off..off + 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32_at(&self, off: usize) -> Option<f32> {
        Some(f32::from_bits(self.u32_at(off)?))
    }
    fn tag_at(&self, off: usize) -> Option<[u8; 4]> {
        let b = self.buf.get(off..off + 4)?;
        Some([b[0], b[1], b[2], b[3]])
    }
}

impl GridMap {
    /// Parse a `.map` tile from its raw bytes, loading only the height chunk.
    ///
    /// Returns `None` if the file magic/version is wrong, the height magic is
    /// wrong, or the height arrays are truncated — mirroring C++ `loadData`
    /// returning failure (the caller then treats the tile as "no terrain").
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<GridMap> {
        let r = Reader { buf: bytes };

        // map_fileheader (MapDefines.h): magic[4], version, build, then 4 (offset,size)
        // pairs (area, height, liquid, holes).
        if r.tag_at(0)? != MAP_MAGIC || r.u32_at(4)? != MAP_VERSION_MAGIC {
            return None;
        }
        let height_offset = r.u32_at(20)? as usize;
        let holes_offset = r.u32_at(36)? as usize;
        let holes_size = r.u32_at(40)? as usize;

        // map_heightHeader: magic[4], flags(u32), gridHeight(f32), gridMaxHeight(f32).
        if r.tag_at(height_offset)? != HEIGHT_MAGIC {
            return None;
        }
        let flags = r.u32_at(height_offset + 4)?;
        let grid_height = r.f32_at(height_offset + 8)?;
        let grid_max_height = r.f32_at(height_offset + 12)?;
        let data_off = height_offset + 16;

        let heights = if flags & FLAG_NO_HEIGHT != 0 {
            Heights::Flat
        } else if flags & FLAG_HEIGHT_AS_INT16 != 0 {
            let v9 = read_u16_array(&r, data_off, V9_SIZE)?;
            let v8 = read_u16_array(&r, data_off + V9_SIZE * 2, V8_SIZE)?;
            Heights::Int16 {
                v9,
                v8,
                mult: (grid_max_height - grid_height) / 65535.0,
            }
        } else if flags & FLAG_HEIGHT_AS_INT8 != 0 {
            let v9 = bytes.get(data_off..data_off + V9_SIZE)?.to_vec();
            let v8 = bytes
                .get(data_off + V9_SIZE..data_off + V9_SIZE + V8_SIZE)?
                .to_vec();
            Heights::Int8 {
                v9,
                v8,
                mult: (grid_max_height - grid_height) / 255.0,
            }
        } else {
            let v9 = read_f32_array(&r, data_off, V9_SIZE)?;
            let v8 = read_f32_array(&r, data_off + V9_SIZE * 4, V8_SIZE)?;
            Heights::Float { v9, v8 }
        };

        let holes = if holes_size != 0 {
            Some(bytes.get(holes_offset..holes_offset + HOLES_SIZE)?.to_vec())
        } else {
            None
        };

        Some(GridMap {
            grid_height,
            heights,
            holes,
        })
    }

    /// Terrain height at world `(x, y)` for this tile, or [`INVALID_HEIGHT`] in a hole.
    ///
    /// Port of `GridMap::getHeightFrom*` (GridMap.cpp). The interpolation is
    /// identical across encodings (triangle pick + linear solve); only the source
    /// type and the int→world scale differ.
    #[must_use]
    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        if let Heights::Flat = self.heights {
            return self.grid_height;
        }

        let gx = MAP_RESOLUTION as f32 * (CENTER_GRID_ID - x / SIZE_OF_GRIDS);
        let gy = MAP_RESOLUTION as f32 * (CENTER_GRID_ID - y / SIZE_OF_GRIDS);
        let xi_raw = gx as i32;
        let yi_raw = gy as i32;
        let fx = gx - xi_raw as f32;
        let fy = gy - yi_raw as f32;
        let xi = (xi_raw & (MAP_RESOLUTION - 1)) as usize;
        let yi = (yi_raw & (MAP_RESOLUTION - 1)) as usize;

        if self.is_hole(xi, yi) {
            return INVALID_HEIGHT;
        }

        // h1..h4 = V9 corners, h5 = 2 * V8 cell center (C++ getHeightFromFloat).
        let v9 = |dx: usize, dy: usize| -> f32 {
            let idx = (xi + dx) * 129 + (yi + dy);
            match &self.heights {
                Heights::Float { v9, .. } => v9[idx],
                Heights::Int16 { v9, .. } => f32::from(v9[idx]),
                Heights::Int8 { v9, .. } => f32::from(v9[idx]),
                Heights::Flat => unreachable!(),
            }
        };
        let v8_center = || -> f32 {
            let idx = xi * 128 + yi;
            match &self.heights {
                Heights::Float { v8, .. } => v8[idx],
                Heights::Int16 { v8, .. } => f32::from(v8[idx]),
                Heights::Int8 { v8, .. } => f32::from(v8[idx]),
                Heights::Flat => unreachable!(),
            }
        };

        let h1 = v9(0, 0);
        let h2 = v9(1, 0);
        let h3 = v9(0, 1);
        let h4 = v9(1, 1);
        let h5 = 2.0 * v8_center();

        let (a, b, c) = if fx + fy < 1.0 {
            if fx > fy {
                (h2 - h1, h5 - h1 - h2, h1) // tri 1: h1,h2,h5
            } else {
                (h5 - h1 - h3, h3 - h1, h1) // tri 2: h1,h3,h5
            }
        } else if fx > fy {
            (h2 + h4 - h5, h4 - h2, h5 - h4) // tri 3: h2,h4,h5
        } else {
            (h4 - h3, h3 + h4 - h5, h5 - h4) // tri 4: h3,h4,h5
        };

        let interpolated = a * fx + b * fy + c;
        match &self.heights {
            // Int variants store raw values; scale into world height (GridMap.cpp:461/531).
            Heights::Int16 { mult, .. } | Heights::Int8 { mult, .. } => {
                interpolated * *mult + self.grid_height
            }
            _ => interpolated,
        }
    }

    /// Port of `GridMap::isHole` (GridMap.cpp:534): 8x8 sub-squares per cell.
    fn is_hole(&self, row: usize, col: usize) -> bool {
        let Some(holes) = &self.holes else {
            return false;
        };
        let cell_row = row / 8;
        let cell_col = col / 8;
        let hole_row = row % 8;
        let hole_col = col % 8;
        (holes[cell_row * 16 * 8 + cell_col * 8 + hole_row] & (1 << hole_col)) != 0
    }
}

fn read_f32_array(r: &Reader<'_>, off: usize, count: usize) -> Option<Vec<f32>> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(r.f32_at(off + i * 4)?);
    }
    Some(out)
}

fn read_u16_array(r: &Reader<'_>, off: usize, count: usize) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let b = r.buf.get(off + i * 2..off + i * 2 + 2)?;
        out.push(u16::from_le_bytes([b[0], b[1]]));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic float `.map` tile: V9/V8 filled by the given closures.
    fn build_float_map(
        grid_height: f32,
        v9: impl Fn(usize, usize) -> f32,
        v8: impl Fn(usize, usize) -> f32,
        holes: Option<&[u8; HOLES_SIZE]>,
    ) -> Vec<u8> {
        // Layout: [fileheader 44][heightheader 16][V9][V8]([holes]).
        let height_off = 44u32;
        let data_off = height_off + 16;
        let v9_bytes = V9_SIZE * 4;
        let v8_bytes = V8_SIZE * 4;
        let holes_off = data_off + v9_bytes as u32 + v8_bytes as u32;
        let holes_size = if holes.is_some() {
            HOLES_SIZE as u32
        } else {
            0
        };

        let mut b = Vec::new();
        b.extend_from_slice(&MAP_MAGIC);
        b.extend_from_slice(&MAP_VERSION_MAGIC.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // build
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
        b.extend_from_slice(&height_off.to_le_bytes()); // heightMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // heightMapSize
        b.extend_from_slice(&0u32.to_le_bytes()); // liquidMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // liquidMapSize
        b.extend_from_slice(&holes_off.to_le_bytes()); // holesOffset
        b.extend_from_slice(&holes_size.to_le_bytes()); // holesSize

        // height header
        b.extend_from_slice(&HEIGHT_MAGIC);
        b.extend_from_slice(&0u32.to_le_bytes()); // flags (float)
        b.extend_from_slice(&grid_height.to_le_bytes());
        b.extend_from_slice(&grid_height.to_le_bytes()); // gridMaxHeight (unused for float)
        for r in 0..129 {
            for c in 0..129 {
                b.extend_from_slice(&v9(r, c).to_le_bytes());
            }
        }
        for r in 0..128 {
            for c in 0..128 {
                b.extend_from_slice(&v8(r, c).to_le_bytes());
            }
        }
        if let Some(h) = holes {
            b.extend_from_slice(h);
        }
        b
    }

    #[test]
    fn flat_tile_returns_grid_height() {
        // NoHeight flag → getHeightFromFlat returns gridHeight everywhere.
        let mut b = Vec::new();
        b.extend_from_slice(&MAP_MAGIC);
        b.extend_from_slice(&MAP_VERSION_MAGIC.to_le_bytes());
        for _ in 0..9 {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        // patch heightMapOffset (index 5 → byte 20) to 44
        b[20..24].copy_from_slice(&44u32.to_le_bytes());
        b.extend_from_slice(&HEIGHT_MAGIC);
        b.extend_from_slice(&FLAG_NO_HEIGHT.to_le_bytes());
        b.extend_from_slice(&123.5f32.to_le_bytes()); // gridHeight
        b.extend_from_slice(&123.5f32.to_le_bytes());

        let gm = GridMap::parse(&b).expect("flat map parses");
        assert_eq!(gm.get_height(0.0, 0.0), 123.5);
        assert_eq!(gm.get_height(100.0, -250.0), 123.5);
    }

    #[test]
    fn constant_float_tile_returns_that_constant() {
        let b = build_float_map(0.0, |_, _| 75.0, |_, _| 75.0, None);
        let gm = GridMap::parse(&b).expect("parses");
        // For a constant grid every triangle solves to the constant.
        assert!((gm.get_height(0.0, 0.0) - 75.0).abs() < 1e-3);
        assert!((gm.get_height(123.4, -456.7) - 75.0).abs() < 1e-3);
    }

    #[test]
    fn planar_ramp_interpolates_like_cpp() {
        // A grid that increases linearly with the V9 row index. At x=0 the world
        // maps to V9 row 0 (gx = 128*32 = 4096 → x_int&127 = 0, frac 0); pick a y
        // that lands mid-cell to exercise interpolation, and compare against the
        // same triangle math computed here.
        let v9 = |r: usize, _c: usize| r as f32; // height == row index
        let v8 = |r: usize, _c: usize| r as f32 + 0.5; // cell-center between rows
        let b = build_float_map(0.0, v9, v8, None);
        let gm = GridMap::parse(&b).expect("parses");

        // x=0 → row 0, frac 0; y chosen so gy frac is ~0 too (column boundary).
        // At a corner (frac 0,0) the result is exactly h1 = V9[0,0] = 0.
        assert!(gm.get_height(0.0, 0.0).abs() < 1e-3);

        // One full V9 row south is +1 in height. x for row 1: gx=4097 → need
        // x/SIZE_OF_GRIDS such that 128*(32 - x/533.3333) = 4097 → x = -SIZE/128.
        let x_row1 = -SIZE_OF_GRIDS / 128.0;
        assert!((gm.get_height(x_row1, 0.0) - 1.0).abs() < 1e-2);
    }

    #[test]
    fn hole_cell_returns_invalid_height() {
        let mut holes = [0u8; HOLES_SIZE];
        // Mark sub-square (row 0, col 0): cell (0,0), holeRow 0, holeCol 0 → bit 0.
        holes[0] = 0b0000_0001;
        let b = build_float_map(0.0, |_, _| 10.0, |_, _| 10.0, Some(&holes));
        let gm = GridMap::parse(&b).expect("parses");
        // x=0,y=0 → x_int=0,y_int=0 → hole.
        assert_eq!(gm.get_height(0.0, 0.0), INVALID_HEIGHT);
    }

    #[test]
    fn int16_tile_scales_into_world_height() {
        // gridHeight 0, gridMaxHeight 65535 → mult = 1.0, raw value == world height.
        let height_off = 44u32;
        let mut b = Vec::new();
        b.extend_from_slice(&MAP_MAGIC);
        b.extend_from_slice(&MAP_VERSION_MAGIC.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // build
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
        b.extend_from_slice(&height_off.to_le_bytes());
        for _ in 0..5 {
            b.extend_from_slice(&0u32.to_le_bytes()); // remaining offsets/sizes
        }
        b.extend_from_slice(&HEIGHT_MAGIC);
        b.extend_from_slice(&FLAG_HEIGHT_AS_INT16.to_le_bytes());
        b.extend_from_slice(&0.0f32.to_le_bytes()); // gridHeight
        b.extend_from_slice(&65535.0f32.to_le_bytes()); // gridMaxHeight → mult 1.0
        for _ in 0..V9_SIZE {
            b.extend_from_slice(&500u16.to_le_bytes());
        }
        for _ in 0..V8_SIZE {
            b.extend_from_slice(&500u16.to_le_bytes());
        }
        let gm = GridMap::parse(&b).expect("int16 parses");
        // constant 500 raw * mult 1.0 + base 0 = 500.
        assert!((gm.get_height(0.0, 0.0) - 500.0).abs() < 1e-2);
    }

    #[test]
    fn rejects_bad_file_magic() {
        let mut b = vec![0u8; 64];
        b[0..4].copy_from_slice(b"XXXX");
        assert!(GridMap::parse(&b).is_none());
    }

    #[test]
    fn rejects_truncated_height_arrays() {
        let mut b = build_float_map(0.0, |_, _| 1.0, |_, _| 1.0, None);
        b.truncate(60); // header ok, arrays cut off
        assert!(GridMap::parse(&b).is_none());
    }
}
