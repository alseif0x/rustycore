use super::select_random_enchantment;
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn random_enchantment_selection_uses_cpp_weighted_chances() {
    let group = [
        (10, 0.0),
        (11, 100.0),
    ];
    assert_eq!(
        select_random_enchantment(group, &mut StdRng::seed_from_u64(5)),
        Some(11)
    );
}
