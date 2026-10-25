//! Windows Installer queries only. No install/configure/launch/repair entry point.
use crate::domain::{apps::InstalledApp, contracts::ScanId, uninstall::*};

pub fn check(snapshot_id: ScanId, app: &InstalledApp) -> UninstallCheck {
    let result = assess(app);
    UninstallCheck {
        snapshot_id,
        app_id: app.id.clone(),
        status: result
            .as_ref()
            .map_or_else(|s| *s, |_| UninstallCheckStatus::MsiMatched),
        preview: result.ok(),
        execution_enabled: false,
    }
}
#[cfg(not(feature = "qa-fixtures"))]
fn assess(app: &InstalledApp) -> Result<MsiUninstallPreview, UninstallCheckStatus> {
    let code = product_code(app)?;
    super::apps::revalidate(app)?;
    let mut products = Vec::new();
    for context in [
        MsiContext::Machine,
        MsiContext::UserManaged,
        MsiContext::UserUnmanaged,
    ] {
        if let Some(product) = read_product(&code, context)? {
            products.push(product)
        }
    }
    let preview = msi_preview(app, &products)?;
    super::apps::revalidate(app)?;
    Ok(preview)
}
#[cfg(feature = "qa-fixtures")]
fn assess(app: &InstalledApp) -> Result<MsiUninstallPreview, UninstallCheckStatus> {
    if super::preferences::qa_directory().is_none() {
        return Err(UninstallCheckStatus::Unavailable);
    }
    product_code(app)?;
    // QA never queries a real installed product, even if a fixture GUID happens to match it.
    msi_preview(
        app,
        &[MsiProduct {
            context: MsiContext::UserUnmanaged,
            state: "5".into(),
            name: app.name.clone(),
            publisher: app.publisher.clone().unwrap_or_default(),
            version: app.version.clone().unwrap_or_default(),
        }],
    )
}
#[cfg(not(feature = "qa-fixtures"))]
fn property(
    code: &[u16],
    context: MsiContext,
    name: windows_sys::core::PCWSTR,
) -> Result<Option<String>, UninstallCheckStatus> {
    use std::ptr;
    use windows_sys::Win32::{Foundation::*, System::ApplicationInstallationAndServicing::*};
    let context = match context {
        MsiContext::Machine => MSIINSTALLCONTEXT_MACHINE,
        MsiContext::UserManaged => MSIINSTALLCONTEXT_USERMANAGED,
        MsiContext::UserUnmanaged => MSIINSTALLCONTEXT_USERUNMANAGED,
    };
    // NULL SID queries only the current user (or the per-machine context), never another SID.
    let mut length = 0;
    let status = unsafe {
        MsiGetProductInfoExW(
            code.as_ptr(),
            ptr::null(),
            context,
            name,
            ptr::null_mut(),
            &mut length,
        )
    };
    if status == ERROR_UNKNOWN_PRODUCT {
        return Ok(None);
    }
    if status != ERROR_SUCCESS && status != ERROR_MORE_DATA {
        return Err(msi_error(status));
    }
    if length > 32768 {
        return Err(UninstallCheckStatus::Unavailable);
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let mut capacity = buffer.len() as u32;
    let status = unsafe {
        MsiGetProductInfoExW(
            code.as_ptr(),
            ptr::null(),
            context,
            name,
            buffer.as_mut_ptr(),
            &mut capacity,
        )
    };
    if status == ERROR_UNKNOWN_PRODUCT {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(msi_error(status));
    }
    if capacity as usize >= buffer.len() {
        return Err(UninstallCheckStatus::Unavailable);
    }
    String::from_utf16(&buffer[..capacity as usize])
        .map(Some)
        .map_err(|_| UninstallCheckStatus::Unavailable)
}
#[cfg(not(feature = "qa-fixtures"))]
fn msi_error(status: u32) -> UninstallCheckStatus {
    if status == windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED {
        UninstallCheckStatus::AccessDenied
    } else {
        UninstallCheckStatus::Unavailable
    }
}
#[cfg(not(feature = "qa-fixtures"))]
fn read_product(
    code: &str,
    context: MsiContext,
) -> Result<Option<MsiProduct>, UninstallCheckStatus> {
    use windows_sys::Win32::System::ApplicationInstallationAndServicing::*;
    let code: Vec<u16> = code.encode_utf16().chain([0]).collect();
    let Some(state) = property(&code, context, INSTALLPROPERTY_PRODUCTSTATE)? else {
        return Ok(None);
    };
    if state != "5" {
        return Ok(Some(MsiProduct {
            context,
            state,
            name: String::new(),
            publisher: String::new(),
            version: String::new(),
        }));
    }
    let mut product = MsiProduct {
        context,
        state,
        name: property(&code, context, INSTALLPROPERTY_INSTALLEDPRODUCTNAME)?
            .ok_or(UninstallCheckStatus::TargetChanged)?,
        publisher: property(&code, context, INSTALLPROPERTY_PUBLISHER)?
            .ok_or(UninstallCheckStatus::TargetChanged)?,
        version: property(&code, context, INSTALLPROPERTY_VERSIONSTRING)?
            .ok_or(UninstallCheckStatus::TargetChanged)?,
    };
    product.state = property(&code, context, INSTALLPROPERTY_PRODUCTSTATE)?
        .ok_or(UninstallCheckStatus::TargetChanged)?;
    Ok(Some(product))
}
#[cfg(all(test, not(feature = "qa-fixtures")))]
mod tests {
    use super::*;
    #[test]
    fn current_registrations_are_assessed_without_enabling_execution_or_logging_identities() {
        let snapshot = crate::win::adapters::apps::collect().unwrap();
        let (mut checked, mut matched) = (0, 0);
        for app in snapshot
            .entries
            .iter()
            .filter(|a| product_code(a).is_ok())
            .take(8)
        {
            let result = check(snapshot.catalog.id, app);
            assert!(!result.execution_enabled);
            assert_eq!(
                result.preview.is_some(),
                result.status == UninstallCheckStatus::MsiMatched
            );
            if let Some(preview) = result.preview {
                assert_eq!(preview.arguments.len(), 3);
                assert_eq!(preview.arguments[1], product_code(app).unwrap());
                matched += 1;
            }
            checked += 1;
        }
        eprintln!("MSI read-only calibration: checked={checked}, matched={matched}; no launch");
    }
    #[test]
    fn nonexistent_product_query_is_read_only_and_has_no_launch_fallback() {
        let code = format!(
            "{{{}}}",
            uuid::Uuid::new_v4().hyphenated().to_string().to_uppercase()
        );
        assert!(
            read_product(&code, MsiContext::UserUnmanaged)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            msi_error(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED),
            UninstallCheckStatus::AccessDenied
        );
    }
}
