use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
use wow_core::ObjectGuid;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP,
    SpellGroupRowLikeCpp, SpellGroupStackRuleLikeCpp, SpellGroupStackRuleStoreLikeCpp,
    SpellGroupStoreLikeCpp, SpellInfoKeyContributorLikeCpp, SpellInfoKeyHotfixOverlayBatchLikeCpp,
    SpellInfoKeyHotfixOverlayRowLikeCpp, SpellInfoKeyHotfixOverlaysLikeCpp,
    SpellNameEntry, SpellStore, SpellChainStoreLikeCpp, SpellRankEdgeLikeCpp,
    character_progression::ChrRacesStore, spell_db2::SpellNameStore,
};

const FAKE_INEBRIATE: i32 = 304;
const GROUP: u32 = 1001;
static NEXT_SPELL_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

fn spell(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![SpellEffectInfo {
            effect_index: 0,
            effect_aura: FAKE_INEBRIATE,
            ..Default::default()
        }],
    }
}

fn aura(slot: u8, spell_id: i32, caster: ObjectGuid, amount: Option<i32>) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id,
        difficulty_id: 0,
        caster_guid: caster,
        slot,
        duration_total: 1_000,
        duration_remaining: 1_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: amount.unwrap_or_default(),
        represented_effect_amounts: amount
            .map(|amount| {
                vec![wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 0,
                    amount,
                }]
            })
            .unwrap_or_default(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

fn keyed_spell_store(spell_ids: &[i32]) -> SpellStore {
    let keys = spell_ids
        .iter()
        .map(|spell_id| (*spell_id as u32, 0_u8));
    let mut store = canonical_spell_store(keys);
    for spell_id in spell_ids {
        store.insert(*spell_id, spell(*spell_id));
    }
    store
}

fn canonical_spell_store(keys: impl IntoIterator<Item = (u32, u8)>) -> SpellStore {
    let keys: BTreeSet<_> = keys.into_iter().collect();
    let spell_ids: BTreeSet<_> = keys.iter().map(|(spell_id, _)| *spell_id).collect();
    let names = SpellNameStore::from_entries(spell_ids.into_iter().map(|spell_id| SpellNameEntry {
        id: spell_id,
        name: format!("fake inebriation fixture spell {spell_id}"),
    }));
    let fixture = CanonicalSpellFixtureDirectory::new();
    let contributor_batches = SPELL_INFO_KEY_CONTRIBUTOR_ORDER_LIKE_CPP
        .into_iter()
        .enumerate()
        .map(|(index, contributor)| {
            let rows = if contributor == SpellInfoKeyContributorLikeCpp::SpellMisc {
                keys.iter()
                    .enumerate()
                    .map(|(record_index, (spell_id, difficulty_id))| {
                        SpellInfoKeyHotfixOverlayRowLikeCpp {
                            record_id: u32::try_from(record_index + 1)
                                .expect("fixture record ID fits u32"),
                            spell_id: *spell_id,
                            difficulty_id: *difficulty_id,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            fixture.write_complete_empty(
                &format!("{contributor:?}.db2"),
                0xA500_0001 + u32::try_from(index).expect("contributor index fits u32"),
            );
            SpellInfoKeyHotfixOverlayBatchLikeCpp { contributor, rows }
        })
        .collect();
    fixture.write_complete_empty("SpellPowerDifficulty.db2", 0xA500_0015);

    SpellStore::load_spell_info_key_seed_from_hotfix_rows_like_cpp(
        &fixture.data_dir(),
        "enUS",
        &names,
        &Db2HotfixRemovalStoreLikeCpp::default(),
        SpellInfoKeyHotfixOverlaysLikeCpp {
            contributor_batches,
            power_difficulty_rows: Vec::new(),
        },
    )
    .expect("load exact canonical SpellInfo key fixture")
}

struct CanonicalSpellFixtureDirectory(PathBuf);

impl CanonicalSpellFixtureDirectory {
    fn new() -> Self {
        loop {
            let nonce = NEXT_SPELL_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "rustycore-fake-inebriation-spell-keys-{}-{nonce}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => {
                    let fixture = Self(root);
                    fs::create_dir_all(fixture.0.join("data/dbc/enUS"))
                        .expect("create spell key fixture directory");
                    return fixture;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create spell key fixture directory: {error}"),
            }
        }
    }

    fn data_dir(&self) -> String {
        self.0
            .join("data")
            .to_str()
            .expect("fixture path is UTF-8")
            .to_owned()
    }

    fn write_complete_empty(&self, file_name: &str, table_hash: u32) {
        let mut bytes = vec![0; 72];
        bytes[0..4].copy_from_slice(b"WDC4");
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

fn rank_group_catalogs<'a>(
    spells: &'a SpellStore,
    races: &'a ChrRacesStore,
    groups: &'a SpellGroupStoreLikeCpp,
    chains: &'a SpellChainStoreLikeCpp,
    stack_rules: &'a SpellGroupStackRuleStoreLikeCpp,
) -> PlayerStatAuraCatalogs<'a> {
    PlayerStatAuraCatalogs::new(
        spells,
        None,
        Some(races),
        Some(groups),
        Some(chains),
        Some(stack_rules),
    )
}

#[test]
fn complete_empty_aura_view_proves_no_fake_inebriation_without_group_catalogs() {
    let spells = SpellStore::new();
    let races = ChrRacesStore::from_entries([]);
    let auras = HashMap::new();
    let view = PlayerStatAuraView::new(
        &auras,
        PlayerStatAuraCatalogs::new(&spells, None, Some(&races), None, None, None),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&[]), Some((0, false)));
}

#[test]
fn fake_inebriation_uses_first_rank_groups_and_native_registration_tie_order() {
    let spells = keyed_spell_store(&[101, 102]);
    let groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [SpellGroupRowLikeCpp {
            group_id: GROUP,
            spell_id: 101,
        }],
        |id| id == 101 || id == 102,
        |id| if id == 102 { 2 } else { 1 },
    )
    .store;
    let chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [SpellRankEdgeLikeCpp {
            spell_id: 102,
            supercedes_spell_id: 101,
        }],
        |id| id == 101 || id == 102,
    );
    let stack_rules = SpellGroupStackRuleStoreLikeCpp {
        stack_rule_by_group_id: BTreeMap::from([(
            GROUP,
            SpellGroupStackRuleLikeCpp::ExclusiveSameEffect,
        )]),
        same_effect_stack_by_group_id: BTreeMap::from([(
            GROUP,
            BTreeSet::from([FAKE_INEBRIATE]),
        )]),
    };
    let caster = ObjectGuid::create_player(1, 991);
    let auras = HashMap::from([
        (1, aura(1, 101, caster, Some(40))),
        (9, aura(9, 102, caster, Some(-40))),
    ]);
    let refs = [
        AppliedAuraRef::new(101, caster, 1, 1),
        AppliedAuraRef::new(102, caster, 9, 1),
    ];
    let races = ChrRacesStore::from_entries([]);
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &groups, &chains, &stack_rules),
    )
    .unwrap();

    assert_eq!(view.fake_inebriation_facts(&refs), Some((-40, true)));
}

