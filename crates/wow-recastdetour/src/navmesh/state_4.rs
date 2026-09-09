//! Navmesh FFI wrapper state definitions, part 4 of 4.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

impl MmapTileHeader {
    #[must_use]
    pub const fn new(dt_version: u32) -> Self {
        Self {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 0,
            uses_liquids: true,
            padding: [0; 3],
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, MmapTileHeaderError> {
        if bytes.len() < MMAP_TILE_HEADER_SIZE_LIKE_CPP {
            return Err(MmapTileHeaderError::TooShort {
                actual: bytes.len(),
                expected: MMAP_TILE_HEADER_SIZE_LIKE_CPP,
            });
        }

        let header = Self {
            mmap_magic: read_u32(bytes, 0),
            dt_version: read_u32(bytes, 4),
            mmap_version: read_u32(bytes, 8),
            size: read_u32(bytes, 12),
            uses_liquids: bytes[16] != 0,
            padding: [bytes[17], bytes[18], bytes[19]],
        };

        if header.mmap_magic != MMAP_MAGIC_LIKE_CPP {
            return Err(MmapTileHeaderError::BadMagic {
                actual: header.mmap_magic,
                expected: MMAP_MAGIC_LIKE_CPP,
            });
        }

        if header.mmap_version != MMAP_VERSION_LIKE_CPP {
            return Err(MmapTileHeaderError::BadMmapVersion {
                actual: header.mmap_version,
                expected: MMAP_VERSION_LIKE_CPP,
            });
        }

        Ok(header)
    }

    pub fn validate_dt_version(&self, expected_dt_version: u32) -> Result<(), MmapTileHeaderError> {
        if self.dt_version != expected_dt_version {
            return Err(MmapTileHeaderError::BadDetourVersion {
                actual: self.dt_version,
                expected: expected_dt_version,
            });
        }

        Ok(())
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; MMAP_TILE_HEADER_SIZE_LIKE_CPP] {
        let mut bytes = [0; MMAP_TILE_HEADER_SIZE_LIKE_CPP];
        bytes[0..4].copy_from_slice(&self.mmap_magic.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.dt_version.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.mmap_version.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.size.to_le_bytes());
        bytes[16] = u8::from(self.uses_liquids);
        bytes[17..20].copy_from_slice(&self.padding);
        bytes
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MmapTileHeaderError {
    #[error("mmap tile header is too short: got {actual} bytes, expected {expected}")]
    TooShort { actual: usize, expected: usize },
    #[error("bad mmap magic: got 0x{actual:08x}, expected 0x{expected:08x}")]
    BadMagic { actual: u32, expected: u32 },
    #[error("bad mmap version: got {actual}, expected {expected}")]
    BadMmapVersion { actual: u32, expected: u32 },
    #[error("bad Detour navmesh version: got {actual}, expected {expected}")]
    BadDetourVersion { actual: u32, expected: u32 },
}

#[must_use]
pub const fn pack_tile_id_like_cpp(x: i32, y: i32) -> u32 {
    ((x as u32) << 16) | (y as u32 & 0xffff)
}

#[must_use]
pub fn mmap_tile_coords_for_wow_position_like_cpp(x: f32, y: f32) -> (i32, i32) {
    (
        (CENTER_GRID_ID_LIKE_CPP as f32 - x / SIZE_OF_GRIDS_LIKE_CPP) as i32,
        (CENTER_GRID_ID_LIKE_CPP as f32 - y / SIZE_OF_GRIDS_LIKE_CPP) as i32,
    )
}

#[must_use]
pub fn map_file_name_like_cpp(map_id: u32) -> String {
    format!("mmaps/{map_id:04}.mmap")
}

#[must_use]
pub fn map_file_path_like_cpp(base_path: impl AsRef<Path>, map_id: u32) -> PathBuf {
    base_path.as_ref().join(map_file_name_like_cpp(map_id))
}

#[must_use]
pub fn tile_file_name_like_cpp(map_id: u32, x: i32, y: i32) -> String {
    format!("mmaps/{map_id:04}{x:02}{y:02}.mmtile")
}

#[must_use]
pub fn tile_file_path_like_cpp(
    base_path: impl AsRef<Path>,
    map_id: u32,
    x: i32,
    y: i32,
) -> PathBuf {
    base_path
        .as_ref()
        .join(tile_file_name_like_cpp(map_id, x, y))
}

pub fn read_mmap_tile_blob_file(
    path: impl AsRef<Path>,
    expected_dt_version: u32,
) -> Result<MmapTileBlob, MmapTileFileError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| MmapTileFileError::ReadTileFile {
        path: path.to_path_buf(),
        source,
    })?;

    MmapTileBlob::parse(&bytes, expected_dt_version).map_err(MmapTileFileError::BadTileBlob)
}

#[derive(Debug, Error)]
pub enum MmapTileFileError {
    #[error("failed to read mmap tile file {path:?}: {source}")]
    ReadTileFile { path: PathBuf, source: io::Error },
    #[error("bad mmap tile blob: {0}")]
    BadTileBlob(MmapTileBlobError),
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub(crate) fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub(crate) fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
