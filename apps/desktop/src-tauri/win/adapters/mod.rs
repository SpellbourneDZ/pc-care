//! Read-only Windows evidence. No mutation or generic registry adapter is exposed.
pub mod cleanup;
#[cfg(windows)]
pub mod execution;
#[cfg(windows)]
pub mod execution_journal;
#[cfg(windows)]
pub mod files;
#[cfg(windows)]
pub mod path;
#[cfg(windows)]
pub mod preferences;
#[cfg(all(windows, feature = "vm-recycle-probe"))]
pub mod recycle_probe;
