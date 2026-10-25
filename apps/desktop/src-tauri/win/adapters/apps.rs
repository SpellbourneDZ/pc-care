//! Registration and package metadata only. Never runs MSI/WMI/uninstall strings or loads icons.
#[cfg(not(feature = "qa-fixtures"))]
use super::registry;
#[cfg(not(feature = "qa-fixtures"))]
use crate::domain::startup::registry_string;
use crate::domain::{
    apps::*,
    contracts::{ApiError, ByteCount},
};
#[cfg(any(not(feature = "qa-fixtures"), test))]
use std::io;
#[cfg(not(feature = "qa-fixtures"))]
use std::ptr;
#[cfg(not(feature = "qa-fixtures"))]
use windows_sys::Win32::{Foundation::*, System::Registry::*};
#[cfg(not(feature = "qa-fixtures"))]
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

pub fn collect() -> Result<AppSnapshot, ApiError> {
    let mut snapshot = AppSnapshot::new()?;
    #[cfg(feature = "qa-fixtures")]
    {
        if super::preferences::qa_directory().is_none() {
            return Err(ApiError::InvalidRequest);
        }
        for (i, source) in AppSource::REGISTRY
            .into_iter()
            .chain([AppSource::Msix])
            .enumerate()
        {
            let mut app = blank(
                source,
                &format!("fixture-{i}"),
                "Искусственная программа".into(),
            );
            app.publisher = Some("Тестовый издатель".into());
            app.version = Some("1.2.3".into());
            app.kind = if source == AppSource::Msix {
                AppKind::Msix
            } else if i == 0 {
                AppKind::Msi
            } else {
                AppKind::Win32
            };
            app.system_component = Some(i == 4);
            app.uninstall_block = if i == 4 {
                UninstallBlock::Protected
            } else {
                UninstallBlock::NotValidated
            };
            app.estimated_bytes = if i == 0 {
                Some(ByteCount(123456789))
            } else {
                None
            };
            snapshot.push(app);
        }
    }
    #[cfg(not(feature = "qa-fixtures"))]
    {
        for source in AppSource::REGISTRY {
            registry_apps(&mut snapshot, source)
        }
        if let Err(error) = packages(&mut snapshot) {
            snapshot.issue(
                AppSource::Msix,
                if error.code().0 == 0x80070005u32 as i32 {
                    AppIssueReason::AccessDenied
                } else {
                    AppIssueReason::Unavailable
                },
            );
        }
    }
    Ok(snapshot)
}
fn blank(source: AppSource, key: &str, name: String) -> InstalledApp {
    InstalledApp {
        id: format!("{}:{key}", source.tag()),
        name,
        publisher: None,
        version: None,
        kind: if source == AppSource::Msix {
            AppKind::Msix
        } else {
            AppKind::Win32
        },
        source,
        system_component: None,
        estimated_bytes: None,
        registered_date: None,
        install_location: None,
        registration: key.into(),
        uninstall_block: UninstallBlock::MissingMethod,
    }
}
#[cfg(any(not(feature = "qa-fixtures"), test))]
fn io_reason(error: &io::Error) -> AppIssueReason {
    match error.kind() {
        io::ErrorKind::PermissionDenied => AppIssueReason::AccessDenied,
        io::ErrorKind::InvalidInput => AppIssueReason::UnsafePath,
        io::ErrorKind::InvalidData => AppIssueReason::InvalidValue,
        _ => AppIssueReason::Unavailable,
    }
}
#[cfg(not(feature = "qa-fixtures"))]
fn value(key: HKEY, name: &str) -> io::Result<Option<(u32, Vec<u8>)>> {
    let name: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let (mut kind, mut size) = (0, 0);
    let status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            ptr::null_mut(),
            &mut size,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    if size > 65536 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut data = vec![0u8; size as usize];
    let status = unsafe {
        RegQueryValueExW(
            key,
            name.as_ptr(),
            ptr::null(),
            &mut kind,
            data.as_mut_ptr(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    data.truncate(size as usize);
    Ok(Some((kind, data)))
}
#[cfg(not(feature = "qa-fixtures"))]
fn text(key: HKEY, name: &str) -> io::Result<Option<String>> {
    value(key, name)?
        .map(|(kind, data)| {
            if kind != REG_SZ && kind != REG_EXPAND_SZ {
                return Err(io::ErrorKind::InvalidData.into());
            }
            registry_string(&data).map_err(|_| io::ErrorKind::InvalidData.into())
        })
        .transpose()
        .map(|s| s.filter(|v| !v.is_empty()))
}
#[cfg(not(feature = "qa-fixtures"))]
fn number(key: HKEY, name: &str) -> io::Result<Option<u32>> {
    value(key, name)?
        .map(|(kind, data)| {
            if kind != REG_DWORD {
                return Err(io::ErrorKind::InvalidData.into());
            }
            data.try_into()
                .map(u32::from_le_bytes)
                .map_err(|_| io::ErrorKind::InvalidData.into())
        })
        .transpose()
}
#[cfg(not(feature = "qa-fixtures"))]
fn read_app(key: HKEY, source: AppSource, name: &str) -> io::Result<Option<InstalledApp>> {
    let Some(display) = text(key, "DisplayName")? else {
        return Ok(None);
    };
    let mut app = blank(source, name, display);
    app.publisher = text(key, "Publisher")?;
    app.version = text(key, "DisplayVersion")?;
    app.install_location = text(key, "InstallLocation")?;
    app.registered_date = text(key, "InstallDate")?;
    app.estimated_bytes = number(key, "EstimatedSize")?
        .filter(|n| *n > 0)
        .map(|n| ByteCount(u64::from(n) * 1024));
    app.kind = if number(key, "WindowsInstaller")? == Some(1) {
        AppKind::Msi
    } else {
        AppKind::Win32
    };
    let system = number(key, "SystemComponent")?.is_some_and(|n| n != 0);
    let protected = system || number(key, "NoRemove")?.is_some_and(|n| n != 0);
    app.system_component = Some(system);
    app.uninstall_block = if protected {
        UninstallBlock::Protected
    } else if text(key, "UninstallString")?.is_some() {
        UninstallBlock::NotValidated
    } else {
        UninstallBlock::MissingMethod
    };
    Ok(Some(app))
}
#[cfg(not(feature = "qa-fixtures"))]
fn registry_apps(snapshot: &mut AppSnapshot, source: AppSource) {
    let hive = if matches!(source, AppSource::Hkcu64 | AppSource::Hkcu32) {
        HKEY_CURRENT_USER
    } else {
        HKEY_LOCAL_MACHINE
    };
    let view = if matches!(source, AppSource::Hkcu32 | AppSource::Hklm32) {
        KEY_WOW64_32KEY
    } else {
        KEY_WOW64_64KEY
    };
    let keys = match registry::open(hive, UNINSTALL, view, true) {
        Ok(Some(keys)) => keys,
        Ok(None) => return,
        Err(e) => {
            snapshot.issue(source, io_reason(&e));
            return;
        }
    };
    let Some(parent) = keys.last() else { return };
    let mut name = vec![0u16; 256];
    for index in 0..8192 {
        let mut len = name.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                parent.0,
                index,
                name.as_mut_ptr(),
                &mut len,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            return;
        }
        if status != ERROR_SUCCESS {
            snapshot.issue(source, AppIssueReason::Unavailable);
            return;
        }
        let name = match String::from_utf16(&name[..len as usize]) {
            Ok(name) => name,
            Err(_) => {
                snapshot.issue(source, AppIssueReason::InvalidValue);
                continue;
            }
        };
        // A registry child name is a single component; never accept a path supplied as a name.
        if name.is_empty() || name.contains(['\\', '\0']) {
            snapshot.issue(source, AppIssueReason::UnsafePath);
            continue;
        }
        let guards = match registry::open(parent.0, &name, view, false) {
            Ok(Some(keys)) => keys,
            Ok(None) => {
                snapshot.issue(source, AppIssueReason::Unavailable);
                continue;
            }
            Err(e) => {
                snapshot.issue(source, io_reason(&e));
                continue;
            }
        };
        let Some(key) = guards.last() else { continue };
        match read_app(key.0, source, &name) {
            Ok(Some(app)) => {
                if !snapshot.push(app) {
                    return;
                }
            }
            Ok(None) => {}
            Err(e) => snapshot.issue(source, io_reason(&e)),
        }
    }
    snapshot.catalog.truncated = true;
    snapshot.issue(source, AppIssueReason::LimitReached);
}
#[cfg(not(feature = "qa-fixtures"))]
fn packages(snapshot: &mut AppSnapshot) -> windows::core::Result<()> {
    use windows::{
        ApplicationModel::PackageSignatureKind,
        Management::Deployment::PackageManager,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        core::HSTRING,
    };
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() }
        }
    }
    let _apartment = Apartment;
    let packages = PackageManager::new()?.FindPackagesByUserSecurityId(&HSTRING::new())?;
    let iterator = packages.First()?;
    let mut visited = 0;
    while iterator.HasCurrent()? {
        if visited >= 4096 {
            snapshot.catalog.truncated = true;
            snapshot.issue(AppSource::Msix, AppIssueReason::LimitReached);
            break;
        }
        visited += 1;
        let package = iterator.Current()?;
        let result = (|| -> windows::core::Result<InstalledApp> {
            let identity = package.Id()?;
            let full = identity.FullName()?.to_string();
            let mut app = blank(
                AppSource::Msix,
                &full,
                package
                    .DisplayName()
                    .map(|s| s.to_string())
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(identity.Name()?.to_string()),
            );
            app.publisher = package
                .PublisherDisplayName()
                .map(|s| s.to_string())
                .ok()
                .filter(|s| !s.is_empty());
            let v = identity.Version()?;
            app.version = Some(format!(
                "{}.{}.{}.{}",
                v.Major, v.Minor, v.Build, v.Revision
            ));
            app.system_component = match (
                package.IsFramework(),
                package.IsResourcePackage(),
                package.SignatureKind(),
            ) {
                (Ok(f), Ok(r), Ok(s)) => Some(f || r || s == PackageSignatureKind::System),
                _ => None,
            };
            app.uninstall_block = if app.system_component != Some(false) {
                UninstallBlock::Protected
            } else {
                UninstallBlock::NotValidated
            };
            app.install_location = package
                .InstalledLocation()
                .and_then(|folder| folder.Path())
                .map(|s| s.to_string())
                .ok();
            Ok(app)
        })();
        match result {
            Ok(app) => {
                if !snapshot.push(app) {
                    break;
                }
            }
            Err(_) => snapshot.issue(AppSource::Msix, AppIssueReason::Unavailable),
        }
        iterator.MoveNext()?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "apps_tests.rs"]
mod tests;
