//! Read-only MSI preparation. A preview is not an executable operation plan.
use super::{apps::*, contracts::ScanId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MsiContext {
    Machine,
    UserManaged,
    UserUnmanaged,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UninstallCheckStatus {
    MsiMatched,
    Unsupported,
    Protected,
    InvalidIdentity,
    TargetChanged,
    NotInstalled,
    AmbiguousContext,
    MetadataMismatch,
    AccessDenied,
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum InstallerBinaryStatus {
    MicrosoftSignatureOffline,
    UntrustedSignature,
    UnsafePath,
    TargetChanged,
    Unavailable,
    Fixture,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MsiUninstallPreview {
    pub product_code: String,
    pub context: MsiContext,
    pub arguments: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct UninstallCheck {
    pub snapshot_id: ScanId,
    pub app_id: String,
    pub status: UninstallCheckStatus,
    pub preview: Option<MsiUninstallPreview>,
    pub execution_enabled: bool,
    pub binary: Option<InstallerBinaryStatus>,
}
pub struct MsiProduct {
    pub context: MsiContext,
    pub state: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
}
pub fn product_code(app: &InstalledApp) -> Result<String, UninstallCheckStatus> {
    if app.kind != AppKind::Msi || app.source == AppSource::Msix {
        return Err(UninstallCheckStatus::Unsupported);
    }
    if app.system_component != Some(false) || app.uninstall_block == UninstallBlock::Protected {
        return Err(UninstallCheckStatus::Protected);
    }
    let bytes = app.registration.as_bytes();
    if bytes.len() != 38
        || bytes[0] != b'{'
        || bytes[37] != b'}'
        || !bytes.is_ascii()
        || ![9, 14, 19, 24].iter().all(|i| bytes[*i] == b'-')
    {
        return Err(UninstallCheckStatus::InvalidIdentity);
    }
    let id = uuid::Uuid::parse_str(&app.registration[1..37])
        .map_err(|_| UninstallCheckStatus::InvalidIdentity)?;
    if id.is_nil() {
        return Err(UninstallCheckStatus::InvalidIdentity);
    }
    Ok(format!(
        "{{{}}}",
        id.hyphenated().to_string().to_uppercase()
    ))
}
pub fn msi_preview(
    app: &InstalledApp,
    products: &[MsiProduct],
) -> Result<MsiUninstallPreview, UninstallCheckStatus> {
    let code = product_code(app)?;
    if products.is_empty() {
        return Err(UninstallCheckStatus::NotInstalled);
    }
    if products.len() != 1 {
        return Err(UninstallCheckStatus::AmbiguousContext);
    }
    let p = &products[0];
    if p.state != "5" {
        return Err(UninstallCheckStatus::NotInstalled);
    }
    let machine = matches!(app.source, AppSource::Hklm32 | AppSource::Hklm64);
    if machine != (p.context == MsiContext::Machine) {
        return Err(UninstallCheckStatus::AmbiguousContext);
    }
    if app.name != p.name
        || app.publisher.as_deref() != Some(p.publisher.as_str())
        || app.version.as_deref() != Some(p.version.as_str())
        || p.publisher.is_empty()
        || p.version.is_empty()
    {
        return Err(UninstallCheckStatus::MetadataMismatch);
    }
    Ok(MsiUninstallPreview {
        product_code: code.clone(),
        context: p.context,
        arguments: vec!["/x".into(), code, "/norestart".into()],
    })
}
#[cfg(test)]
#[path = "uninstall_tests.rs"]
mod tests;
