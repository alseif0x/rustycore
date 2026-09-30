//! Transient permissions for one explicitly selected loot application operation.
//! Ordinary wrappers preserve cfg!(test); feature fixtures never change that default.
#[derive(Clone, Copy)]
pub(super) enum LootOperationPolicy {
    Production,
    #[cfg(any(test, feature = "test-fixtures"))]
    PersonalMoneyFixture,
    #[cfg(any(test, feature = "test-fixtures"))]
    GameObjectFixture,
    #[cfg(any(test, feature = "test-fixtures"))]
    LocalRequestFixture,
}
impl LootOperationPolicy {
    pub(super) const fn release_policy(self) -> Self {
        match self {
            #[cfg(any(test, feature = "test-fixtures"))]
            Self::PersonalMoneyFixture => Self::Production,
            other => other,
        }
    }
    pub(super) const fn permits_local_cache(self, authority_observed: bool) -> bool {
        match self {
            Self::Production => super::represented_local_loot_fixture_allowed_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            Self::PersonalMoneyFixture | Self::GameObjectFixture | Self::LocalRequestFixture => !authority_observed,
        }
    }
    pub(super) const fn permits_initial_binding(self) -> bool {
        match self {
            Self::Production => super::represented_local_loot_fixture_allowed_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            Self::PersonalMoneyFixture | Self::GameObjectFixture | Self::LocalRequestFixture => false,
        }
    }
}
impl super::LootCyclePolicy {
    pub(super) const fn operation_policy(self) -> LootOperationPolicy {
        match self {
            Self::Production => LootOperationPolicy::Production,
            #[cfg(any(test, feature = "test-fixtures"))]
            Self::LegacyFixture => LootOperationPolicy::PersonalMoneyFixture,
        }
    }
}
