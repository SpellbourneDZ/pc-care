//! Read-only Windows evidence. No mutation or generic registry adapter is exposed.
pub mod cleanup;
#[cfg(windows)]
pub mod files;
#[cfg(windows)]
pub mod path;
#[cfg(windows)]
pub mod preferences;
#[cfg(windows)]
pub mod storage;
