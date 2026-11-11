//! Frozen character bank-tab settings; canonical construction remains absent.
/// Private names, icons and descriptions deliberately have no Debug implementation.
pub struct NewBankTabSettings {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub deposit_flags: i32,
}
