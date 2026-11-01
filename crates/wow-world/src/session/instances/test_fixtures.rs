//! Test-only detached instance state owned by the Session fixture.

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_instances::InstanceTestFixtureLikeCpp;
