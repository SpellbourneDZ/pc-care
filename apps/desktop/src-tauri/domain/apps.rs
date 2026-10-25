//! Registered app inventory is evidence, not authority to execute its uninstall command.
use super::contracts::{ApiError, ByteCount, ScanId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AppSource {
    Hkcu64,
    Hkcu32,
    Hklm64,
    Hklm32,
    Msix,
}
impl AppSource {
    pub const REGISTRY: [Self; 4] = [Self::Hkcu64, Self::Hkcu32, Self::Hklm64, Self::Hklm32];
    pub fn tag(self) -> &'static str {
        match self {
            Self::Hkcu64 => "hkcu64",
            Self::Hkcu32 => "hkcu32",
            Self::Hklm64 => "hklm64",
            Self::Hklm32 => "hklm32",
            Self::Msix => "msix",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AppKind {
    Win32,
    Msi,
    Msix,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UninstallBlock {
    NotValidated,
    MissingMethod,
    Protected,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AppIssueReason {
    AccessDenied,
    Unavailable,
    InvalidValue,
    UnsafePath,
    LimitReached,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AppIssue {
    pub source: AppSource,
    pub reason: AppIssueReason,
    pub count: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub kind: AppKind,
    pub source: AppSource,
    pub system_component: Option<bool>,
    pub estimated_bytes: Option<ByteCount>,
    pub registered_date: Option<String>,
    pub install_location: Option<String>,
    pub registration: String,
    pub uninstall_block: UninstallBlock,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AppCatalog {
    pub id: ScanId,
    pub total: u32,
    pub issues: Vec<AppIssue>,
    pub truncated: bool,
    pub fixture_mode: bool,
    #[ts(type = "number")]
    pub collected_ms: u64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AppSort {
    Name,
    Publisher,
    Size,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AppQuery {
    pub search: String,
    pub kind: Option<AppKind>,
    pub source: Option<AppSource>,
    pub sort: AppSort,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AppPage {
    pub snapshot_id: ScanId,
    pub total: u32,
    pub offset: u32,
    pub entries: Vec<InstalledApp>,
}
pub struct AppSnapshot {
    pub catalog: AppCatalog,
    pub entries: Vec<InstalledApp>,
    bytes: usize,
}
impl AppSnapshot {
    pub fn new() -> Result<Self, ApiError> {
        Ok(Self {
            catalog: AppCatalog {
                id: ScanId::new(),
                total: 0,
                issues: vec![],
                truncated: false,
                fixture_mode: cfg!(feature = "qa-fixtures"),
                collected_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| ApiError::Internal)?
                    .as_millis() as u64,
            },
            entries: vec![],
            bytes: 0,
        })
    }
    pub fn issue(&mut self, source: AppSource, reason: AppIssueReason) {
        if let Some(issue) = self
            .catalog
            .issues
            .iter_mut()
            .find(|i| i.source == source && i.reason == reason)
        {
            issue.count = issue.count.saturating_add(1)
        } else {
            self.catalog.issues.push(AppIssue {
                source,
                reason,
                count: 1,
            })
        }
    }
    pub fn push(&mut self, entry: InstalledApp) -> bool {
        if self.entries.iter().any(|e| e.id == entry.id) {
            return true;
        }
        let bytes = std::mem::size_of::<InstalledApp>()
            + entry.id.len()
            + entry.name.len()
            + entry.registration.len()
            + [
                &entry.publisher,
                &entry.version,
                &entry.registered_date,
                &entry.install_location,
            ]
            .iter()
            .map(|s| s.as_ref().map_or(0, String::len))
            .sum::<usize>();
        if self.entries.len() >= 4096 || self.bytes + bytes > 16 * 1024 * 1024 {
            self.catalog.truncated = true;
            self.issue(entry.source, AppIssueReason::LimitReached);
            return false;
        }
        self.bytes += bytes;
        self.entries.push(entry);
        self.catalog.total = self.entries.len() as u32;
        true
    }
    pub fn page(&self, q: AppQuery) -> Result<AppPage, ApiError> {
        if !(1..=500).contains(&q.limit) || q.search.len() > 1024 {
            return Err(ApiError::InvalidRequest);
        }
        let search = q.search.to_lowercase();
        let mut rows: Vec<_> = self
            .entries
            .iter()
            .filter(|e| {
                q.kind.is_none_or(|k| e.kind == k)
                    && q.source.is_none_or(|s| e.source == s)
                    && [
                        Some(e.name.as_str()),
                        e.publisher.as_deref(),
                        e.version.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|s| s.to_lowercase().contains(&search))
            })
            .collect();
        rows.sort_unstable_by(|a, b| {
            match q.sort {
                AppSort::Name => a.name.cmp(&b.name),
                AppSort::Publisher => a.publisher.cmp(&b.publisher),
                AppSort::Size => b
                    .estimated_bytes
                    .map(|v| v.0)
                    .cmp(&a.estimated_bytes.map(|v| v.0)),
            }
            .then(a.id.cmp(&b.id))
        });
        Ok(AppPage {
            snapshot_id: self.catalog.id,
            total: rows.len() as u32,
            offset: q.offset,
            entries: rows
                .into_iter()
                .skip(q.offset as usize)
                .take(q.limit as usize)
                .cloned()
                .collect(),
        })
    }
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod tests;
