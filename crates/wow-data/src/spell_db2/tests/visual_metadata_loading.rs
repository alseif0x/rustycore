use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let nonce = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rustycore-visual-loader-copy-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("data/dbc/enUS"))
            .expect("create visual DB2 fixture directory");
        Self(root)
    }

    fn data_dir(&self) -> String {
        self.0
            .join("data")
            .to_str()
            .expect("temporary path is UTF-8")
            .to_string()
    }

    fn write_incomplete_copy(&self, file_name: &str) {
        fs::write(
            self.0.join("data/dbc/enUS").join(file_name),
            unresolved_copy_wdc4(),
        )
        .expect("write unresolved-copy WDC4 fixture");
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unresolved_copy_wdc4() -> Vec<u8> {
    const HEADER_SIZE: usize = 72;
    const SECTION_HEADER_SIZE: usize = 40;
    let data_offset = HEADER_SIZE + SECTION_HEADER_SIZE;
    let mut bytes = vec![0; data_offset + 8];
    put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
    put_u32(&mut bytes, 12, 1);
    put_u32(&mut bytes, 20, 0xABCD_1234);
    put_u16(&mut bytes, 42, u16::MAX);
    put_u32(&mut bytes, 68, 1);
    put_u32(&mut bytes, HEADER_SIZE + 8, data_offset as u32);
    put_u32(&mut bytes, HEADER_SIZE + 36, 1);
    put_u32(&mut bytes, data_offset, 900);
    put_u32(&mut bytes, data_offset + 4, 901);
    bytes
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn unresolved_error(result: anyhow::Result<impl Sized>) -> String {
    let error = result
        .err()
        .expect("complete loader rejects unresolved WDC4 copy sources");
    let message = error.to_string();
    assert!(
        message.contains("unresolved WDC4 copy records"),
        "{message}"
    );
    message
}

#[test]
fn visual_metadata_loaders_require_complete_copy_resolution_before_overlays() {
    let fixtures = FixtureDirectory::new();
    fixtures.write_incomplete_copy("SpellEffect.db2");
    fixtures.write_incomplete_copy("SpellXSpellVisual.db2");
    let data_dir = fixtures.data_dir();
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::default();

    let legacy_effects = SpellEffectDb2Store::load(&data_dir, "enUS")
        .expect("legacy effect load retains its file-only behavior");
    assert!(legacy_effects.is_empty());
    let legacy_visuals = SpellXSpellVisualStore::load(&data_dir, "enUS")
        .expect("legacy visual load retains its file-only behavior");
    assert!(legacy_visuals.is_empty());

    unresolved_error(SpellEffectDb2Store::load_complete(&data_dir, "enUS"));
    unresolved_error(SpellXSpellVisualStore::load_complete(&data_dir, "enUS"));

    let effect_overlay = test_spell_effect_entry(900, 73_401, 0);
    unresolved_error(
        SpellEffectDb2Store::load_effective_from_hotfix_rows_like_cpp(
            &data_dir,
            "enUS",
            [effect_overlay],
            &removals,
        ),
    );

    let visual_overlay = SpellXSpellVisualEntry {
        id: 900,
        difficulty_id: 0,
        spell_visual_id: 1,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id: 73_401,
    };
    unresolved_error(
        SpellXSpellVisualStore::load_effective_from_hotfix_rows_like_cpp(
            &data_dir,
            "enUS",
            [visual_overlay],
            &removals,
        ),
    );
}
