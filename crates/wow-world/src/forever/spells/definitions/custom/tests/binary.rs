use super::*;

fn run(
    mut s: SpellDefinitionSeeds,
    key: Key,
    draw: &mut impl FnMut(f32, f32) -> Result<f32, SpellValueError>,
) -> (SpellDefinitionSeeds, CustomAttributeCounts) {
    let mut counts = CustomAttributeCounts::default();
    super::super::binary::initialize(&mut s, key, &items(), draw, &mut counts).unwrap();
    (s, counts)
}

#[test]
fn every_effect_and_periodic_aura_exclusion_keeps_source_binary_predicates() {
    let key = (900_001, 0);
    for kind in 0..361 {
        for aura in [0, 3, 89, 4, 53, 62, 226, 2] {
            let s = fresh(
                Default::default(),
                [(key, definition(vec![effect(kind, aura, 10.0)]))],
            );
            let (s, _) = run(s, key, &mut no_draw);
            let expected = kind != 0
                && !matches!(kind, 2 | 58 | 17 | 121 | 31 | 64 | 142)
                && !(matches!(
                    kind,
                    27 | 6 | 35 | 65 | 128 | 129 | 119 | 143 | 174 | 202 | 271
                ) && matches!(aura, 3 | 89 | 4 | 53 | 62 | 226));
            assert_eq!(flags(&s, key) & BINARY != 0, expected, "{kind}/{aura}");
        }
    }
}

#[test]
fn zero_control_and_no_immunities_read_current_flags_not_later_family_attributes() {
    let key = (900_001, 0);
    for (kind, cc, immunity, expected) in [
        (3, false, false, false),
        (68, false, false, true),
        (3, true, false, true),
        (68, true, true, false),
    ] {
        let mut d = definition(vec![effect(kind, 0, 0.0)]);
        d.custom_attributes = if cc { AURA_CC } else { 0 };
        d.fields.attributes[0] = if immunity { 0x2000_0000 } else { 0 };
        let (s, _) = run(fresh(Default::default(), [(key, d)]), key, &mut no_draw);
        assert_eq!(flags(&s, key) & BINARY != 0, expected);
    }
    let mut d = definition(vec![effect(3, 0, 0.0)]);
    d.fields.spell_family_name = 4;
    d.fields.spell_family_flags[0] = 0x20000;
    let s = admitted(Default::default(), [(key, d)], vec![key])
        .with_custom_attributes(&items(), &mut no_draw)
        .unwrap();
    assert_eq!(flags(&s, key) & (BINARY | AURA_CC), AURA_CC);
}

#[test]
fn source_draw_occurs_before_exception_ids_and_first_qualifier_stops_binary_scan() {
    for (id, family, family_flags) in [
        (69649, 0, [0; 4]),
        (55095, 0, [0; 4]),
        (900_001, 3, [0x20, 0, 0, 0]),
        (900_001, 5, [0, 0x40000, 0, 0]),
    ] {
        let key = (id, 0);
        let mut e = effect(3, 0, 100.0);
        e.scaling_variance = 0.5;
        let mut d = definition(vec![e]);
        d.fields.spell_family_name = family;
        d.fields.spell_family_flags = family_flags;
        let mut draws = 0;
        let (s, counts) = run(fresh(Default::default(), [(key, d)]), key, &mut |_, _| {
            draws += 1;
            Ok(0.0)
        });
        assert_eq!(
            (draws, counts.binary_assignments, flags(&s, key) & BINARY),
            (1, 0, 0)
        );
    }
    let key = (900_001, 0);
    let rows = [10.0, 20.0].map(|bp| {
        let mut e = effect(3, 0, bp);
        e.scaling_variance = 0.5;
        e
    });
    let mut draws = 0;
    let (_, counts) = run(
        fresh(Default::default(), [(key, definition(Vec::from(rows)))]),
        key,
        &mut |_, _| {
            draws += 1;
            Ok(0.0)
        },
    );
    assert_eq!((draws, counts.binary_assignments), (1, 1));
}

#[test]
fn always_hit_skips_draws_but_does_not_clear_an_existing_sql_binary_bit() {
    let key = (900_001, 0);
    let mut e = effect(3, 0, 10.0);
    e.scaling_variance = 0.5;
    let mut d = definition(vec![e]);
    d.fields.attributes[3] = 0x40000;
    d.custom_attributes = BINARY;
    let (s, counts) = run(fresh(Default::default(), [(key, d)]), key, &mut no_draw);
    assert_eq!(flags(&s, key), BINARY);
    assert_eq!(counts.binary_assignments, 0);
}
