//! Durable local data: editable configuration and machine-owned state.
//!
//! Configuration stays small in Phase 1: one optional startup theme loaded
//! from the platform configuration directory. State persistence arrives with
//! its own phase and keeps a separate versioned format.

pub mod config;
