//! Closed source registry queries and metadata-only Startup inventory. No write rights or commands.
use super::{
    files::{self, Exclusions},
    path::LocalPath,
    storage,
};
use crate::domain::{contracts::*, startup::*};
#[cfg(not(feature = "qa-fixtures"))]
use std::ptr;
use windows_sys::Win32::System::Registry::*;
#[cfg(not(feature = "qa-fixtures"))]
use windows_sys::Win32::{
    Foundation::*,
    UI::Shell::{FOLDERID_CommonStartup, FOLDERID_Startup},
};
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn collect(exclusions: &[String]) -> Result<StartupSnapshot, ApiError> {
    let mut snapshot = StartupSnapshot::new(cfg!(feature = "qa-fixtures"))?;
    let mut excluded = Exclusions::default();
    for path in exclusions {
        excluded.add(path).map_err(|_| ApiError::SettingsInvalid)?;
    }
    for source in StartupSource::ALL {
        match source {
            StartupSource::UserFolder | StartupSource::CommonFolder => match folder(source) {
                Ok(path) => folder_entries(&mut snapshot, source, &path, &excluded),
                Err(reason) => snapshot.issue(source, reason),
            },
            _ => registry_entries(&mut snapshot, source),
        }
    }
    Ok(snapshot)
}
fn folder(source: StartupSource) -> Result<LocalPath, StartupIssueReason> {
    #[cfg(not(feature = "qa-fixtures"))]
    {
        files::known_folder(if source == StartupSource::UserFolder {
            &FOLDERID_Startup
        } else {
            &FOLDERID_CommonStartup
        })
        .map_err(|_| StartupIssueReason::UnsafePath)
    }
    #[cfg(feature = "qa-fixtures")]
    {
        let root = super::preferences::qa_directory()
            .and_then(|p| p.parent().map(|p| p.join("Profile")))
            .ok_or(StartupIssueReason::UnsafePath)?;
        LocalPath::parse(
            root.join(if source == StartupSource::UserFolder {
                "StartupUser"
            } else {
                "StartupCommon"
            })
            .to_str()
            .ok_or(StartupIssueReason::UnsafePath)?,
        )
        .map_err(|_| StartupIssueReason::UnsafePath)
    }
}
fn entry(
    source: StartupSource,
    name: String,
    path: String,
    command: Option<String>,
) -> StartupEntry {
    StartupEntry {
        id: 0,
        name,
        source,
        scope: source.scope(),
        path,
        command,
        publisher: None,
        enabled: None,
        can_toggle: false,
        can_reveal: false,
        location: None,
    }
}
fn folder_entries(
    snapshot: &mut StartupSnapshot,
    source: StartupSource,
    path: &LocalPath,
    excluded: &Exclusions,
) {
    if excluded.contains(path) {
        snapshot.issue(source, StartupIssueReason::UnsafePath);
        return;
    }
    let entries = match storage::readonly_directory(path) {
        Ok(entries) => entries,
        Err(crate::domain::policy::Denial::NotPresent) => return,
        Err(reason) => {
            snapshot.issue(source, denial(reason));
            return;
        }
    };
    for (index, value) in entries.take(4097).enumerate() {
        if index == 4096 {
            snapshot.catalog.truncated = true;
            snapshot.issue(source, StartupIssueReason::LimitReached);
            break;
        }
        let value = match value {
            Ok(value) => value,
            Err(error) => {
                snapshot.issue(source, denial(files::denial_from_io(error)));
                continue;
            }
        };
        let Some(name) = value.file_name().to_str().map(str::to_owned) else {
            snapshot.issue(source, StartupIssueReason::InvalidValue);
            continue;
        };
        if name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }
        let local = match path.child(&name) {
            Ok(path) => path,
            Err(_) => {
                snapshot.issue(source, StartupIssueReason::UnsafePath);
                continue;
            }
        };
        if excluded.contains(&local) {
            snapshot.issue(source, StartupIssueReason::UnsafePath);
            continue;
        }
        let metadata = match storage::metadata_node(&local) {
            Ok(metadata) => metadata,
            Err(reason) => {
                snapshot.issue(source, denial(reason));
                continue;
            }
        };
        let mut record = entry(source, name, local.as_str().into(), None);
        record.can_reveal = true;
        record.location = Some(metadata);
        if !snapshot.push(record) {
            break;
        }
    }
}
fn denial(reason: crate::domain::policy::Denial) -> StartupIssueReason {
    use crate::domain::policy::Denial;
    match reason {
        Denial::AccessDenied => StartupIssueReason::AccessDenied,
        Denial::ReparsePoint
        | Denial::CloudPlaceholder
        | Denial::OutsideScope
        | Denial::UnsupportedPath
        | Denial::HardLink => StartupIssueReason::UnsafePath,
        _ => StartupIssueReason::Unavailable,
    }
}
#[cfg(not(feature = "qa-fixtures"))]
fn run_key(hive: HKEY, view: u32) -> Result<Option<Vec<super::registry::Key>>, StartupIssueReason> {
    super::registry::open(hive, RUN, view, false).map_err(|error| match error.kind() {
        std::io::ErrorKind::PermissionDenied => StartupIssueReason::AccessDenied,
        std::io::ErrorKind::InvalidInput => StartupIssueReason::UnsafePath,
        _ => StartupIssueReason::Unavailable,
    })
}

