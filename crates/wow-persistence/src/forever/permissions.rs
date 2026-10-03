//! Complete default-RBAC startup input, not an account-specific policy manager.
//! Explicit account grants/denials and nonzero security retain admission gates.

#[derive(Default)]
pub struct DefaultPermissionRows {
    pub known: Vec<u32>,
    pub links: Vec<(u32, u32)>,
    pub roots: Vec<u32>,
}
