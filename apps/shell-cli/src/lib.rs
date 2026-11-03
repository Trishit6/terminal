//! Application library for the custom shell binary.
//!
//! The library target keeps the REPL testable while `main.rs` remains a thin
//! process boundary.

pub mod cli;
pub mod repl;
pub mod startup;
