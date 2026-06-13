use rand::Rng;
use rand::distributions::{Distribution, WeightedIndex};

/// C++ `irand(min, max)`: inclusive signed integer range.
pub fn irand_like_cpp(min: i32, max: i32) -> i32 {
    irand_with_rng_like_cpp(min, max, &mut rand::thread_rng())
}

pub fn irand_with_rng_like_cpp<R: Rng + ?Sized>(min: i32, max: i32, rng: &mut R) -> i32 {
    assert!(max >= min, "irand_like_cpp requires max >= min");
    rng.gen_range(min..=max)
}

/// C++ `urand(min, max)`: inclusive unsigned integer range.
pub fn urand_like_cpp(min: u32, max: u32) -> u32 {
    urand_with_rng_like_cpp(min, max, &mut rand::thread_rng())
}

pub fn urand_with_rng_like_cpp<R: Rng + ?Sized>(min: u32, max: u32, rng: &mut R) -> u32 {
    assert!(max >= min, "urand_like_cpp requires max >= min");
    rng.gen_range(min..=max)
}

/// C++ `urandms(min, max)`: inclusive millisecond value between second bounds.
pub fn urandms_like_cpp(min_seconds: u32, max_seconds: u32) -> u32 {
    urandms_with_rng_like_cpp(min_seconds, max_seconds, &mut rand::thread_rng())
}

pub fn urandms_with_rng_like_cpp<R: Rng + ?Sized>(
    min_seconds: u32,
    max_seconds: u32,
    rng: &mut R,
) -> u32 {
    const IN_MILLISECONDS: u32 = 1_000;
    assert!(
        u32::MAX / IN_MILLISECONDS >= max_seconds,
        "urandms_like_cpp max seconds would overflow milliseconds"
    );
    urand_with_rng_like_cpp(
        min_seconds * IN_MILLISECONDS,
        max_seconds * IN_MILLISECONDS,
        rng,
    )
}

/// C++ `rand32()`: random `uint32`.
pub fn rand32_like_cpp() -> u32 {
    rand::thread_rng().r#gen()
}

/// C++ `frand(min, max)`: float range using the active random engine.
pub fn frand_like_cpp(min: f32, max: f32) -> f32 {
    frand_with_rng_like_cpp(min, max, &mut rand::thread_rng())
}

pub fn frand_with_rng_like_cpp<R: Rng + ?Sized>(min: f32, max: f32, rng: &mut R) -> f32 {
    assert!(max >= min, "frand_like_cpp requires max >= min");
    if max == min {
        return min;
    }
    rng.gen_range(min..max)
}

/// C++ `rand_norm()`: float in `[0.0, 1.0)`.
pub fn rand_norm_like_cpp() -> f32 {
    rand_norm_with_rng_like_cpp(&mut rand::thread_rng())
}

pub fn rand_norm_with_rng_like_cpp<R: Rng + ?Sized>(rng: &mut R) -> f32 {
    rng.gen_range(0.0f32..1.0f32)
}

/// C++ `rand_chance()`: float in `[0.0, 100.0)`.
pub fn rand_chance_like_cpp() -> f32 {
    rand_chance_with_rng_like_cpp(&mut rand::thread_rng())
}

pub fn rand_chance_with_rng_like_cpp<R: Rng + ?Sized>(rng: &mut R) -> f32 {
    rng.gen_range(0.0f32..100.0f32)
}

/// C++ `roll_chance_f(chance)`.
pub fn roll_chance_f_like_cpp(chance: f32) -> bool {
    chance > rand_chance_like_cpp()
}

pub fn roll_chance_f_with_rng_like_cpp<R: Rng + ?Sized>(chance: f32, rng: &mut R) -> bool {
    chance > rand_chance_with_rng_like_cpp(rng)
}

/// C++ `roll_chance_i(chance)`.
pub fn roll_chance_i_like_cpp(chance: i32) -> bool {
    roll_chance_i_with_rng_like_cpp(chance, &mut rand::thread_rng())
}

pub fn roll_chance_i_with_rng_like_cpp<R: Rng + ?Sized>(chance: i32, rng: &mut R) -> bool {
    chance > irand_with_rng_like_cpp(0, 99, rng)
}

/// C++ `urandweighted(count, chances)`: weighted index in `0..chances.len()`.
pub fn urandweighted_like_cpp(chances: &[f64]) -> u32 {
    urandweighted_with_rng_like_cpp(chances, &mut rand::thread_rng())
}

pub fn urandweighted_with_rng_like_cpp<R: Rng + ?Sized>(chances: &[f64], rng: &mut R) -> u32 {
    assert!(
        !chances.is_empty(),
        "urandweighted_like_cpp requires at least one chance"
    );
    assert!(
        chances.iter().all(|chance| *chance >= 0.0),
        "urandweighted_like_cpp requires non-negative weights"
    );
    if chances.iter().all(|chance| *chance == 0.0) {
        return urand_with_rng_like_cpp(0, u32::try_from(chances.len() - 1).unwrap(), rng);
    }

    let dist = WeightedIndex::new(chances).expect("urandweighted_like_cpp requires valid weights");
    u32::try_from(dist.sample(rng)).expect("weighted index must fit in u32")
}

