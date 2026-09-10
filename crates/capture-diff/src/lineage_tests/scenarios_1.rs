//! Capture-lineage regressions, part 1 of 2.
//!
//! Moved out of the lineage_tests.rs root under #683; every test is unchanged.

use super::*;

#[test]
fn raw_manifest_or_artifact_tamper_is_rejected() {
    let root = test_root("raw-tamper");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

    fs::write(&cpp, b"tampered").unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("raw artifact tamper must fail");
    assert!(error.to_string().contains("size") || error.to_string().contains("SHA-256"));
    fs::write(&cpp, b"raw-cpp").unwrap();

    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
    json["artifact"]["tree_sha256"] = serde_json::Value::String("d".repeat(64));
    fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("raw manifest hash tamper must fail");
    assert!(error.to_string().contains("tree SHA-256"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn raw_manifest_requires_complete_consistent_process_provenance() {
    let root = test_root("raw-provenance");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let original = fs::read(&cpp_manifest).unwrap();

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json.as_object_mut().unwrap().remove("source_repo_head");
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing source HEAD must fail schema validation");
    assert!(format!("{error:#}").contains("parsing raw manifest"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["live_exec_sha256"] = serde_json::Value::String("9".repeat(64));
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("source/live executable mismatch must fail");
    assert!(format!("{error:#}").contains("expected/source/live executable SHA-256"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["harness_worktree_clean"] = serde_json::Value::Bool(false);
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("dirty capture harness must fail");
    assert!(format!("{error:#}").contains("harness worktree must be clean"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json.as_object_mut().unwrap().remove("pm2_exec_sha256");
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing PM2 entrypoint hash must fail schema validation");
    assert!(format!("{error:#}").contains("parsing raw manifest"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["created_at"] = serde_json::Value::String("yesterday".to_string());
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("non-RFC3339 timestamp must fail");
    assert!(format!("{error:#}").contains("RFC3339"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["source_derivation"] = reviewed_cpp_source_derivation();
    fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("an unrelated flow must not claim reviewed C++ derivation evidence");
    assert!(format!("{error:#}").contains("reserved for C++ creature-spell-casting"));

    fs::write(&cpp_manifest, &original).unwrap();
    let mut rust_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
    rust_json["source_worktree_dirty"] = serde_json::Value::Bool(true);
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&rust_json).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("dirty Rust source worktree must fail");
    assert!(format!("{error:#}").contains("same clean state"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nested_raw_manifest_is_rejected_instead_of_excluded_from_tree_hash() {
    let root = test_root("nested-raw-manifest");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let nested = rust.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join(RUST_RAW_MANIFEST_FILE), b"{}").unwrap();

    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("nested raw manifest must fail rather than disappear from hashing");
    assert!(format!("{error:#}").contains("unexpected or nested"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_raw_or_derived_manifest_is_rejected() {
    let root = test_root("missing-manifest");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

    fs::remove_file(&cpp_manifest).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing C++ raw manifest must fail");
    assert!(error.to_string().contains("C++ raw manifest"));

    let flow_dir = make_derived_flow(&root, flow, &raw);
    fs::remove_file(flow_dir.join(LINEAGE_FILE)).unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("missing derived lineage must fail");
    assert!(error.to_string().contains("reading required lineage"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verify_rejects_retained_manifest_and_output_tamper() {
    let root = test_root("derived-tamper");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
    let flow_dir = make_derived_flow(&root, flow, &raw);
    verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();

    fs::write(flow_dir.join("rust/one.bin"), b"tampered-output").unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("derived output tamper must fail");
    assert!(error.to_string().contains("tree SHA-256"));
    fs::write(flow_dir.join("rust/one.bin"), b"derived-bin").unwrap();

    let retained = flow_dir
        .join(RAW_PROVENANCE_DIR)
        .join(CPP_RAW_MANIFEST_FILE);
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&retained).unwrap()).unwrap();
    json["artifact"]["sha256"] = serde_json::Value::String("e".repeat(64));
    fs::write(&retained, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("retained raw manifest tamper must fail");
    assert!(error.to_string().contains("raw manifest SHA-256"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verify_rejects_lineage_schema_and_hash_tamper() {
    let root = test_root("lineage-tamper");
    let flow = "required-flow";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
    let flow_dir = make_derived_flow(&root, flow, &raw);

    let path = flow_dir.join(LINEAGE_FILE);
    let original = fs::read(&path).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["outputs"]["cpp_pkt"]["sha256"] = serde_json::Value::String("f".repeat(64));
    fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("lineage output hash tamper must fail");
    assert!(error.to_string().contains("cpp.pkt"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["selection"]["ignored_opcodes"] = serde_json::json!([{
        "direction": "s2c",
        "opcode": 11732
    }]);
    fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("an extra derived-flow filter must fail the reviewed contract");
    assert!(error.to_string().contains("reviewed import contract"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["unexpected"] = serde_json::Value::Bool(true);
    fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("unknown lineage fields must fail schema validation");
    assert!(error.to_string().contains("parsing required lineage"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_import_before_exchange_leaves_old_flow_untouched() {
    let root = test_root("interrupted-import");
    let target = root.join("required-flow");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("README.md"), b"reviewed metadata").unwrap();
    fs::write(target.join("cpp.pkt"), b"old-complete-flow").unwrap();

    let staging_path;
    {
        let transaction = AtomicFlowImport::prepare(&root, "required-flow").unwrap();
        staging_path = transaction.staging_dir().to_path_buf();
        fs::write(
            transaction.staging_dir().join("cpp.pkt"),
            b"new-partial-flow",
        )
        .unwrap();
        // A signal/error before publish drops the transaction here.
    }

    assert_eq!(
        fs::read(target.join("cpp.pkt")).unwrap(),
        b"old-complete-flow"
    );
    assert_eq!(
        fs::read(target.join("README.md")).unwrap(),
        b"reviewed metadata"
    );
    assert!(!staging_path.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_rejects_unknown_existing_metadata_instead_of_carrying_it_forward() {
    let root = test_root("unknown-metadata");
    let target = root.join("required-flow");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("README.md"), b"reviewed").unwrap();
    fs::write(target.join("unreviewed.secret"), b"must not propagate").unwrap();

    let error = AtomicFlowImport::prepare(&root, "required-flow")
        .err()
        .expect("unknown metadata must fail closed");
    assert!(error.to_string().contains("unknown non-generated entry"));
    assert_eq!(
        fs::read(target.join("unreviewed.secret")).unwrap(),
        b"must not propagate"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detour_import_copies_only_the_reviewed_fixture_tree() {
    let root = test_root("detour-reviewed-fixture");
    let target = root.join("detour-chase-around-obstacle");
    let fixture = target.join("fixture");
    let mmaps = fixture.join("mmaps");
    let committed_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/detour-chase-around-obstacle/fixture");
    fs::create_dir_all(&mmaps).unwrap();
    fs::write(target.join("README.md"), b"reviewed").unwrap();
    fs::copy(
        committed_fixture.join("fixture.json"),
        fixture.join("fixture.json"),
    )
    .unwrap();
    fs::copy(
        committed_fixture.join("mmaps/0001.mmap"),
        mmaps.join("0001.mmap"),
    )
    .unwrap();
    fs::copy(
        committed_fixture.join("mmaps/00015026.mmtile"),
        mmaps.join("00015026.mmtile"),
    )
    .unwrap();

    {
        let transaction = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle").unwrap();
        assert_eq!(
            fs::read(transaction.staging_dir().join("fixture/fixture.json")).unwrap(),
            fs::read(committed_fixture.join("fixture.json")).unwrap()
        );
        assert_eq!(
            fs::read(
                transaction
                    .staging_dir()
                    .join("fixture/mmaps/00015026.mmtile")
            )
            .unwrap(),
            fs::read(committed_fixture.join("mmaps/00015026.mmtile")).unwrap()
        );
    }

    fs::write(mmaps.join("0001.mmap"), b"tampered").unwrap();
    let error = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle")
        .err()
        .expect("tampered reviewed asset must fail closed");
    assert!(error.to_string().contains("map header differs"));
    fs::copy(
        committed_fixture.join("mmaps/0001.mmap"),
        mmaps.join("0001.mmap"),
    )
    .unwrap();
    fs::write(fixture.join("unreviewed.bin"), b"must not propagate").unwrap();
    let error = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle")
        .err()
        .expect("unknown fixture entry must fail closed");
    assert!(error.to_string().contains("unreviewed root entry"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creature_spell_import_copies_only_the_reviewed_fixture_derivation() {
    let root = test_root("creature-spell-reviewed-fixture");
    let target = root.join("creature-spell-casting");
    let fixture = target.join("fixture");
    let committed_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/creature-spell-casting/fixture");
    fs::create_dir_all(&fixture).unwrap();
    fs::write(target.join("README.md"), b"reviewed").unwrap();
    for name in ["fixture.json", "cpp-reference.patch"] {
        fs::copy(committed_fixture.join(name), fixture.join(name)).unwrap();
    }

    {
        let transaction = AtomicFlowImport::prepare(&root, "creature-spell-casting").unwrap();
        assert_eq!(
            fs::read(transaction.staging_dir().join("fixture/fixture.json")).unwrap(),
            fs::read(committed_fixture.join("fixture.json")).unwrap()
        );
        assert_eq!(
            fs::read(
                transaction
                    .staging_dir()
                    .join("fixture/cpp-reference.patch")
            )
            .unwrap(),
            fs::read(committed_fixture.join("cpp-reference.patch")).unwrap()
        );
    }

    fs::write(fixture.join("fixture.json"), b"tampered").unwrap();
    let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
        .err()
        .expect("tampered reviewed fixture manifest must fail closed");
    assert!(error.to_string().contains("manifest differs"));
    fs::copy(
        committed_fixture.join("fixture.json"),
        fixture.join("fixture.json"),
    )
    .unwrap();
    fs::write(fixture.join("cpp-reference.patch"), b"tampered").unwrap();
    let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
        .err()
        .expect("tampered reviewed source patch must fail closed");
    assert!(error.to_string().contains("reference patch differs"));
    fs::copy(
        committed_fixture.join("cpp-reference.patch"),
        fixture.join("cpp-reference.patch"),
    )
    .unwrap();
    fs::write(fixture.join("unreviewed.bin"), b"must not propagate").unwrap();
    let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
        .err()
        .expect("unknown creature-spell fixture entry must fail closed");
    assert!(error.to_string().contains("unreviewed root entry"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn complete_existing_flow_is_exchanged_as_one_generation() {
    let root = test_root("atomic-exchange");
    let target = root.join("required-flow");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("README.md"), b"reviewed metadata").unwrap();
    fs::write(target.join("cpp.pkt"), b"old").unwrap();

    let transaction = AtomicFlowImport::prepare(&root, "required-flow").unwrap();
    fs::write(transaction.staging_dir().join("cpp.pkt"), b"new").unwrap();
    fs::write(transaction.staging_dir().join(LINEAGE_FILE), b"complete").unwrap();
    transaction.publish().unwrap();

    assert_eq!(fs::read(target.join("cpp.pkt")).unwrap(), b"new");
    assert_eq!(fs::read(target.join(LINEAGE_FILE)).unwrap(), b"complete");
    assert_eq!(
        fs::read(target.join("README.md")).unwrap(),
        b"reviewed metadata"
    );
    assert!(fs::read_dir(&root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("partial")
    }));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn loot_raw_pair_requires_canonical_guard_bot_report_and_cross_side_identity() {
    let root = test_root("loot-identity");
    let flow = "loot-single-item-claim";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

    let original = fs::read(&rust_manifest).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["fixture_guard"]["enabled"] = serde_json::Value::Bool(false);
    fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("disabled fixture guard must fail");
    assert!(format!("{error:#}").contains("fixture_guard.enabled"));

    let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
    json["bot_report"]["exec_sha256"] = serde_json::Value::String("8".repeat(64));
    fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("different bot binary identity must fail");
    assert!(format!("{error:#}").contains("different canonical bot identities"));

    fs::write(&rust_manifest, &original).unwrap();
    let report_path = serde_json::from_slice::<serde_json::Value>(&original).unwrap()["bot_report"]
        ["report_path"]
        .as_str()
        .unwrap()
        .to_string();
    fs::write(&report_path, b"{}").unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("bot report tamper must fail");
    assert!(format!("{error:#}").contains("bot report SHA-256"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn loot_race_raw_pair_accepts_gameobject_guard_and_rejects_split_runtime_target() {
    let root = test_root("loot-race-identity");
    let flow = "loot-two-session-atomic-race";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
    let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
    let mut report: serde_json::Value =
        serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    report["results"][1]["loot_race_target_runtime_counter"] = serde_json::Value::from(41);
    let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
    fs::write(&report_path, &report_bytes).unwrap();
    manifest["bot_report"]["report_sha256"] =
        serde_json::Value::String(sha256_bytes(&report_bytes));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("split live target counter must fail");
    assert!(
        format!("{error:#}").contains("one shared target/list"),
        "unexpected error: {error:#}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creature_spell_raw_pair_requires_reviewed_guard_and_shared_database_snapshot() {
    let root = test_root("creature-spell-identity");
    let flow = "creature-spell-casting";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
    let flow_dir = make_derived_flow(&root, flow, &raw);
    verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();
    assert!(
        flow_dir
            .join(RAW_PROVENANCE_DIR)
            .join(CPP_BOT_REPORT_FILE)
            .is_file()
            && flow_dir
                .join(RAW_PROVENANCE_DIR)
                .join(RUST_BOT_REPORT_FILE)
                .is_file(),
        "derived creature-spell lineage did not retain both bot reports"
    );
    let lineage_path = flow_dir.join(LINEAGE_FILE);
    let lineage_original = fs::read(&lineage_path).unwrap();
    let lineage_json: serde_json::Value = serde_json::from_slice(&lineage_original).unwrap();
    assert_eq!(
        lineage_json["sources"]["cpp"]["source_derivation"],
        reviewed_cpp_source_derivation(),
        "derived lineage did not retain the exact reviewed C++ derivation"
    );
    assert!(
        lineage_json["sources"]["rust"]
            .get("source_derivation")
            .is_none(),
        "derived lineage invented C++ derivation evidence for Rust"
    );

    let mut tampered_lineage = lineage_json.clone();
    tampered_lineage["sources"]["cpp"]["source_derivation"]["patch_sha256"] =
        serde_json::Value::String("0".repeat(64));
    fs::write(
        &lineage_path,
        serde_json::to_vec_pretty(&tampered_lineage).unwrap(),
    )
    .unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("tampered derived source patch identity must fail");
    assert!(format!("{error:#}").contains("reviewed patch"));

    let mut missing_lineage = lineage_json;
    missing_lineage["sources"]["cpp"]
        .as_object_mut()
        .unwrap()
        .remove("source_derivation");
    fs::write(
        &lineage_path,
        serde_json::to_vec_pretty(&missing_lineage).unwrap(),
    )
    .unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("missing derived source derivation must fail");
    assert!(format!("{error:#}").contains("source_derivation is missing"));
    fs::write(&lineage_path, &lineage_original).unwrap();

    let cpp_original = fs::read(&cpp_manifest).unwrap();
    let mut missing_derivation: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    missing_derivation
        .as_object_mut()
        .unwrap()
        .remove("source_derivation");
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&missing_derivation).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing C++ source derivation must fail");
    assert!(format!("{error:#}").contains("requires source_derivation evidence"));

    for (field, value) in [
        (
            "contract",
            serde_json::Value::String("wrong-contract".into()),
        ),
        (
            "remote_url",
            serde_json::Value::String("https://example.invalid/reference.git".into()),
        ),
        (
            "remote_ref",
            serde_json::Value::String("refs/remotes/origin/other".into()),
        ),
        ("base_head", serde_json::Value::String("0".repeat(40))),
        ("base_tree", serde_json::Value::String("0".repeat(40))),
        ("patched_head", serde_json::Value::String("0".repeat(40))),
        ("patched_tree", serde_json::Value::String("0".repeat(40))),
        (
            "patch_path",
            serde_json::Value::String("fixture/other.patch".into()),
        ),
        ("patch_sha256", serde_json::Value::String("0".repeat(64))),
        (
            "changed_paths",
            serde_json::json!([
                CREATURE_SPELL_CPP_CHANGED_PATH,
                "src/server/game/DataStores/Unreviewed.cpp"
            ]),
        ),
    ] {
        let mut changed: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
        changed["source_derivation"][field] = value;
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&changed).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("mutated reviewed C++ source derivation must fail");
        assert!(
            format!("{error:#}").contains("source_derivation"),
            "unexpected {field} mutation error: {error:#}"
        );
    }

    let rust_original_with_derivation = fs::read(&rust_manifest).unwrap();
    let mut rust_derivation: serde_json::Value =
        serde_json::from_slice(&rust_original_with_derivation).unwrap();
    rust_derivation["source_derivation"] = reviewed_cpp_source_derivation();
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&rust_derivation).unwrap(),
    )
    .unwrap();
    fs::write(&cpp_manifest, &cpp_original).unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("Rust must not claim the C++ source derivation");
    assert!(format!("{error:#}").contains("Rust raw manifests must not contain"));
    fs::write(&rust_manifest, rust_original_with_derivation).unwrap();

    let mut dirty_cpp: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    dirty_cpp["source_worktree_dirty"] = serde_json::Value::Bool(true);
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&dirty_cpp).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("dirty creature-spell C++ source worktree must fail");
    assert!(
        format!("{error:#}").contains("creature-spell-casting C++ source worktree must be clean"),
        "unexpected error: {error:#}"
    );

    let mut wrong_revision: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    wrong_revision["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&wrong_revision).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("unrelated creature-spell C++ executable revision must fail");
    assert!(
        format!("{error:#}").contains(
            "creature-spell-casting C++ embedded executable revision must equal source_repo_head"
        ),
        "unexpected error: {error:#}"
    );
    fs::write(&cpp_manifest, &cpp_original).unwrap();

    let original = fs::read(&rust_manifest).unwrap();

    let mut missing_revision: serde_json::Value = serde_json::from_slice(&original).unwrap();
    missing_revision["source_exec_revision"] = serde_json::Value::Null;
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&missing_revision).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing creature-spell Rust executable revision must fail");
    assert!(
        format!("{error:#}").contains(
            "creature-spell-casting Rust embedded executable revision must equal source_repo_head"
        ),
        "unexpected error: {error:#}"
    );

    let mut wrong_revision: serde_json::Value = serde_json::from_slice(&original).unwrap();
    wrong_revision["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&wrong_revision).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("unrelated creature-spell Rust executable revision must fail");
    assert!(
        format!("{error:#}").contains(
            "creature-spell-casting Rust embedded executable revision must equal source_repo_head"
        ),
        "unexpected error: {error:#}"
    );
    fs::write(&rust_manifest, &original).unwrap();

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest.as_object_mut().unwrap().remove("fixture_guard");
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("missing creature-spell fixture guard must fail");
    assert!(
        format!("{error:#}").contains("requires fixture_guard evidence"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["fixture_manifest_sha256"] =
        serde_json::Value::String("5".repeat(64));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("unreviewed creature-spell fixture manifest must fail");
    assert!(
        format!("{error:#}").contains("exact reviewed manifest"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["database_snapshot_sha256"] =
        serde_json::Value::String("3".repeat(64));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("different creature-spell database snapshots must fail");
    assert!(
        format!("{error:#}").contains("creature-spell fixture identities differ"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["creature_entry"] = serde_json::Value::from(22_379);
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("wrong creature-spell world identity must fail");
    assert!(
        format!("{error:#}").contains("Cabal Interrogator"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["cleanup_verified"] = serde_json::Value::Bool(false);
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("unverified creature-spell cleanup must fail");
    assert!(
        format!("{error:#}").contains("cleanup was not verified"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["bot_report"] = serde_json::Value::Null;
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("creature-spell fixture must require bot evidence");
    assert!(
        format!("{error:#}").contains("requires bot_report evidence"),
        "unexpected error: {error:#}"
    );

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["account_id"] = serde_json::Value::from(10);
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("noncanonical creature-spell bot identity must fail");
    assert!(
        format!("{error:#}").contains("canonical TESTBOT2 identity"),
        "unexpected error: {error:#}"
    );

    let manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
    let report_original = fs::read(&report_path).unwrap();
    let mut report: serde_json::Value = serde_json::from_slice(&report_original).unwrap();
    report["results"][0]["creature_spell_go_hit_target_count"] = serde_json::Value::from(0);
    let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
    fs::write(&report_path, &report_bytes).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["bot_report"]["report_sha256"] =
        serde_json::Value::String(sha256_bytes(&report_bytes));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("creature-spell report without one hit must fail");
    assert!(
        format!("{error:#}").contains("canonical successful creature-spell window"),
        "unexpected error: {error:#}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detour_raw_pair_allows_private_evidence_to_differ_but_pins_shared_fixture_and_window() {
    let root = test_root("detour-identity");
    let flow = "detour-chase-around-obstacle";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

    let cpp_original = fs::read(&cpp_manifest).unwrap();
    let mut cpp_dirty: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    cpp_dirty["source_worktree_dirty"] = serde_json::Value::Bool(true);
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&cpp_dirty).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("dirty legacy C++ detour provenance must fail");
    assert!(
        format!("{error:#}").contains("detour C++ source worktree must be clean"),
        "unexpected error: {error:#}"
    );
    fs::write(&cpp_manifest, cpp_original).unwrap();

    let cpp_original = fs::read(&cpp_manifest).unwrap();
    let mut cpp_wrong_binary: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    cpp_wrong_binary["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&cpp_wrong_binary).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("C++ binary/source revision mismatch must fail");
    assert!(
        format!("{error:#}").contains("embedded executable revision must equal source_repo_head"),
        "unexpected error: {error:#}"
    );
    fs::write(&cpp_manifest, cpp_original).unwrap();

    let rust_original = fs::read(&rust_manifest).unwrap();
    let mut rust_wrong_binary: serde_json::Value = serde_json::from_slice(&rust_original).unwrap();
    rust_wrong_binary["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&rust_wrong_binary).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("Rust binary/source revision mismatch must fail");
    assert!(
        format!("{error:#}")
            .contains("Rust embedded executable revision must equal source_repo_head"),
        "unexpected error: {error:#}"
    );
    fs::write(&rust_manifest, rust_original).unwrap();

    let original = fs::read(&rust_manifest).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["fixture_manifest_sha256"] =
        serde_json::Value::String("5".repeat(64));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("different committed fixture identity must fail");
    assert!(format!("{error:#}").contains("exact reviewed detour manifest"));

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    manifest["fixture_guard"]["database_snapshot_sha256"] =
        serde_json::Value::String("3".repeat(64));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("different initial database snapshots must fail");
    assert!(format!("{error:#}").contains("detour fixture identities differ"));

    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
    let mut report: serde_json::Value =
        serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    report["results"][0]["detour_chase_window_target_moves"] = serde_json::Value::from(2);
    let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
    fs::write(&report_path, &report_bytes).unwrap();
    manifest["bot_report"]["report_sha256"] =
        serde_json::Value::String(sha256_bytes(&report_bytes));
    fs::write(
        &rust_manifest,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("a non-isolated movement window must fail");
    assert!(format!("{error:#}").contains("canonical isolated detour-chase window"));
    fs::remove_dir_all(root).unwrap();
}
