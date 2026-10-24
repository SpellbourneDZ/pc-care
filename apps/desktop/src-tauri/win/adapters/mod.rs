//! Read-only Windows evidence. No mutation or generic registry adapter is exposed.
#[cfg(windows)]
pub mod files;
#[cfg(windows)]
pub mod path;
