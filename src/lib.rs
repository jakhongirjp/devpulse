//! devpulse: engineering health metrics from git history.
//!
//! The crate is split so that only `git` touches the outside world:
//! `git` (read history) → `analysis` (pure metrics) → `report` (render).

pub mod analysis;
pub mod cli;
pub mod config;
pub mod git;
pub mod model;
pub mod report;