#[test]
fn fake_inebriation_fails_closed_on_unknown_order_refs_amounts_or_sum_overflow() {
    let caster = ObjectGuid::create_player(1, 992);
    let spells = keyed_spell_store(&[101, 102]);
    let empty_groups = SpellGroupStoreLikeCpp::default();
    let chains = SpellChainStoreLikeCpp::default();
    let stack_rules = SpellGroupStackRuleStoreLikeCpp::default();
    let races = ChrRacesStore::from_entries([]);

    let auras = HashMap::from([(3, aura(3, 101, caster, None))]);
    let reference = AppliedAuraRef::new(101, caster, 3, 1);
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &empty_groups, &chains, &stack_rules),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&[reference]), None);
    assert_eq!(view.fake_inebriation_facts(&[]), None);

    let auras = HashMap::from([(3, aura(3, 101, caster, Some(5)))]);
    let view = PlayerStatAuraView::new(
        &auras,
        PlayerStatAuraCatalogs::new(&spells, None, Some(&races), None, None, None),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&[reference]), None);

    let auras = HashMap::from([
        (3, aura(3, 101, caster, Some(i32::MAX))),
        (4, aura(4, 102, caster, Some(1))),
    ]);
    let refs = [
        AppliedAuraRef::new(101, caster, 3, 1),
        AppliedAuraRef::new(102, caster, 4, 1),
    ];
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &empty_groups, &chains, &stack_rules),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&refs), None);
}

