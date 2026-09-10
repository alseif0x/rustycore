//! Tests for the battle_pet_account_adapter module.
//!
//! Separated from battle_pet_account_adapter.rs under #687.

use super::*;

#[test]
fn signed_column_casts_preserve_non_negative_values_like_cpp() {
    assert_eq!(battle_pet_column_i64_as_u64_like_cpp(0, 0).unwrap(), 0);
    assert_eq!(battle_pet_column_i64_as_u64_like_cpp(42, 0).unwrap(), 42);
    assert_eq!(
        battle_pet_column_i64_as_u64_like_cpp(i64::MAX, 0).unwrap(),
        i64::MAX as u64
    );
    assert_eq!(battle_pet_column_i32_as_u32_like_cpp(70, 1).unwrap(), 70);
    assert_eq!(battle_pet_column_i16_as_u16_like_cpp(25, 2).unwrap(), 25);
    assert_eq!(battle_pet_column_i8_as_u8_like_cpp(3, 7).unwrap(), 3);
}

#[test]
fn signed_column_casts_reject_negative_values_like_cpp() {
    for result in [
        battle_pet_column_i64_as_u64_like_cpp(-1, 0),
        battle_pet_column_i64_as_u64_like_cpp(i64::MIN, 0),
    ] {
        let error = result.unwrap_err();
        assert!(
            matches!(error, BattlePetPersistenceErrorLikeCpp::Database(ref message)
                if message.contains("negative value") && message.contains("column 0")),
            "unexpected error: {error:?}"
        );
    }
    assert!(battle_pet_column_i32_as_u32_like_cpp(-1, 1).is_err());
    assert!(battle_pet_column_i16_as_u16_like_cpp(-1, 2).is_err());
    assert!(battle_pet_column_i8_as_u8_like_cpp(-1, 7).is_err());
}

#[test]
fn advisory_lock_names_are_scoped_to_one_login_database() {
    let first = battle_pet_account_lock_name_like_cpp("0123456789abcdef0123456789abcdef", u32::MAX);
    let second =
        battle_pet_account_lock_name_like_cpp("fedcba9876543210fedcba9876543210", u32::MAX);
    assert_ne!(first, second);
    assert!(first.len() <= 64);
    assert!(second.len() <= 64);
}
