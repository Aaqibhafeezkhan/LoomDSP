//! Real-time audio helpers for LoomDSP.
//!
//! Everything in this crate is safe to call on the audio thread: no
//! allocation, no locks, no system calls.
//!
//! This crate has no LoomDSP-specific types, so it can be published and
//! reused on its own.

#![warn(missing_docs)]

mod denormal;
mod smooth;

pub use denormal::ScopedFlushDenormals;
pub use smooth::Smoothed;
