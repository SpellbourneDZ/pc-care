//! One background batch, with disk intent/outcomes and no automatic replay.
use super::execution_journal::JournalSession;
use crate::domain::{
    contracts::*,
    execution::{self, ExecutionFailure, ExecutionRecord, RecycleReceipt},
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

#[derive(Clone)]
pub struct ExecutionManager {
    root: Option<PathBuf>,
    active: Arc<Mutex<Option<Active>>>,
}
struct Active {
    snapshot: ExecutionSnapshot,
    cancelled: Arc<AtomicBool>,
    finished: bool,
}
#[derive(Debug, Clone)]
pub struct ExecutionSnapshot {
    pub record: ExecutionRecord,
    pub cancel_requested: bool,
    pub journal_error: bool,
    pub recovery_pending: bool,
}

impl ExecutionManager {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            active: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(
        &self,
        plan: OperationPlan,
        mut recycle: impl FnMut(
            &Candidate,
            &AtomicBool,
            &JournalSession,
        ) -> Result<RecycleReceipt, ExecutionFailure>
        + Send
        + 'static,
    ) -> Result<ExecutionSnapshot, ApiError> {
        let record = ExecutionRecord::new(&plan)?;
        let mut active = self.active.lock().map_err(|_| ApiError::Internal)?;
        if let Some(current) = active.as_ref() {
            if current.snapshot.record.plan_id == plan.id {
                return Ok(current.snapshot.clone());
            }
            if !current.finished {
                return Err(ApiError::Capacity);
            }
        }
        let root = self.root.as_ref().ok_or(ApiError::AdapterError)?;
        let session = JournalSession::open(root, true)?;
        match session.read(plan.id) {
            Ok(mut previous) => {
                previous.recover();
                session.replace(&previous)?;
                session.settle_recovery(plan.id)?;
                return Ok(ExecutionSnapshot {
                    record: previous,
                    cancel_requested: false,
                    journal_error: false,
                    recovery_pending: session.recovery_pending(plan.id)?,
                });
            }
            Err(ApiError::NotFound) => {}
            Err(error) => return Err(error),
        }
        if !session.pending_recoveries()?.is_empty() {
            return Err(ApiError::Capacity);
        }
        session.create(&record)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let snapshot = ExecutionSnapshot {
            record,
            cancel_requested: false,
            journal_error: false,
            recovery_pending: false,
        };
        *active = Some(Active {
            snapshot: snapshot.clone(),
            cancelled: cancelled.clone(),
            finished: false,
        });
        let shared = self.active.clone();
        let spawned = thread::Builder::new()
            .name("pc-care-execute".into())
            .spawn(move || {
                let mut journal_failed = false;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execution::run(
                        &plan,
                        &cancelled,
                        |record| {
                            journal_failed = true;
                            session.replace(record)?;
                            // Keep the durable receipt even if settling its recovery map fails.
                            {
                                let mut active = shared.lock().map_err(|_| ApiError::Internal)?;
                                if record.phase == execution::ExecutionPhase::Running {
                                    active.as_mut().ok_or(ApiError::Internal)?.snapshot.record =
                                        record.clone();
                                }
                            }
                            session.settle_recovery(plan.id)?;
                            let pending = session.recovery_pending(plan.id)?;
                            shared
                                .lock()
                                .map_err(|_| ApiError::Internal)?
                                .as_mut()
                                .ok_or(ApiError::Internal)?
                                .snapshot
                                .recovery_pending = pending;
                            journal_failed = false;
                            Ok(())
                        },
                        |candidate, cancel| {
                            let outcome = recycle(candidate, cancel, &session);
                            if outcome.is_err() && session.recovery_pending(plan.id).unwrap_or(true)
                            {
                                return Err(ExecutionFailure::OutcomeUnknown);
                            }
                            outcome
                        },
                    )
                }))
                .unwrap_or(Err(ApiError::Internal));
                if let Ok(mut active) = shared.lock()
                    && let Some(active) = active.as_mut()
                {
                    if result.is_err() {
                        active.snapshot.record.recover();
                        let recovery_failed = session.replace(&active.snapshot.record).is_err();
                        active.snapshot.journal_error = journal_failed || recovery_failed;
                    } else if let Ok(record) = result {
                        active.snapshot.record = record;
                    }
                    match session.recovery_pending(plan.id) {
                        Ok(pending) => active.snapshot.recovery_pending = pending,
                        Err(_) => {
                            active.snapshot.recovery_pending = true;
                            active.snapshot.journal_error = true;
                        }
                    }
                    drop(session);
                    active.finished = true;
                }
            });
        if spawned.is_err() {
            let current = active.as_mut().ok_or(ApiError::Internal)?;
            current.snapshot.record.recover();
            current.finished = true;
            current.snapshot.journal_error = JournalSession::open(root, false)
                .and_then(|session| session.replace(&current.snapshot.record))
                .is_err();
            return Err(ApiError::AdapterError);
        }
        Ok(snapshot)
    }

    pub fn get(&self, id: PlanId) -> Result<ExecutionSnapshot, ApiError> {
        let active = self.active.lock().map_err(|_| ApiError::Internal)?;
        if let Some(current) = active.as_ref()
            && current.snapshot.record.plan_id == id
        {
            return Ok(current.snapshot.clone());
        }
        let session =
            JournalSession::open(self.root.as_ref().ok_or(ApiError::AdapterError)?, false)?;
        let mut record = session.read(id)?;
        if record.phase == execution::ExecutionPhase::Running {
            record.recover();
            session.replace(&record)?;
        }
        session.settle_recovery(id)?;
        Ok(ExecutionSnapshot {
            record,
            cancel_requested: false,
            journal_error: false,
            recovery_pending: session.recovery_pending(id)?,
        })
    }

    pub fn cancel(&self, id: PlanId) -> Result<ExecutionSnapshot, ApiError> {
        {
            let mut active = self.active.lock().map_err(|_| ApiError::Internal)?;
            if let Some(current) = active.as_mut()
                && current.snapshot.record.plan_id == id
            {
                if !current.finished {
                    current.cancelled.store(true, Ordering::Release);
                    current.snapshot.cancel_requested = true;
                }
                return Ok(current.snapshot.clone());
            }
        }
        self.get(id)
    }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
