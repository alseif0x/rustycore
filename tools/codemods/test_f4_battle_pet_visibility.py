"""Focused self-tests for named battle-pet account visibility promotions."""

import unittest
from pathlib import Path

import f4_battle_pet_visibility as codemod


class BattlePetVisibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.lexer = codemod._lexer(Path(codemod.REPO))

    def test_type_promotion_keeps_tuple_field_private_and_is_idempotent(self):
        source = "pub(crate) struct BattlePetLeaseIdLikeCpp(u64);\n"

        widened, changed = codemod.promote_type(
            source, self.lexer, "struct", "BattlePetLeaseIdLikeCpp"
        )
        repeated, changed_again = codemod.promote_type(
            widened, self.lexer, "struct", "BattlePetLeaseIdLikeCpp"
        )

        self.assertTrue(changed)
        self.assertFalse(changed_again)
        self.assertEqual(repeated, "pub struct BattlePetLeaseIdLikeCpp(u64);\n")

    def test_async_generic_method_is_matched_by_exact_owner_and_name(self):
        source = """impl BattlePetAccountOwnerLikeCpp {
    pub(crate) async fn try_mutate_pet_like_cpp<R, F>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
    }
    pub(crate) fn private_helper_like_cpp(&self) {}
}
impl OtherOwnerLikeCpp {
    pub(crate) async fn try_mutate_pet_like_cpp<R>(&self) -> R { todo!() }
}
"""

        widened, changed = codemod.promote_method(
            source,
            self.lexer,
            "BattlePetAccountOwnerLikeCpp",
            "try_mutate_pet_like_cpp",
        )

        self.assertTrue(changed)
        self.assertIn("pub async fn try_mutate_pet_like_cpp<R, F>", widened)
        self.assertIn("pub(crate) fn private_helper_like_cpp", widened)
        self.assertIn("pub(crate) async fn try_mutate_pet_like_cpp<R>", widened)

    def test_duplicate_type_or_method_candidate_is_rejected(self):
        duplicate_type = (
            "pub(crate) struct BattlePetLeaseIdLikeCpp(u64);\n"
            "pub(crate) struct BattlePetLeaseIdLikeCpp(u32);\n"
        )
        with self.assertRaisesRegex(codemod.CodemodError, "expected one struct declaration"):
            codemod.promote_type(
                duplicate_type, self.lexer, "struct", "BattlePetLeaseIdLikeCpp"
            )

        duplicate_method = """impl BattlePetAccountOwnerLikeCpp {
    pub(crate) async fn try_mutate_pet_like_cpp<R>(&self) -> R { todo!() }
    pub(crate) async fn try_mutate_pet_like_cpp<T>(&self) -> T { todo!() }
}
"""
        with self.assertRaisesRegex(codemod.CodemodError, "expected one method"):
            codemod.promote_method(
                duplicate_method,
                self.lexer,
                "BattlePetAccountOwnerLikeCpp",
                "try_mutate_pet_like_cpp",
            )

    def test_only_reviewed_aliases_are_promoted_and_repeat_is_noop(self):
        _, old, new = codemod.ALIAS_BLOCKS[0]
        source = (
            old
            + "\n#[cfg(test)]\npub(crate) use wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;\n"
        )

        widened, changed = codemod._replace_exact_block(
            source, "test aliases", old, new
        )
        repeated, changed_again = codemod._replace_exact_block(
            widened, "test aliases", old, new
        )

        self.assertTrue(changed)
        self.assertFalse(changed_again)
        self.assertEqual(repeated, widened)
        self.assertIn("pub(crate) use wow_persistence::PersistenceFutureLikeCpp", widened)

    def test_receipt_alias_keeps_feature_gate_and_future_alias_is_test_only(self):
        _, old, new = codemod.ALIAS_BLOCKS[1]

        widened, changed = codemod._replace_exact_block(old, "receipt alias", old, new)

        self.assertTrue(changed)
        self.assertEqual(widened, new)
        self.assertIn(
            '#[cfg(any(test, feature = "test-fixtures"))]\n'
            "pub use wow_persistence::DurableBattlePetAddReceiptLikeCpp;",
            widened,
        )
        self.assertIn(
            "#[cfg(test)]\n"
            'pub(crate) use '
            'wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;',
            widened,
        )
        self.assertNotIn(
            '#[cfg(any(test, feature = "test-fixtures"))]\n'
            'pub(crate) use wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;',
            widened,
        )

        repeated, changed_again = codemod._replace_exact_block(
            widened, "receipt alias", old, new
        )
        self.assertFalse(changed_again)
        self.assertEqual(repeated, widened)

        malformed_gate = new.replace(
            "#[cfg(test)]\npub(crate) use wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;",
            '#[cfg(feature = "test-fixtures")]\n'
            "pub(crate) use wow_persistence::PersistenceFutureLikeCpp as PersistenceFuture;",
        )
        with self.assertRaisesRegex(codemod.CodemodError, "one original or widened"):
            codemod._replace_exact_block(
                malformed_gate, "receipt alias", old, new
            )


if __name__ == "__main__":
    unittest.main()
