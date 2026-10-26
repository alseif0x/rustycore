use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, SpellInfoKeyContributorLikeCpp,
    SpellInfoKeyHotfixOverlayBatchLikeCpp, SpellInfoKeyHotfixOverlayRowLikeCpp,
    SpellInfoKeyHotfixOverlaysLikeCpp, SpellNameEntry, SpellStore,
    SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP,
    spell_db2::SpellNameStore,
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

/// Build exact regular-spell keys through the production canonical loader,
/// without inventing any `SpellInfo` payload.
pub(crate) fn canonical_spell_store(
    keys: impl IntoIterator<Item = (u32, u8)>,
) -> SpellStore {
    let keys: BTreeSet<_> = keys.into_iter().collect();
    let spell_ids: BTreeSet<_> = keys.iter().map(|(spell_id, _)| *spell_id).collect();
    let names = SpellNameStore::from_entries(spell_ids.into_iter().map(|spell_id| {
        SpellNameEntry {
            id: spell_id,
            name: format!("selected_charge fixture spell {spell_id}"),
        }
    }));
    let fixtures = CanonicalSpellFixtureDirectory::new();
    let contributor_batches = SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP
        .into_iter()
        .enumerate()
        .map(|(index, contributor)| {
            let rows = if contributor == SpellInfoKeyContributorLikeCpp::SpellMisc {
                keys.iter()
                    .enumerate()
                    .map(|(record_id, (spell_id, difficulty_id))| {
                        SpellInfoKeyHotfixOverlayRowLikeCpp {
                            record_id: u32::try_from(record_id + 1)
                                .expect("fixture record ID fits u32"),
                            spell_id: *spell_id,
                            difficulty_id: *difficulty_id,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            fixtures.write_complete_empty(
                contributor_file_name(contributor),
                0xA500_0001 + u32::try_from(index).expect("contributor index fits u32"),
            );
            SpellInfoKeyHotfixOverlayBatchLikeCpp { contributor, rows }
        })
        .collect();
    fixtures.write_complete_empty("SpellPowerDifficulty.db2", 0xA500_0015);

    SpellStore::load_spell_info_key_seed_from_hotfix_rows_like_cpp(
        &fixtures.data_dir(),
        "enUS",
        &names,
        &Db2HotfixRemovalStoreLikeCpp::default(),
        SpellInfoKeyHotfixOverlaysLikeCpp {
            contributor_batches,
            power_difficulty_rows: Vec::new(),
        },
    )
    .expect("load canonical SpellInfo key fixture")
}

fn contributor_file_name(contributor: SpellInfoKeyContributorLikeCpp) -> &'static str {
    match contributor {
        SpellInfoKeyContributorLikeCpp::SpellEffect => "SpellEffect.db2",
        SpellInfoKeyContributorLikeCpp::SpellAuraOptions => "SpellAuraOptions.db2",
        SpellInfoKeyContributorLikeCpp::SpellAuraRestrictions => "SpellAuraRestrictions.db2",
        SpellInfoKeyContributorLikeCpp::SpellCastingRequirements => {
            "SpellCastingRequirements.db2"
        }
        SpellInfoKeyContributorLikeCpp::SpellCategories => "SpellCategories.db2",
        SpellInfoKeyContributorLikeCpp::SpellClassOptions => "SpellClassOptions.db2",
        SpellInfoKeyContributorLikeCpp::SpellCooldowns => "SpellCooldowns.db2",
        SpellInfoKeyContributorLikeCpp::SpellEquippedItems => "SpellEquippedItems.db2",
        SpellInfoKeyContributorLikeCpp::SpellInterrupts => "SpellInterrupts.db2",
        SpellInfoKeyContributorLikeCpp::SpellLabel => "SpellLabel.db2",
        SpellInfoKeyContributorLikeCpp::SpellLevels => "SpellLevels.db2",
        SpellInfoKeyContributorLikeCpp::SpellMisc => "SpellMisc.db2",
        SpellInfoKeyContributorLikeCpp::SpellPower => "SpellPower.db2",
        SpellInfoKeyContributorLikeCpp::SpellReagents => "SpellReagents.db2",
        SpellInfoKeyContributorLikeCpp::SpellReagentsCurrency => "SpellReagentsCurrency.db2",
        SpellInfoKeyContributorLikeCpp::SpellScaling => "SpellScaling.db2",
        SpellInfoKeyContributorLikeCpp::SpellShapeshift => "SpellShapeshift.db2",
        SpellInfoKeyContributorLikeCpp::SpellTargetRestrictions => "SpellTargetRestrictions.db2",
        SpellInfoKeyContributorLikeCpp::SpellTotems => "SpellTotems.db2",
        SpellInfoKeyContributorLikeCpp::SpellXSpellVisual => "SpellXSpellVisual.db2",
    }
}

struct CanonicalSpellFixtureDirectory(PathBuf);

impl CanonicalSpellFixtureDirectory {
    fn new() -> Self {
        loop {
            let nonce = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rustycore-selected-charge-spell-keys-{}-{nonce}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => {
                    let fixture = Self(root);
                    fs::create_dir_all(fixture.0.join("data/dbc/enUS"))
                        .expect("create canonical spell fixture directory");
                    return fixture;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create canonical spell fixture directory: {error}"),
            }
        }
    }

    fn data_dir(&self) -> String {
        self.0
            .join("data")
            .to_str()
            .expect("temporary path is UTF-8")
            .to_owned()
    }

    fn write_complete_empty(&self, file_name: &str, table_hash: u32) {
        let mut bytes = vec![0; 72];
        bytes[0..4].copy_from_slice(&u32::from_le_bytes(*b"WDC4").to_le_bytes());
        bytes[20..24].copy_from_slice(&table_hash.to_le_bytes());
        fs::write(self.0.join("data/dbc/enUS").join(file_name), bytes)
            .expect("write complete empty WDC4 fixture");
    }
}

impl Drop for CanonicalSpellFixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn selected_charge_fixture_preserves_exact_canonical_spell_info_keys_without_payloads() {
    let store = canonical_spell_store([(75_123, 7), (0xF123_4567, 9)]);

    assert!(store.contains_spell_info_exact_like_cpp(75_123, 7));
    assert!(!store.contains_spell_info_exact_like_cpp(75_123, 0));
    assert!(store.contains_spell_info_exact_like_cpp(0xF123_4567, 9));
    assert!(store.get(75_123).is_none());
}
