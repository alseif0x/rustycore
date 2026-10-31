// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic MAPS/VMAP wire, no client bytes.
#![allow(dead_code)]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Fixture {
    pub root: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let parent = std::env::temp_dir().canonicalize().unwrap();
        assert!(
            !parent.ancestors().any(|p| p.join(".git").exists()),
            "fixture root must be outside Git"
        );
        let root = parent.join(format!(
            "forever-terrain-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }
    pub fn plane(&self) {
        let map_dir = self.root.join("0001");
        fs::create_dir(&map_dir).unwrap();
        let mid = f32::from_bits(0x4685_5555);
        let bounds = [mid, mid, 0., mid + 4., mid + 4., 0.];
        let mut map = b"VMAP_4.ENODE".to_vec();
        map.extend(bih(bounds));
        fs::write(map_dir.join("0001.vmtree"), map).unwrap();
        let mut tile = b"VMAP_4.E".to_vec();
        words(&mut tile, &[1]);
        tile.extend([1, 0]);
        words(&mut tile, &[9]);
        floats(&mut tile, &[mid, mid, 0., 0., 0., 0., 1.]);
        floats(&mut tile, &bounds);
        words(&mut tile, &[5]);
        tile.extend(b"plane");
        fs::write(map_dir.join("0001_32_32.vmtile"), tile).unwrap();
        let mut index = b"VMAP_4.E".to_vec();
        words(&mut index, &[1, 0]);
        fs::write(map_dir.join("0001_32_32.vmtileidx"), index).unwrap();
        let bounds = [0., 0., 0., 4., 4., 0.];
        let mut model = b"VMAP_4.EWMOD".to_vec();
        words(&mut model, &[8, 0, 777]);
        model.extend(b"GMOD");
        words(&mut model, &[1]);
        floats(&mut model, &bounds);
        words(&mut model, &[0, 7]);
        model.extend(b"VERT");
        words(&mut model, &[40, 3]);
        floats(&mut model, &[0., 0., 0., 4., 0., 0., 0., 4., 0.]);
        model.extend(b"TRIM");
        words(&mut model, &[16, 1, 0, 1, 2]);
        model.extend(b"MBIH");
        model.extend(bih(bounds));
        model.extend(b"LIQU");
        words(&mut model, &[0]);
        model.extend(b"GBIH");
        model.extend(bih(bounds));
        fs::write(self.root.join("plane.vmo"), model).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn words(out: &mut Vec<u8>, words: &[u32]) {
    for word in words {
        out.extend(word.to_le_bytes());
    }
}
fn floats(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        words(out, &[value.to_bits()]);
    }
}
fn bih(bounds: [f32; 6]) -> Vec<u8> {
    let mut out = Vec::new();
    floats(&mut out, &bounds);
    words(&mut out, &[3, 0xc000_0000, 1, 0, 1, 0]);
    out
}
pub fn ground(build: u32, height: f32, encoding: u32, holes: bool) -> Vec<u8> {
    let sample_bytes = match encoding {
        1 => 0,
        2 => 2,
        4 => 1,
        0 => 4,
        _ => panic!("fixture encoding"),
    };
    let height_len = 16 + (129 * 129 + 128 * 128) * sample_bytes;
    let mut out = b"MAPS".to_vec();
    words(
        &mut out,
        &[
            10,
            build,
            44,
            8,
            52,
            height_len,
            0,
            0,
            if holes { 52 + height_len } else { 0 },
            if holes { 2048 } else { 0 },
        ],
    );
    out.extend(b"AREA");
    out.extend([1, 0, 0, 0]);
    out.extend(b"MHGT");
    words(&mut out, &[encoding]);
    floats(&mut out, &[height, height + 1.]);
    if encoding == 0 {
        for _ in 0..129 * 129 + 128 * 128 {
            floats(&mut out, &[height]);
        }
    } else {
        out.resize(52 + height_len as usize, 0);
    }
    if holes {
        out.resize(out.len() + 2048, 255);
    }
    out
}