/// C++ `Trinity::Containers::RandomResize(container, requestedSize)` for `Vec`.
///
/// If the vector already has at most `requested_size` elements it is unchanged.
/// Otherwise the kept elements are selected randomly while preserving their
/// original relative order.
pub fn random_resize_vec_like_cpp<T>(items: &mut Vec<T>, requested_size: usize) {
    random_resize_vec_with_rng_like_cpp(items, requested_size, &mut rand::thread_rng());
}

pub fn random_resize_vec_with_rng_like_cpp<T, R: Rng + ?Sized>(
    items: &mut Vec<T>,
    requested_size: usize,
    rng: &mut R,
) {
    if items.len() <= requested_size {
        return;
    }

    assert!(
        u32::try_from(items.len()).is_ok(),
        "RandomResize uses uint32 element counters in C++"
    );
    assert!(
        u32::try_from(requested_size).is_ok(),
        "RandomResize requested size must fit uint32 like C++"
    );

    let mut keep_index = 0usize;
    let mut elements_to_keep = requested_size as u32;
    let mut elements_to_process = items.len() as u32;

    for current_index in 0..items.len() {
        if elements_to_process == 0 {
            break;
        }

        if urand_with_rng_like_cpp(1, elements_to_process, rng) <= elements_to_keep {
            if keep_index != current_index {
                items.swap(keep_index, current_index);
            }
            keep_index += 1;
            elements_to_keep -= 1;
        }

        elements_to_process -= 1;
    }

    items.truncate(keep_index);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn integer_helpers_are_inclusive_like_cpp() {
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..256 {
            let value = urand_with_rng_like_cpp(7, 7, &mut rng);
            assert_eq!(value, 7);
            let signed = irand_with_rng_like_cpp(-3, -3, &mut rng);
            assert_eq!(signed, -3);
        }
    }

    #[test]
    fn urandms_multiplies_seconds_to_milliseconds_like_cpp() {
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..256 {
            let value = urandms_with_rng_like_cpp(2, 2, &mut rng);
            assert_eq!(value, 2_000);
        }
    }

    #[test]
    fn float_helpers_stay_inside_cpp_ranges() {
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..256 {
            let normal = rand_norm_with_rng_like_cpp(&mut rng);
            assert!((0.0..1.0).contains(&normal));

            let chance = rand_chance_with_rng_like_cpp(&mut rng);
            assert!((0.0..100.0).contains(&chance));

            let ranged = frand_with_rng_like_cpp(-2.5, 8.25, &mut rng);
            assert!((-2.5..8.25).contains(&ranged));
        }

        assert_eq!(frand_with_rng_like_cpp(5.0, 5.0, &mut rng), 5.0);
    }

    #[test]
    fn chance_helpers_match_cpp_comparison_direction() {
        let mut rng = StdRng::seed_from_u64(4);
        assert!(!roll_chance_f_with_rng_like_cpp(0.0, &mut rng));
        assert!(roll_chance_f_with_rng_like_cpp(100.0, &mut rng));

        let mut rng = StdRng::seed_from_u64(5);
        assert!(!roll_chance_i_with_rng_like_cpp(0, &mut rng));
        assert!(roll_chance_i_with_rng_like_cpp(100, &mut rng));
    }

    #[test]
    fn weighted_helper_returns_only_weighted_indices_like_cpp() {
        let mut rng = StdRng::seed_from_u64(6);
        for _ in 0..128 {
            assert_eq!(
                urandweighted_with_rng_like_cpp(&[0.0, 3.0, 0.0], &mut rng),
                1
            );
        }
    }

    #[test]
    fn weighted_helper_accepts_all_zero_weights_like_cpp() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..128 {
            let value = urandweighted_with_rng_like_cpp(&[0.0, 0.0, 0.0], &mut rng);
            assert!(value < 3);
        }
    }

    #[test]
    fn random_resize_noops_when_container_is_already_small_like_cpp() {
        let mut rng = StdRng::seed_from_u64(8);
        let mut values = vec![1, 2, 3];

        random_resize_vec_with_rng_like_cpp(&mut values, 3, &mut rng);
        assert_eq!(values, vec![1, 2, 3]);

        random_resize_vec_with_rng_like_cpp(&mut values, 4, &mut rng);
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn random_resize_keeps_requested_count_and_relative_order_like_cpp() {
        let mut rng = StdRng::seed_from_u64(9);
        let original = vec![10, 20, 30, 40, 50, 60];
        let mut values = original.clone();

        random_resize_vec_with_rng_like_cpp(&mut values, 3, &mut rng);

        assert_eq!(values.len(), 3);
        let mut last_index = 0usize;
        for value in values {
            let index = original
                .iter()
                .position(|candidate| *candidate == value)
                .expect("kept values must come from the original vector");
            assert!(index >= last_index);
            last_index = index;
        }
    }

    #[test]
    fn random_resize_can_drop_everything_like_cpp() {
        let mut rng = StdRng::seed_from_u64(10);
        let mut values = vec![1, 2, 3];

        random_resize_vec_with_rng_like_cpp(&mut values, 0, &mut rng);

        assert!(values.is_empty());
    }
}
