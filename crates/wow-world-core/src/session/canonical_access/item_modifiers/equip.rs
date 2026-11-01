// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::OwnedItemModifiersAccessLikeCpp;

impl OwnedItemModifiersAccessLikeCpp<'_> {
    pub fn reborrow_like_cpp(&self) -> OwnedItemModifiersAccessLikeCpp<'_> {
        OwnedItemModifiersAccessLikeCpp { core: self.core }
    }
}
