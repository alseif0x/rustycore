use super::*;
use std::cell::RefCell;

struct TracedAdmission<'a> {
    fixture: &'a AdmissionFixture,
    reads: RefCell<Vec<&'static str>>,
    missing: Option<&'static str>,
}

impl<'a> TracedAdmission<'a> {
    fn new(fixture: &'a AdmissionFixture, missing: Option<&'static str>) -> Self {
        Self { fixture, reads: RefCell::new(Vec::new()), missing }
    }

    fn record(&self, read: &'static str) -> bool {
        self.reads.borrow_mut().push(read);
        self.missing != Some(read)
    }
}

impl AppearanceAdmissionSource for TracedAdmission<'_> {
    fn modified_appearance(&self, id: u32) -> Option<AppearanceModifiedFacts> {
        if !self.record("modified") {
            return None;
        }
        self.fixture.modified_appearance(id)
    }

    fn search_name(&self, id: u32) -> Option<AppearanceSearchFacts> {
        if !self.record("search") {
            return None;
        }
        self.fixture.search_name(id)
    }

    fn item_subclass(&self, id: u32) -> Option<u8> {
        if !self.record("item") {
            return None;
        }
        self.fixture.item_subclass(id)
    }

    fn sparse_template(&self, id: u32) -> Option<AppearanceSparseFacts> {
        if !self.record("sparse") {
            return None;
        }
        self.fixture.sparse_template(id)
    }

    fn storage_template(&self, id: u32) -> Option<AppearanceStorageFacts> {
        if !self.record("storage") {
            return None;
        }
        self.fixture.storage_template(id)
    }

    fn has_player_guid(&self) -> bool {
        self.record("guid");
        self.fixture.has_player_guid()
    }

    fn player_race(&self) -> u8 {
        self.record("race");
        self.fixture.player_race()
    }

    fn team_for_race(&self, race: u8) -> u32 {
        self.record("team");
        self.fixture.team_for_race(race)
    }

    fn player_level(&self) -> u8 {
        self.record("level");
        self.fixture.player_level()
    }

    fn player_skill(&self, skill: u16) -> Option<u16> {
        if !self.record("skill") {
            return None;
        }
        self.fixture.player_skill(skill)
    }

    fn knows_spell(&self, id: i32) -> bool {
        self.record("known");
        self.fixture.knows_spell(id)
    }

    fn reputation_rank(&self, id: u32) -> Option<u32> {
        if !self.record("reputation") {
            return None;
        }
        self.fixture.reputation_rank(id)
    }

    fn learning_effects(&self, id: u32) -> Vec<(u8, i32)> {
        self.record("effects");
        self.fixture.learning_effects(id)
    }

    fn player_class(&self) -> u8 {
        self.record("class");
        self.fixture.player_class()
    }

    fn class_mask(&self, id: u8) -> u32 {
        self.record("class_mask");
        self.fixture.class_mask(id)
    }

    fn item_quality(&self, id: u32) -> Option<i8> {
        if !self.record("quality") {
            return None;
        }
        self.fixture.item_quality(id)
    }

    fn weapon_proficiency(&self) -> Option<u32> {
        if !self.record("weapon") {
            return None;
        }
        self.fixture.weapon_proficiency()
    }

    fn armor_class_mask(&self, id: u32) -> u32 {
        self.record("armor");
        self.fixture.armor_class_mask(id)
    }

    fn permanent_appearance_exists(&self, id: u32) -> Option<bool> {
        if !self.record("permanent") {
            return None;
        }
        self.fixture.permanent_appearance_exists(id)
    }

}

fn admission_fixture() -> AdmissionFixture {
    let mut fixture = AdmissionFixture::default();
    fixture.install_player(
        ObjectGuid::create_player(1, 90), "AdmissionBoundary".into(),
        Position::ZERO, 571, 1, 1, 80, 0,
    );
    fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
        ItemModifiedAppearanceEntry {
            id: 65, item_id: 777, item_appearance_modifier_id: 0,
            item_appearance_id: 9_000, order_index: 0, transmog_source_type_enum: 0,
        },
    ])));
    install_appearance_test_item(
        &mut fixture, 777, ItemClass::Weapon, ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon, ItemQuality::Uncommon, [0; 4], 0,
    );
    fixture.player.add_weapon_proficiency_like_cpp(1 << ItemSubClassWeapon::Sword as u32);
    fixture
}