#[test]
fn fake_inebriation_checked_abs_only_compares_occupied_groups() {
    let caster = ObjectGuid::create_player(1, 993);
    let spells = keyed_spell_store(&[101, 102]);
    let groups = SpellGroupStoreLikeCpp {
        spell_entries_by_group_id: BTreeMap::from([(GROUP, vec![101, 102])]),
        group_ids_by_spell_id: BTreeMap::from([(101, vec![GROUP]), (102, vec![GROUP])]),
    };
    let chains = SpellChainStoreLikeCpp::default();
    let stack_rules = SpellGroupStackRuleStoreLikeCpp {
        stack_rule_by_group_id: BTreeMap::from([(
            GROUP,
            SpellGroupStackRuleLikeCpp::ExclusiveSameEffect,
        )]),
        same_effect_stack_by_group_id: BTreeMap::from([(
            GROUP,
            BTreeSet::from([FAKE_INEBRIATE]),
        )]),
    };
    let races = ChrRacesStore::from_entries([]);

    let auras = HashMap::from([(4, aura(4, 101, caster, Some(i32::MIN)))]);
    let refs = [AppliedAuraRef::new(101, caster, 4, 1)];
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &groups, &chains, &stack_rules),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&refs), Some((i32::MIN, true)));

    let auras = HashMap::from([
        (4, aura(4, 101, caster, Some(i32::MIN))),
        (8, aura(8, 102, caster, Some(1))),
    ]);
    let refs = [
        AppliedAuraRef::new(101, caster, 4, 1),
        AppliedAuraRef::new(102, caster, 8, 1),
    ];
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &groups, &chains, &stack_rules),
    )
    .unwrap();
    assert_eq!(view.fake_inebriation_facts(&refs), None);
}

#[test]
fn fake_inebriation_absent_same_effect_type_metadata_skips_that_loaded_group() {
    let caster = ObjectGuid::create_player(1, 994);
    let spells = keyed_spell_store(&[101, 102]);
    let groups = SpellGroupStoreLikeCpp {
        spell_entries_by_group_id: BTreeMap::from([(GROUP, vec![101, 102])]),
        group_ids_by_spell_id: BTreeMap::from([(101, vec![GROUP]), (102, vec![GROUP])]),
    };
    let chains = SpellChainStoreLikeCpp::default();
    let stack_rules = SpellGroupStackRuleStoreLikeCpp {
        stack_rule_by_group_id: BTreeMap::from([(
            GROUP,
            SpellGroupStackRuleLikeCpp::ExclusiveSameEffect,
        )]),
        same_effect_stack_by_group_id: BTreeMap::new(),
    };
    let auras = HashMap::from([
        (4, aura(4, 101, caster, Some(30))),
        (8, aura(8, 102, caster, Some(40))),
    ]);
    let refs = [
        AppliedAuraRef::new(101, caster, 4, 1),
        AppliedAuraRef::new(102, caster, 8, 1),
    ];
    let races = ChrRacesStore::from_entries([]);
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&spells, &races, &groups, &chains, &stack_rules),
    )
    .unwrap();

    assert_eq!(view.fake_inebriation_facts(&refs), Some((70, true)));
}

#[test]
fn fake_inebriation_genuine_nonzero_difficulty_key_needs_no_difficulty_zero_payload() {
    use wow_data::spell_db2::*;

    let effect = SpellEffectDb2Entry {
        id: 1,
        difficulty_id: 3,
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: FAKE_INEBRIATE as i16,
        effect_aura_period: 0,
        effect_base_points: 99,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: 303,
    };
    let store = canonical_spell_store([(303, 3)]).hydrate_effective_core_db2_like_cpp(
        wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
            SpellCategoriesStore::from_entries([]),
            SpellMiscStore::from_entries([]),
            SpellEffectDb2Store::from_entries([effect]),
            SpellShapeshiftStore::from_entries([]),
            SpellInterruptsStore::from_entries([]),
            SpellCastTimesStore::from_entries([]),
            SpellCooldownsStore::from_entries([]),
            SpellCastingRequirementsStore::from_entries([]),
            SpellPowerStore::from_entries([]),
            SpellPowerDifficultyStore::from_entries([]),
        ),
    );

    assert!(store.contains_spell_info_exact_like_cpp(303, 3));
    assert!(!store.contains_spell_info_exact_like_cpp(303, 0));
    assert!(store.get(303).is_none());
    assert!(store.effect_rows_for_exact_difficulty(303, 0).is_none());

    let caster = ObjectGuid::create_player(1, 995);
    let mut application = aura(3, 303, caster, Some(17));
    application.difficulty_id = 3;
    let auras = HashMap::from([(3, application)]);
    let reference = AppliedAuraRef::new(303, caster, 3, 1);
    let races = ChrRacesStore::from_entries([]);
    let groups = SpellGroupStoreLikeCpp::default();
    let chains = SpellChainStoreLikeCpp::default();
    let stack_rules = SpellGroupStackRuleStoreLikeCpp::default();
    let view = PlayerStatAuraView::new(
        &auras,
        rank_group_catalogs(&store, &races, &groups, &chains, &stack_rules),
    )
    .expect("the exact difficulty-three Aura uses its indexed effect row");

    assert_eq!(view.fake_inebriation_facts(&[reference]), Some((17, true)));
}
