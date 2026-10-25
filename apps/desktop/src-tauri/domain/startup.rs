//! Read-only source inventory. Registration is not proof that Windows will execute it.
use super::{
    contracts::{ApiError, ScanId},
    storage::StorageNode,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_ENTRIES: usize = 2048;
pub const MAX_DATA: usize = 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StartupSource {
    HkcuRun64,
    HkcuRun32,
    HklmRun64,
    HklmRun32,
    UserFolder,
    CommonFolder,
}
impl StartupSource {
    pub const ALL: [Self; 6] = [
        Self::HkcuRun64,
        Self::HkcuRun32,
        Self::HklmRun64,
        Self::HklmRun32,
        Self::UserFolder,
        Self::CommonFolder,
    ];
    pub fn scope(self) -> StartupScope {
        match self {
            Self::HklmRun64 | Self::HklmRun32 | Self::CommonFolder => StartupScope::Machine,
            _ => StartupScope::User,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StartupScope {
    User,
    Machine,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StartupIssueReason {
    AccessDenied,
    Unavailable,
    InvalidValue,
    UnsupportedType,
    LimitReached,
    UnsafePath,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartupIssue {
    pub source: StartupSource,
    pub reason: StartupIssueReason,
    pub count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartupEntry {
    pub id: u32,
    pub name: String,
    pub source: StartupSource,
    pub scope: StartupScope,
    pub path: String,
    pub command: Option<String>,
    pub publisher: Option<String>,
    pub enabled: Option<bool>,
    pub can_toggle: bool,
    pub can_reveal: bool,
    #[serde(skip)]
    #[ts(skip)]
    pub(crate) location: Option<StorageNode>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartupCatalog {
    pub id: ScanId,
    pub total: u32,
    pub issues: Vec<StartupIssue>,
    pub truncated: bool,
    pub fixture_mode: bool,
    #[ts(type = "number")]
    pub collected_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartupQuery {
    pub search: String,
    pub source: Option<StartupSource>,
    pub scope: Option<StartupScope>,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartupPage {
    pub snapshot_id: ScanId,
    pub total: u32,
    pub offset: u32,
    pub entries: Vec<StartupEntry>,
}

pub struct StartupSnapshot {
    pub catalog: StartupCatalog,
    pub entries: Vec<StartupEntry>,
    bytes: usize,
}
impl StartupSnapshot {
    pub fn new(fixture_mode: bool) -> Result<Self, ApiError> {
        Ok(Self {
            catalog: StartupCatalog {
                id: ScanId::new(),
                total: 0,
                issues: Vec::new(),
                truncated: false,
                fixture_mode,
                collected_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| ApiError::Internal)?
                    .as_millis() as u64,
            },
            entries: Vec::new(),
            bytes: 0,
        })
    }
    pub fn issue(&mut self, source: StartupSource, reason: StartupIssueReason) {
        if let Some(issue) = self
            .catalog
            .issues
            .iter_mut()
            .find(|i| i.source == source && i.reason == reason)
        {
            issue.count = issue.count.saturating_add(1)
        } else {
            self.catalog.issues.push(StartupIssue {
                source,
                reason,
                count: 1,
            })
        }
    }
    pub fn push(&mut self, mut entry: StartupEntry) -> bool {
        let bytes = std::mem::size_of::<StartupEntry>()
            + entry.name.len()
            + entry.path.len()
            + entry.command.as_ref().map_or(0, String::len)
            + entry.publisher.as_ref().map_or(0, String::len)
            + entry
                .location
                .as_ref()
                .map_or(0, |n| n.path.len() + n.name.len());
        if self.entries.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_DATA {
            self.catalog.truncated = true;
            self.issue(entry.source, StartupIssueReason::LimitReached);
            return false;
        }
        self.bytes += bytes;
        entry.id = self.entries.len() as u32;
        self.entries.push(entry);
        self.catalog.total = self.entries.len() as u32;
        true
    }
    pub fn page(&self, query: StartupQuery) -> Result<StartupPage, ApiError> {
        if !(1..=500).contains(&query.limit) || query.search.len() > 1024 {
            return Err(ApiError::InvalidRequest);
        }
        let search = query.search.to_lowercase();
        let mut rows: Vec<_> = self
            .entries
            .iter()
            .filter(|e| {
                query.source.is_none_or(|s| e.source == s)
                    && query.scope.is_none_or(|s| e.scope == s)
                    && [
                        Some(e.name.as_str()),
                        Some(e.path.as_str()),
                        e.command.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|text| text.to_lowercase().contains(&search))
            })
            .collect();
        rows.sort_unstable_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(StartupPage {
            snapshot_id: self.catalog.id,
            total: rows.len() as u32,
            offset: query.offset,
            entries: rows
                .into_iter()
                .skip(query.offset as usize)
                .take(query.limit as usize)
                .cloned()
                .collect(),
        })
    }
}

pub fn registry_string(bytes: &[u8]) -> Result<String, StartupIssueReason> {
    if bytes.len() > 65536 || !bytes.len().is_multiple_of(2) {
        return Err(StartupIssueReason::InvalidValue);
    }
    let mut words: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u16::from_le_bytes([v[0], v[1]]))
        .collect();
    if words.last() == Some(&0) {
        words.pop();
    }
    if words.contains(&0) {
        return Err(StartupIssueReason::InvalidValue);
    }
    String::from_utf16(&words).map_err(|_| StartupIssueReason::InvalidValue)
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
