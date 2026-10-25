//! Read-only Windows evidence. No mutation or generic registry adapter is exposed.
#[cfg(windows)]
pub mod apps;
pub mod cleanup;
#[cfg(windows)]
pub mod files;
#[cfg(all(windows, not(feature = "qa-fixtures")))]
mod installer_binary;
#[cfg(windows)]
pub mod path;
#[cfg(windows)]
pub mod preferences;
#[cfg(all(windows, not(feature = "qa-fixtures")))]
mod registry;
#[cfg(windows)]
pub mod startup;
#[cfg(windows)]
pub mod storage;
#[cfg(windows)]
pub mod uninstall;
