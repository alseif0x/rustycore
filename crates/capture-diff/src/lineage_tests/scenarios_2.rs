//! Capture-lineage regressions, part 2 of 2.
//!
//! Moved out of the lineage_tests.rs root under #683; every test is unchanged.

use super::*;

#[test]
fn detour_bot_reports_are_cryptographically_bound_to_each_selected_raw_side() {
    fn bind_report(manifest_path: &Path, heartbeat: &[u8], movement: &[u8]) {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        report["results"][0]["detour_chase_heartbeat_sha256"] =
            serde_json::Value::String(sha256_bytes(heartbeat));
        report["results"][0]["detour_chase_monster_move_sha256"] =
            serde_json::Value::String(sha256_bytes(movement));
        report["results"][0]["detour_chase_monster_move_bytes"] =
            serde_json::Value::from(movement.len() as u64);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }

    fn selected_capture(source: &str, heartbeat: &[u8], movement: &[u8]) -> Capture {
        Capture::new(
            source,
            vec![
                CapturedPacket {
                    direction: Direction::C2S,
                    connection_id: 1,
                    opcode: 0x3A10,
                    body: heartbeat.to_vec(),
                },
                CapturedPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: 0x2DD4,
                    body: movement.to_vec(),
                },
                CapturedPacket {
                    direction: Direction::C2S,
                    connection_id: 1,
                    opcode: 0x3769,
                    body: b"fence".to_vec(),
                },
            ],
        )
    }

    let root = test_root("detour-report-packet-binding");
    let flow = "detour-chase-around-obstacle";
    let (cpp_path, cpp_manifest, rust_path, rust_manifest) = make_raw_pair(&root, flow);
    let cpp_heartbeat = b"cpp-heartbeat";
    let cpp_movement = b"cpp-movement";
    let rust_heartbeat = b"rust-heartbeat";
    let rust_movement = b"rust-movement";
    bind_report(&cpp_manifest, cpp_heartbeat, cpp_movement);
    bind_report(&rust_manifest, rust_heartbeat, rust_movement);
    let raw = validate_raw_pair(
        flow,
        &cpp_path,
        &cpp_manifest,
        &rust_path,
        &rust_manifest,
        true,
    )
    .unwrap();
    let cpp = selected_capture("cpp", cpp_heartbeat, cpp_movement);
    let rust = selected_capture("rust", rust_heartbeat, rust_movement);
    validate_bot_report_capture_binding(flow, &raw, &cpp, &rust).unwrap();

    let mut mismatched = rust.clone();
    mismatched.packets[1].body.push(0);
    let error = validate_bot_report_capture_binding(flow, &raw, &cpp, &mismatched)
        .expect_err("a report from a different execution must fail");
    assert!(format!("{error:#}").contains("does not match selected RAW"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn creature_spell_bot_reports_are_cryptographically_bound_to_selected_start_go() {
    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        current: u8,
        used: u8,
    }

    impl BitWriter {
        fn bits(&mut self, value: u32, width: u8) {
            for shift in (0..width).rev() {
                self.current |= (((value >> shift) & 1) as u8) << (7 - self.used);
                self.used += 1;
                if self.used == 8 {
                    self.bytes.push(self.current);
                    self.current = 0;
                    self.used = 0;
                }
            }
        }

        fn finish(mut self) -> Vec<u8> {
            if self.used != 0 {
                self.bytes.push(self.current);
            }
            self.bytes
        }
    }

    fn guid(
        high_type: u8,
        subtype: u8,
        realm: u16,
        map: u16,
        entry: u32,
        counter: u64,
    ) -> crate::semantic::ExactObjectGuid {
        crate::semantic::ExactObjectGuid {
            low: counter & 0x0000_00FF_FFFF_FFFF,
            high: (u64::from(high_type & 0x3F) << 58)
                | (u64::from(realm & 0x1FFF) << 42)
                | (u64::from(map & 0x1FFF) << 29)
                | (u64::from(entry & 0x7F_FFFF) << 6)
                | u64::from(subtype & 0x3F),
        }
    }

    fn push_guid(out: &mut Vec<u8>, guid: crate::semantic::ExactObjectGuid) {
        let low = guid.low.to_le_bytes();
        let high = guid.high.to_le_bytes();
        let low_mask = low.iter().enumerate().fold(0u8, |mask, (index, byte)| {
            mask | (u8::from(*byte != 0) << index)
        });
        let high_mask = high.iter().enumerate().fold(0u8, |mask, (index, byte)| {
            mask | (u8::from(*byte != 0) << index)
        });
        out.push(low_mask);
        out.push(high_mask);
        out.extend(low.into_iter().filter(|byte| *byte != 0));
        out.extend(high.into_iter().filter(|byte| *byte != 0));
    }

    fn spell_body(
        caster: crate::semantic::ExactObjectGuid,
        cast_id: crate::semantic::ExactObjectGuid,
        victim: crate::semantic::ExactObjectGuid,
        spell_go: bool,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        for value in [
            caster,
            caster,
            cast_id,
            crate::semantic::ExactObjectGuid { low: 0, high: 0 },
        ] {
            push_guid(&mut out, value);
        }
        out.extend(15_691_i32.to_le_bytes());
        out.extend(244_493_i32.to_le_bytes());
        out.extend(if spell_go { 0x100_u32 } else { 2_u32 }.to_le_bytes());
        out.extend(0_u32.to_le_bytes());
        out.extend(if spell_go { 123_u32 } else { 0_u32 }.to_le_bytes());
        out.extend(0_u32.to_le_bytes());
        out.extend(0_f32.to_bits().to_le_bytes());
        out.push(0);
        out.extend(0_u32.to_le_bytes());
        out.extend(0_u32.to_le_bytes());
        out.extend(0_u32.to_le_bytes());
        out.push(0);
        push_guid(
            &mut out,
            crate::semantic::ExactObjectGuid { low: 0, high: 0 },
        );

        let mut counts = BitWriter::default();
        counts.bits(u32::from(spell_go), 16);
        counts.bits(0, 16);
        counts.bits(0, 16);
        counts.bits(0, 9);
        counts.bits(0, 1);
        counts.bits(0, 16);
        counts.bits(0, 1);
        counts.bits(0, 1);
        out.extend(counts.finish());

        let mut target = BitWriter::default();
        target.bits(2, 28);
        target.bits(0, 1);
        target.bits(0, 1);
        target.bits(0, 1);
        target.bits(0, 1);
        target.bits(0, 7);
        out.extend(target.finish());
        push_guid(&mut out, victim);
        push_guid(
            &mut out,
            crate::semantic::ExactObjectGuid { low: 0, high: 0 },
        );
        if spell_go {
            push_guid(&mut out, victim);
            out.push(0); // basic SpellGo combat-log bit plus canonical padding
        }
        out
    }

    fn bind_report(
        manifest_path: &Path,
        start: &[u8],
        go: &[u8],
        caster: crate::semantic::ExactObjectGuid,
        cast_id: crate::semantic::ExactObjectGuid,
        victim: crate::semantic::ExactObjectGuid,
    ) {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let result = &mut report["results"][0];
        result["creature_spell_start_body_sha256"] = serde_json::Value::String(sha256_bytes(start));
        result["creature_spell_start_body_bytes"] = serde_json::Value::from(start.len() as u64);
        result["creature_spell_go_body_sha256"] = serde_json::Value::String(sha256_bytes(go));
        result["creature_spell_go_body_bytes"] = serde_json::Value::from(go.len() as u64);
        result["creature_spell_cast_id_low"] = serde_json::Value::from(cast_id.low);
        result["creature_spell_cast_id_high"] = serde_json::Value::from(cast_id.high);
        result["creature_spell_caster_guid_low"] = serde_json::Value::from(caster.low);
        result["creature_spell_caster_guid_high"] = serde_json::Value::from(caster.high);
        result["creature_spell_victim_guid_low"] = serde_json::Value::from(victim.low);
        result["creature_spell_victim_guid_high"] = serde_json::Value::from(victim.high);
        result["creature_spell_target_runtime_counter"] =
            serde_json::Value::from(caster.low & 0x0000_00FF_FFFF_FFFF);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }

    let caster = guid(8, 0, 1, 530, 22_378, 78_686);
    let cast_id = guid(47, 3, 1, 530, 15_691, 44);
    let victim = guid(2, 0, 1, 0, 0, 15);
    let start = spell_body(caster, cast_id, victim, false);
    let go = spell_body(caster, cast_id, victim, true);
    let selected = |source: &str| {
        Capture::new(
            source,
            vec![
                CapturedPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: SMSG_SPELL_START,
                    body: start.clone(),
                },
                CapturedPacket {
                    direction: Direction::S2C,
                    connection_id: 1,
                    opcode: SMSG_SPELL_GO,
                    body: go.clone(),
                },
            ],
        )
    };
    crate::semantic::validate_creature_spell_casting_capture(&selected("shape")).unwrap();

    let root = test_root("creature-spell-report-packet-binding");
    let flow = "creature-spell-casting";
    let (cpp_path, cpp_manifest, rust_path, rust_manifest) = make_raw_pair(&root, flow);
    bind_report(&cpp_manifest, &start, &go, caster, cast_id, victim);
    bind_report(&rust_manifest, &start, &go, caster, cast_id, victim);
    let raw = validate_raw_pair(
        flow,
        &cpp_path,
        &cpp_manifest,
        &rust_path,
        &rust_manifest,
        true,
    )
    .unwrap();
    let cpp = selected("cpp");
    let rust = selected("rust");
    validate_bot_report_capture_binding(flow, &raw, &cpp, &rust).unwrap();

    let mut mismatched = rust.clone();
    mismatched.packets[1].body.push(0);
    let error = validate_bot_report_capture_binding(flow, &raw, &cpp, &mismatched)
        .expect_err("a creature-spell report from a different execution must fail");
    assert!(format!("{error:#}").contains("does not match selected RAW"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vendor_raw_pair_requires_exact_bot_report_and_retains_both_reports() {
    let root = test_root("vendor-report");
    let flow = "vendor-extended-cost-purchase";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
    let flow_dir = make_derived_flow(&root, flow, &raw);
    assert!(
        flow_dir
            .join(RAW_PROVENANCE_DIR)
            .join(CPP_BOT_REPORT_FILE)
            .is_file()
    );
    assert!(
        flow_dir
            .join(RAW_PROVENANCE_DIR)
            .join(RUST_BOT_REPORT_FILE)
            .is_file()
    );

    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
    let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
    let mut report: serde_json::Value =
        serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    report["results"][0]["vendor_relogin_verified"] = serde_json::Value::Bool(false);
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
        .expect_err("vendor report without relog proof must fail");
    assert!(format!("{error:#}").contains("canonical successful vendor flow"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn derived_loot_lineage_retains_and_revalidates_bot_reports() {
    let root = test_root("loot-report-retention");
    let flow = "loot-single-item-claim";
    let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
    let flow_dir = make_derived_flow(&root, flow, &raw);
    verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();

    fs::write(
        flow_dir.join(RAW_PROVENANCE_DIR).join(RUST_BOT_REPORT_FILE),
        b"{}",
    )
    .unwrap();
    let error = verify_required_lineage(flow, &flow_dir, &required_selection())
        .expect_err("retained bot report tamper must fail");
    assert!(error.to_string().contains("bot report SHA-256"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_flow_publication_is_atomic_noreplace_under_target_race() {
    let root = test_root("atomic-noreplace");
    let transaction = AtomicFlowImport::prepare(&root, "new-flow").unwrap();
    fs::write(transaction.staging_dir().join("cpp.pkt"), b"candidate").unwrap();

    let target = root.join("new-flow");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"concurrent owner").unwrap();
    let error = transaction
        .publish()
        .expect_err("concurrent target must never be replaced");
    assert!(format!("{error:#}").contains("without replacement"));
    assert_eq!(
        fs::read(target.join("sentinel")).unwrap(),
        b"concurrent owner"
    );
    assert!(!target.join("cpp.pkt").exists());
    fs::remove_dir_all(root).unwrap();
}
