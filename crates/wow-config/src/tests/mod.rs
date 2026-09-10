//! Configuration loader regressions.
//!
//! Separated from lib.rs under #683.

use crate::*;

use super::*;
use std::sync::{Mutex, MutexGuard};

static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn global_config_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().expect("test lock poisoned")
}

/// Helper: create an isolated `ConfigStore` and parse into it so tests
/// do not interfere with the global singleton.
fn parse(content: &str) -> ConfigStore {
    let mut store = ConfigStore::default();
    store.parse(content).expect("parse failed");
    store
}

// -- Parsing basics -----------------------------------------------------

// -- Comments -----------------------------------------------------------

// -- Empty / whitespace lines -------------------------------------------

// -- Case-insensitive lookup --------------------------------------------

// -- Numeric parsing ----------------------------------------------------

// -- Defaults -----------------------------------------------------------

// -- Global API round-trip ----------------------------------------------

// -- Error paths --------------------------------------------------------

// -- Multiple keys ------------------------------------------------------

// -- Overwrite on reload ------------------------------------------------

// -- Value with equals sign ---------------------------------------------

// -- Quoted value containing hash ---------------------------------------

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rustycore_wow_config_{name}_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("unnamed")
    ));

    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp dir failed");
    path
}
mod scenarios;
