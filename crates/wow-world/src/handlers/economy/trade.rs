// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Legacy trade handler family.
//!
//! All opcodes of this family moved to
//! `wow-world-application::trade_handlers` under #1263 F5; the shell keeps only
//! the cfg(test) delegates in `test_shims`.

#[cfg(test)]
mod test_shims;