#[test]
fn appearance_admission_missing_rows_stop_before_player_queries() {
    let fixture = admission_fixture();
    let catalog_reads = ["modified", "search", "item", "sparse", "storage"];
    for (index, missing) in catalog_reads.iter().copied().enumerate() {
        let source = TracedAdmission::new(&fixture, Some(missing));
        assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(&source, 65));
        assert_eq!(*source.reads.borrow(), catalog_reads[..=index]);
    }
}

#[test]
fn appearance_admission_rejects_sources_before_catalog_and_player_reads() {
    let mut fixture = admission_fixture();
    for source_type in [6, 9] {
        fixture.modified = Some(Arc::new(ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 65, item_id: 777, item_appearance_modifier_id: 0,
                item_appearance_id: 9_000, order_index: 0,
                transmog_source_type_enum: source_type,
            },
        ])));
        let source = TracedAdmission::new(&fixture, None);
        assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(&source, 65));
        assert_eq!(*source.reads.borrow(), ["modified"]);
    }
}

#[test]
fn appearance_admission_resolves_permanent_owner_after_quality_and_proficiency() {
    let mut fixture = admission_fixture();
    fixture.player.gameplay_state_mut().collections.add_temporary_item_appearance_like_cpp(
        65, ObjectGuid::create_item(1, 900),
    );
    let source = TracedAdmission::new(&fixture, None);
    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(&source, 65));
    let reads = source.reads.borrow();
    assert_eq!(reads.last(), Some(&"permanent"));
    assert!(reads.iter().position(|read| *read == "quality").unwrap()
        < reads.iter().position(|read| *read == "weapon").unwrap());
    drop(reads);
    let missing_owner = TracedAdmission::new(&fixture, Some("permanent"));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(&missing_owner, 65));
}

#[test]
fn appearance_admission_requires_both_low_quality_flags() {
    let mut fixture = admission_fixture();
    for (flags, allowed) in [
        ([0, ItemFlags2::IgnoreQualityForItemVisualSource as u32, 0, 0], false),
        ([0, 0, ItemFlags3::ActsAsTransmogHiddenVisualOption as u32, 0], false),
        ([0, ItemFlags2::IgnoreQualityForItemVisualSource as u32,
            ItemFlags3::ActsAsTransmogHiddenVisualOption as u32, 0], true),
    ] {
        install_appearance_test_item(
            &mut fixture, 777, ItemClass::Armor, ItemSubClassArmor::Miscellaneous as u8,
            InventoryType::Cloak, ItemQuality::Normal, flags, 0,
        );
        assert_eq!(PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65), allowed);
    }
}

#[test]
fn appearance_favorites_preserve_new_removed_and_unchanged_transitions() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    assert!(collections.set_appearance_favorite(65, true));
    assert!(!collections.set_appearance_favorite(65, true));
    assert!(collections.set_appearance_favorite(65, false));
    assert!(!collections.favorite_item_appearances_like_cpp().contains_key(&65));
    collections.favorite_item_appearance_entry_like_cpp(65)
        .or_insert(FavoriteAppearanceStateLikeCpp::Unchanged);
    assert!(collections.set_appearance_favorite(65, false));
    assert!(collections.set_appearance_favorite(65, false));
    assert_eq!(collections.favorite_item_appearances_like_cpp().get(&65),
        Some(&FavoriteAppearanceStateLikeCpp::Removed));
    assert!(collections.set_appearance_favorite(65, true));
    assert_eq!(collections.favorite_item_appearances_like_cpp().get(&65),
        Some(&FavoriteAppearanceStateLikeCpp::Unchanged));
    assert!(!collections.set_appearance_favorite(99, false));
}

#[test]
fn appearance_loading_ignores_duplicate_zero_and_honors_canonical_empty() {
    let (blocks, ids, dense, highest) =
        PlayerCollectionStateLikeCpp::prepare_appearance_blocks([(2, 1), (2, 0), (0, 2)]);
    assert_eq!(blocks, BTreeMap::from([(0, 2), (2, 1)]));
    assert_eq!(ids, HashSet::from([1, 64]));
    assert_eq!(dense, vec![2, 0, 1]);
    assert_eq!(highest, Some(2));
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.item_appearances.insert(65);
    assert!(collections.active_appearance_blocks(|| Some(Vec::new())).is_empty());
    assert_eq!(collections.active_appearance_blocks(|| None), vec![0, 0, 2]);
    collections.item_appearance_blocks = vec![1];
    assert_eq!(collections.active_appearance_blocks(|| panic!("collection blocks win")), vec![1]);
}