fn registry_entries(snapshot: &mut StartupSnapshot, source: StartupSource) {
    #[cfg(feature = "qa-fixtures")]
    {
        let data: Vec<u8> = r#""C:\PC Care fixture\test.exe" --example "two words""#
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        registry_value(
            snapshot,
            source,
            "Искусственная запись".into(),
            REG_SZ,
            &data,
        );
    }
    #[cfg(not(feature = "qa-fixtures"))]
    {
        let (hive, view) = match source {
            StartupSource::HkcuRun64 => (HKEY_CURRENT_USER, KEY_WOW64_64KEY),
            StartupSource::HkcuRun32 => (HKEY_CURRENT_USER, KEY_WOW64_32KEY),
            StartupSource::HklmRun64 => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            StartupSource::HklmRun32 => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
            _ => return,
        };
        let keys = match run_key(hive, view) {
            Ok(Some(keys)) => keys,
            Ok(None) => return,
            Err(reason) => {
                snapshot.issue(source, reason);
                return;
            }
        };
        let Some(key) = keys.last() else {
            snapshot.issue(source, StartupIssueReason::Unavailable);
            return;
        };
        let mut name = vec![0u16; 16384];
        let mut data = vec![0u8; 65536];
        for index in 0..4096 {
            let (mut name_len, mut data_len, mut kind) = (name.len() as u32, data.len() as u32, 0);
            let result = unsafe {
                RegEnumValueW(
                    key.0,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    ptr::null(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut data_len,
                )
            };
            if result == ERROR_NO_MORE_ITEMS {
                return;
            }
            if result != ERROR_SUCCESS {
                snapshot.issue(
                    source,
                    if result == ERROR_MORE_DATA {
                        StartupIssueReason::LimitReached
                    } else {
                        StartupIssueReason::Unavailable
                    },
                );
                if result == ERROR_MORE_DATA {
                    snapshot.catalog.truncated = true;
                    continue;
                } else {
                    return;
                }
            }
            let name = match String::from_utf16(&name[..name_len as usize]) {
                Ok(name) => name,
                Err(_) => {
                    snapshot.issue(source, StartupIssueReason::InvalidValue);
                    continue;
                }
            };
            if !registry_value(snapshot, source, name, kind, &data[..data_len as usize]) {
                return;
            }
        }
        snapshot.catalog.truncated = true;
        snapshot.issue(source, StartupIssueReason::LimitReached);
    }
}
fn registry_value(
    snapshot: &mut StartupSnapshot,
    source: StartupSource,
    name: String,
    kind: u32,
    data: &[u8],
) -> bool {
    if kind != REG_SZ && kind != REG_EXPAND_SZ {
        snapshot.issue(source, StartupIssueReason::UnsupportedType);
        return true;
    }
    let command = match registry_string(data) {
        Ok(command) => command,
        Err(reason) => {
            snapshot.issue(source, reason);
            return true;
        }
    };
    let hive = if source.scope() == StartupScope::User {
        "HKEY_CURRENT_USER"
    } else {
        "HKEY_LOCAL_MACHINE"
    };
    snapshot.push(entry(source, name, format!("{hive}\\{RUN}"), Some(command)))
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
