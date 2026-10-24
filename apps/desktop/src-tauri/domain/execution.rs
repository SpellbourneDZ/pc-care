//! Execution intent is persisted before effects; interrupted work is never replayed.
use super::{cleanup::RuleId, contracts::*, engine::MAX_PLAN_ITEMS, policy::Denial};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    Running,
    Completed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionItemStatus {
    Pending,
    Started,
    Success,
    Skipped,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExecutionFailure {
    Policy(Denial),
    Cancelled,
    Interrupted,
    OutcomeUnknown,
    AdapterError,
    Unsupported,
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionItem {
    pub candidate_id: CandidateId,
    pub rule_id: String,
    pub planned_bytes: ByteCount,
    pub status: ExecutionItemStatus,
    pub reason: Option<ExecutionFailure>,
    pub recycled: bool,
    pub actual_reclaimed_bytes: Option<ByteCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRecord {
    pub schema_version: u32,
    pub plan_id: PlanId,
    pub simulated: bool,
    pub phase: ExecutionPhase,
    pub planned_bytes: ByteCount,
    pub items: Vec<ExecutionItem>,
}

impl ExecutionRecord {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.schema_version != 1 {
            return Err(ApiError::Unsupported);
        }
        if self.items.is_empty()
            || self.items.len() > MAX_PLAN_ITEMS
            || self
                .items
                .iter()
                .map(|item| item.candidate_id)
                .collect::<HashSet<_>>()
                .len()
                != self.items.len()
            || self
                .items
                .iter()
                .try_fold(0_u64, |sum, item| sum.checked_add(item.planned_bytes.0))
                != Some(self.planned_bytes.0)
        {
            return Err(ApiError::InvalidRequest);
        }
        for item in &self.items {
            let status_valid = match item.status {
                ExecutionItemStatus::Pending | ExecutionItemStatus::Started => {
                    self.phase == ExecutionPhase::Running
                        && item.reason.is_none()
                        && !item.recycled
                        && item.actual_reclaimed_bytes.is_none()
                }
                ExecutionItemStatus::Success => {
                    !self.simulated
                        && item.recycled
                        && item.reason.is_none()
                        && item.actual_reclaimed_bytes.is_none()
                }
                ExecutionItemStatus::Unknown => {
                    !item.recycled
                        && item.reason == Some(ExecutionFailure::OutcomeUnknown)
                        && item.actual_reclaimed_bytes.is_none()
                }
                ExecutionItemStatus::Skipped => {
                    !item.recycled
                        && match item.reason {
                            Some(ExecutionFailure::DryRun) => {
                                self.simulated && item.actual_reclaimed_bytes == Some(ByteCount(0))
                            }
                            Some(
                                ExecutionFailure::Policy(_)
                                | ExecutionFailure::Cancelled
                                | ExecutionFailure::Interrupted
                                | ExecutionFailure::Unsupported,
                            ) => item.actual_reclaimed_bytes.is_none(),
                            _ => false,
                        }
                }
                ExecutionItemStatus::Error => {
                    !self.simulated
                        && !item.recycled
                        && item.reason == Some(ExecutionFailure::AdapterError)
                        && item.actual_reclaimed_bytes.is_none()
                }
            };
            if !status_valid || RuleId::from_key(&item.rule_id).is_none() {
                return Err(ApiError::InvalidRequest);
            }
        }
        Ok(())
    }
    pub fn new(plan: &OperationPlan) -> Result<Self, ApiError> {
        if plan.targets.is_empty()
            || plan.targets.len() > MAX_PLAN_ITEMS
            || plan
                .targets
                .iter()
                .map(|item| item.id)
                .collect::<HashSet<_>>()
                .len()
                != plan.targets.len()
            || plan.targets.iter().any(|item| {
                item.source != ScanKind::Cleanup
                    || item.safety == SafetyClass::Protected
                    || item.rule_id != item.source_key
                    || RuleId::from_key(&item.rule_id)
                        .is_none_or(|rule| rule.descriptor().safety != item.safety)
            })
        {
            return Err(ApiError::InvalidRequest);
        }
        let total = plan
            .targets
            .iter()
            .try_fold(0_u64, |sum, item| sum.checked_add(item.bytes.0))
            .ok_or(ApiError::InvalidRequest)?;
        if total != plan.planned_bytes.0 {
            return Err(ApiError::InvalidRequest);
        }
        Ok(Self {
            schema_version: 1,
            plan_id: plan.id,
            simulated: plan.simulated,
            phase: ExecutionPhase::Running,
            planned_bytes: plan.planned_bytes,
            items: plan
                .targets
                .iter()
                .map(|item| ExecutionItem {
                    candidate_id: item.id,
                    rule_id: item.rule_id.clone(),
                    planned_bytes: item.bytes,
                    status: ExecutionItemStatus::Pending,
                    reason: None,
                    recycled: false,
                    actual_reclaimed_bytes: None,
                })
                .collect(),
        })
    }

    pub fn recover(&mut self) {
        if self.phase != ExecutionPhase::Running {
            return;
        }
        for item in &mut self.items {
            match item.status {
                ExecutionItemStatus::Started => {
                    item.status = ExecutionItemStatus::Unknown;
                    item.reason = Some(ExecutionFailure::OutcomeUnknown)
                }
                ExecutionItemStatus::Pending => {
                    item.status = ExecutionItemStatus::Skipped;
                    item.reason = Some(ExecutionFailure::Interrupted)
                }
                _ => {}
            }
        }
        self.phase = ExecutionPhase::Interrupted;
    }
}

/// Only the native adapter may construct a receipt after confirmed recycle completion.
pub struct RecycleReceipt;

pub fn run(
    plan: &OperationPlan,
    cancelled: &AtomicBool,
    mut persist: impl FnMut(&ExecutionRecord) -> Result<(), ApiError>,
    mut recycle: impl FnMut(&Candidate, &AtomicBool) -> Result<RecycleReceipt, ExecutionFailure>,
) -> Result<ExecutionRecord, ApiError> {
    let mut record = ExecutionRecord::new(plan)?;
    persist(&record)?;
    for index in 0..record.items.len() {
        if cancelled.load(Ordering::Acquire) {
            for item in &mut record.items[index..] {
                item.status = ExecutionItemStatus::Skipped;
                item.reason = Some(ExecutionFailure::Cancelled);
            }
            record.phase = ExecutionPhase::Cancelled;
            persist(&record)?;
            return Ok(record);
        }
        if plan.simulated {
            let item = &mut record.items[index];
            item.status = ExecutionItemStatus::Skipped;
            item.reason = Some(ExecutionFailure::DryRun);
            item.actual_reclaimed_bytes = Some(ByteCount(0));
        } else {
            record.items[index].status = ExecutionItemStatus::Started;
            persist(&record)?;
            let outcome = recycle(&plan.targets[index], cancelled);
            let item = &mut record.items[index];
            match outcome {
                Ok(RecycleReceipt) => {
                    item.status = ExecutionItemStatus::Success;
                    item.recycled = true;
                }
                Err(reason) => {
                    item.status = match reason {
                        ExecutionFailure::OutcomeUnknown => ExecutionItemStatus::Unknown,
                        ExecutionFailure::AdapterError => ExecutionItemStatus::Error,
                        _ => ExecutionItemStatus::Skipped,
                    };
                    item.reason = Some(reason);
                }
            }
        }
        persist(&record)?;
        if record.items[index].status == ExecutionItemStatus::Unknown {
            record.recover();
            persist(&record)?;
            return Ok(record);
        }
    }
    record.phase = if cancelled.load(Ordering::Acquire) {
        ExecutionPhase::Cancelled
    } else {
        ExecutionPhase::Completed
    };
    persist(&record)?;
    Ok(record)
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
