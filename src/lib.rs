//! **Hydrogen** — Multi-Product AI Curriculum Compiler, `Level 9 -> Level 1` Visual Solution Engine,
//! and Encrypted `L7` `.raphs` Vault Manager (`Rust` + `memory_core_rs 1.7.0` + `theta_calc` + `carbon-protocol`).

pub mod base64_util;
pub mod carbon;
pub mod config;
pub mod error;
pub mod theta;
pub mod vault;

/// Package version of `hydrogen`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
