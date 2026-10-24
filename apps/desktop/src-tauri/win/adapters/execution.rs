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
        recycle: impl FnMut(&Candidate, &AtomicBool) -> Result<RecycleReceipt, ExecutionFailure>
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
                return Ok(ExecutionSnapshot {
                    record: previous,
                    cancel_requested: false,
                    journal_error: false,
                });
            }
            Err(ApiError::NotFound) => {}
            Err(error) => return Err(error),
        }
        session.create(&record)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let snapshot = ExecutionSnapshot {
            record,
            cancel_requested: false,
            journal_error: false,
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
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execution::run(
                        &plan,
                        &cancelled,
                        |record| {
                            session.replace(record)?;
                            let mut active = shared.lock().map_err(|_| ApiError::Internal)?;
                            if record.phase == execution::ExecutionPhase::Running {
                                active.as_mut().ok_or(ApiError::Internal)?.snapshot.record =
                                    record.clone();
                            }
                            Ok(())
                        },
                        recycle,
                    )
                }))
                .unwrap_or(Err(ApiError::Internal));
                if let Ok(mut active) = shared.lock()
                    && let Some(active) = active.as_mut()
                {
                    if result.is_err() {
                        active.snapshot.record.recover();
                        active.snapshot.journal_error =
                            session.replace(&active.snapshot.record).is_err();
                    } else if let Ok(record) = result {
                        active.snapshot.record = record;
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
        Ok(ExecutionSnapshot {
            record,
            cancel_requested: false,
            journal_error: false,
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
