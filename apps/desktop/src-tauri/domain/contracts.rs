use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

macro_rules! id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
        #[serde(transparent)]
        pub struct $name(#[ts(type = "string")] pub Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}
id!(ScanTaskId);
id!(ScanId);
id!(CandidateId);
id!(PlanId);
id!(SettingsRevision);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Ru,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub theme: Theme,
    pub language: Language,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SettingsSnapshot {
    pub schema_version: u32,
    pub revision: Option<SettingsRevision>,
    pub preferences: Preferences,
}

// JSON decimal strings preserve file sizes above JavaScript's safe integer range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(transparent)]
pub struct ByteCount(
    #[serde(with = "decimal")]
    #[ts(type = "string")]
    pub u64,
);

mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    Safe,
    Review,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ScanKind {
    Mock,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MockScenario {
    Complete,
    AdapterError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ApiError {
    InvalidRequest,
    NotFound,
    NotReady,
    Unsupported,
    Capacity,
    Protected,
    ConfirmationRequired,
    PlanExpired,
    AlreadyExecuted,
    TargetChanged,
    Timeout,
    AdapterError,
    Disconnected,
    Internal,
    Transport,
    SettingsInvalid,
    SettingsUnavailable,
    SettingsConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: CandidateId,
    pub source: ScanKind,
    pub source_key: String,
    pub display_path: String,
    pub identity: String,
    pub rule_id: String,
    pub bytes: ByteCount,
    pub logical_bytes: ByteCount,
    pub safety: SafetyClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScanSnapshot {
    pub id: ScanId,
    pub task_id: ScanTaskId,
    pub source: ScanKind,
    pub candidates: Vec<Candidate>,
    pub issues: Vec<ScanIssue>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct TaskProgress {
    pub task_id: ScanTaskId,
    pub status: TaskStatus,
    pub processed: u32,
    pub total: Option<u32>,
    pub scan_id: Option<ScanId>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CandidatePage {
    pub scan_id: ScanId,
    pub offset: u32,
    pub total: u32,
    pub candidates: Vec<Candidate>,
    pub issues: Vec<ScanIssue>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScanIssue {
    pub rule_id: String,
    pub reason: super::policy::Denial,
    pub count: u32,
    pub sample_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CleanupRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub safety: SafetyClass,
    pub min_age_hours: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PlanWarning {
    SimulationOnly,
    ReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OperationPlan {
    pub id: PlanId,
    pub scan_id: ScanId,
    pub simulated: bool,
    pub targets: Vec<Candidate>,
    pub planned_bytes: ByteCount,
    pub expires_in_ms: u32,
    pub warnings: Vec<PlanWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Success,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ItemResult {
    pub candidate_id: CandidateId,
    pub status: ItemStatus,
    pub error: Option<ApiError>,
    pub actual_reclaimed_bytes: ByteCount,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub plan_id: PlanId,
    pub simulated: bool,
    pub items: Vec<ItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartScanRequest {
    pub kind: ScanKind,
    pub scenario: MockScenario,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApiRequest {
    GetCleanupRules,
    GetPreferences,
    SavePreferences {
        expected_revision: Option<SettingsRevision>,
        preferences: Preferences,
    },
    StartScan(StartScanRequest),
    GetTask {
        task_id: ScanTaskId,
    },
    CancelTask {
        task_id: ScanTaskId,
    },
    GetScanResult {
        scan_id: ScanId,
        offset: u32,
        limit: u32,
    },
    BuildOperationPlan {
        scan_id: ScanId,
        selected_candidate_ids: Vec<CandidateId>,
    },
    ExecutePlan {
        plan_id: PlanId,
        explicit_confirmation: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApiResponse {
    CleanupRules(Vec<CleanupRule>),
    Preferences(SettingsSnapshot),
    ScanStarted { task_id: ScanTaskId },
    Task(TaskProgress),
    ScanResult(CandidatePage),
    OperationPlan(OperationPlan),
    OperationResult(OperationResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApiReply {
    Ok { value: ApiResponse },
    Error { error: ApiError },
}
impl From<Result<ApiResponse, ApiError>> for ApiReply {
    fn from(value: Result<ApiResponse, ApiError>) -> Self {
        match value {
            Ok(value) => Self::Ok { value },
            Err(error) => Self::Error { error },
        }
    }
}