#[test]
fn appearance_runtime_route_keeps_resolution_admission_and_temporary_queries_lazy() {
    let reads = RefCell::new(Vec::new());
    assert_eq!(PlayerCollectionStateLikeCpp::runtime_appearance_route(
        false, || panic!("not soulbound"), |_| panic!("not soulbound"), || panic!("not soulbound"),
    ), None);
    assert_eq!(PlayerCollectionStateLikeCpp::runtime_appearance_route(
        true,
        || { reads.borrow_mut().push("resolve"); Some(65) },
        |_| { reads.borrow_mut().push("admit"); false },
        || panic!("rejected admission"),
    ), None);
    assert_eq!(*reads.borrow(), ["resolve", "admit"]);
    reads.borrow_mut().clear();
    assert_eq!(PlayerCollectionStateLikeCpp::runtime_appearance_route(
        true,
        || { reads.borrow_mut().push("resolve"); Some(65) },
        |_| { reads.borrow_mut().push("admit"); true },
        || { reads.borrow_mut().push("temporary"); true },
    ), Some(RuntimeAppearanceRoute::Temporary(65)));
    assert_eq!(*reads.borrow(), ["resolve", "admit", "temporary"]);
}

#[test]
fn appearance_set_completion_retains_missing_negative_and_first_completed_slot_behavior() {
    let reads = RefCell::new(Vec::new());
    assert!(PlayerCollectionStateLikeCpp::transmog_set_complete(
        Some([1, 2]),
        |id| if id == 1 { None } else { Some(-1) },
        |_| panic!("no valid item"),
        |_| panic!("no valid slot"),
    ));
    assert!(PlayerCollectionStateLikeCpp::transmog_set_complete(
        Some([65, 96, 65]),
        |id| Some(id as i32),
        |_| Some(InventoryType::Weapon),
        |id| { reads.borrow_mut().push(id); (true, false) },
    ));
    assert_eq!(*reads.borrow(), [65]);
    assert!(!PlayerCollectionStateLikeCpp::transmog_set_complete(
        Some([65]),
        |_| Some(777),
        |_| Some(InventoryType::Head),
        |_| (true, true),
    ));
}

#[test]
fn appearance_duplicate_player_flag_keeps_partial_conditional_field_write() {
    let mut player = Player::new(Some(1), false);
    player.add_transmog_block_like_cpp(1 << 1);
    player.add_conditional_transmog_like_cpp(1);
    player.gameplay_state_mut().collections.add_temporary_item_appearance_like_cpp(
        1, ObjectGuid::create_item(1, 900),
    );
    player.clear_data_changes();
    assert!(PlayerCollectionStateLikeCpp::apply_permanent_appearance_fields(
        &mut player, 1, true, 0, 1 << 1,
    ).is_none());
    assert!(player.conditional_transmog_like_cpp().is_empty());
    assert!(player.gameplay_state().collections.has_temporary_item_appearance_like_cpp(1));
}

#[test]
fn appearance_admission_skill_query_is_lazy_and_uses_actual_player_skill_records() {
    let mut fixture = admission_fixture();
    let row = fixture.search.as_ref().unwrap().get(777).unwrap().clone();
    fixture.search = Some(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            required_skill: 333,
            required_skill_rank: 1,
            ..row
        },
    ])));
    let source = TracedAdmission::new(&fixture, Some("skill"));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(&source, 65));
    assert_eq!(source.reads.borrow().last(), Some(&"skill"));
    assert!(!PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65));
    fixture.player.replace_skill_records_like_cpp(
        vec![crate::PlayerSkillRecord {
            skill_line_id: 333,
            current_value: 1,
            max_value: 1,
            step: 0,
            profession_slot: -1,
            state: crate::PlayerSkillLoadState::Unchanged,
        }],
        true,
        true,
        Some(1),
        std::collections::BTreeSet::new(),
    );
    assert!(PlayerCollectionStateLikeCpp::can_add_appearance(&fixture, 65));
}
