use super::*;

#[test]
fn heirloom_preparation_keeps_input_order_and_last_duplicate() {
    let mut calls = Vec::new();
    let rows = PlayerCollectionStateLikeCpp::prepare_heirlooms(
        [(20, 1), (10, 2), (20, 3)],
        |item_id, flags| {
            calls.push((item_id, flags));
            Some(flags * 10)
        },
    );
    assert_eq!(calls, vec![(20, 1), (10, 2), (20, 3)]);
    assert_eq!(rows.keys().copied().collect::<Vec<_>>(), vec![10, 20]);
    assert_eq!(
        rows[&20],
        PlayerAccountHeirloomDataLikeCpp {
            flags: 3,
            bonus_id: 30
        }
    );
}

#[test]
fn heirloom_preparation_accepts_missing_catalog_zero_bonus() {
    let rows = PlayerCollectionStateLikeCpp::prepare_heirlooms([(u32::MAX, 7)], |_, _| Some(0));
    assert_eq!(
        rows[&u32::MAX],
        PlayerAccountHeirloomDataLikeCpp {
            flags: 7,
            bonus_id: 0
        }
    );
}

#[test]
fn heirloom_missing_row_skips_without_erasing_previous_duplicate() {
    let rows =
        PlayerCollectionStateLikeCpp::prepare_heirlooms([(10, 1), (10, 2), (20, 3)], |_, flags| {
            (flags == 1).then_some(5)
        });
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[&10],
        PlayerAccountHeirloomDataLikeCpp {
            flags: 1,
            bonus_id: 5
        }
    );
}

#[test]
fn toy_preparation_preserves_all_flags_and_last_duplicate() {
    let rows = PlayerCollectionStateLikeCpp::prepare_toys([
        (10, false, false),
        (20, true, false),
        (30, false, true),
        (u32::MAX, true, true),
        (20, false, true),
    ]);
    assert_eq!(
        rows.into_iter().collect::<Vec<_>>(),
        vec![(10, 0), (20, 2), (30, 2), (u32::MAX, 3),]
    );
    assert_eq!(PlayerCollectionStateLikeCpp::toy_flags(true, false), 1);
}
