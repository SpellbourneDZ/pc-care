//! Decisions over backend evidence, never frontend paths or permission grants.
use super::contracts::CandidateId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Denial {
    UnsupportedPath,
    Protected,
    Excluded,
    OutsideScope,
    ReparsePoint,
    HardLink,
    CloudPlaceholder,
    NotRegularFile,
    AccessDenied,
    Locked,
    Unavailable,
    TargetChanged,
    ForeignId,
    TooRecent,
    ApplicationRunning,
    LimitReached,
    NotPresent,
    SensitiveData,
    UnverifiedType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileIdentity {
    pub volume: u64,
    pub id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileMetadata {
    pub identity: FileIdentity,
    pub size: u64,
    pub allocated_size: u64,
    pub created: i64,
    pub modified: i64,
    pub changed: i64,
    pub attributes: u32,
    pub links: u32,
}

pub fn check_identity(
    selected: CandidateId,
    snapshot_id: CandidateId,
    before: &FileMetadata,
    current: &FileMetadata,
) -> Result<(), Denial> {
    if selected != snapshot_id {
        return Err(Denial::ForeignId);
    }
    if before != current {
        return Err(Denial::TargetChanged);
    }
    Ok(())
}
