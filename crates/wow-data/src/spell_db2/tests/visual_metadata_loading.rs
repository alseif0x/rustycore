use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

pub(super) struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    pub(super) fn new() -> Self {
        let nonce = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "rustycore-visual-loader-copy-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("data/dbc/enUS"))
            .expect("create visual DB2 fixture directory");
        Self(root)
    }

    pub(super) fn data_dir(&self) -> String {
        self.0
            .join("data")
            .to_str()
            .expect("temporary path is UTF-8")
            .to_string()
    }

    pub(super) fn write_incomplete_copy(&self, file_name: &str) {
        fs::write(
            self.0.join("data/dbc/enUS").join(file_name),
            unresolved_copy_wdc4(),
        )
        .expect("write unresolved-copy WDC4 fixture");
    }

    pub(super) fn write_invalid_file(&self, file_name: &str) {
        fs::write(
            self.0.join("data/dbc/enUS").join(file_name),
            b"invalid WDC4 fixture",
        )
        .expect("write invalid WDC4 fixture");
    }

    pub(super) fn write_complete_empty(&self, file_name: &str, table_hash: u32) {
        const HEADER_SIZE: usize = 72;
        let mut bytes = vec![0; HEADER_SIZE];
        put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
        put_u32(&mut bytes, 20, table_hash);
        fs::write(self.0.join("data/dbc/enUS").join(file_name), bytes)
            .expect("write complete empty WDC4 fixture");
    }

    pub(super) fn write_complete_records(
        &self,
        file_name: &str,
        table_hash: u32,
        first_record_id: u32,
        record_count: u32,
        field_widths: &[u16],
    ) {
        const HEADER_SIZE: usize = 72;
        const SECTION_HEADER_SIZE: usize = 40;
        const FIELD_META_SIZE: usize = 4;
        const FIELD_STORAGE_INFO_SIZE: usize = 24;

        assert!(record_count > 0, "record fixture must contain a row");
        let field_count = field_widths.len();
        let field_storage_offset =
            HEADER_SIZE + SECTION_HEADER_SIZE + field_count * FIELD_META_SIZE;
        let records_offset = field_storage_offset + field_count * FIELD_STORAGE_INFO_SIZE;
        let record_bits = field_widths
            .iter()
            .map(|width| usize::from(*width))
            .sum::<usize>();
        let record_size = record_bits.div_ceil(8);
        let record_count = usize::try_from(record_count).expect("fixture count fits usize");
        let mut bytes = vec![0; records_offset + record_size * record_count];

        put_u32(&mut bytes, 0, u32::from_le_bytes(*b"WDC4"));
        put_u32(&mut bytes, 4, record_count as u32);
        put_u32(&mut bytes, 8, field_count as u32);
        put_u32(&mut bytes, 12, record_size as u32);
        put_u32(&mut bytes, 20, table_hash);
        put_u32(&mut bytes, 28, first_record_id);
        put_u32(&mut bytes, 32, first_record_id + record_count as u32 - 1);
        put_u16(&mut bytes, 42, u16::MAX);
        put_u32(&mut bytes, 44, field_count as u32);
        put_u32(
            &mut bytes,
            56,
            (field_count * FIELD_STORAGE_INFO_SIZE) as u32,
        );
        put_u32(&mut bytes, 68, 1);

        let section_offset = HEADER_SIZE;
        put_u32(&mut bytes, section_offset + 8, records_offset as u32);
        put_u32(&mut bytes, section_offset + 12, record_count as u32);

        let mut field_offset = 0usize;
        for (field, width) in field_widths.iter().enumerate() {
            let info_offset = field_storage_offset + field * FIELD_STORAGE_INFO_SIZE;
            put_u16(&mut bytes, info_offset, field_offset as u16);
            put_u16(&mut bytes, info_offset + 2, *width);
            put_u32(&mut bytes, info_offset + 8, 0);
            field_offset += usize::from(*width);
        }

        fs::write(self.0.join("data/dbc/enUS").join(file_name), bytes)
            .expect("write fixed-record WDC4 fixture");
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

pub(super) fn unresolved_error(result: anyhow::Result<impl Sized>) -> String {
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
