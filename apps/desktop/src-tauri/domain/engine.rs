use super::{contracts::*, mock};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

const MAX_TASKS: usize = 32;
const MAX_PLANS: usize = 32;
const PLAN_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Engine {
    state: Arc<Mutex<State>>,
    mock_enabled: bool,
}
#[derive(Default)]
struct State {
    tasks: HashMap<ScanTaskId, Task>,
    scans: HashMap<ScanId, ScanSnapshot>,
    plans: HashMap<PlanId, StoredPlan>,
}
struct Task {
    progress: TaskProgress,
    started: Instant,
}
struct StoredPlan {
    plan: OperationPlan,
    created: Instant,
    executed: bool,
}

impl Engine {
    pub fn new(mock_enabled: bool) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            mock_enabled,
        }
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>, ApiError> {
        self.state.lock().map_err(|_| ApiError::Internal)
    }

    pub fn start_scan(
        &self,
        request: StartScanRequest,
        emit: impl FnMut(TaskProgress) -> bool + Send + 'static,
    ) -> Result<ScanTaskId, ApiError> {
        if !self.mock_enabled {
            return Err(ApiError::Unsupported);
        }
        if !(1..=30_000).contains(&request.timeout_ms) {
            return Err(ApiError::InvalidRequest);
        }
        let id = ScanTaskId::new();
        {
            let mut state = self.lock()?;
            if state.tasks.len() >= MAX_TASKS {
                let oldest = state
                    .tasks
                    .iter()
                    .filter(|(_, task)| task.progress.status != TaskStatus::Running)
                    .min_by_key(|(_, task)| task.started)
                    .map(|(id, _)| *id)
                    .ok_or(ApiError::Capacity)?;
                if let Some(scan_id) = state.tasks.remove(&oldest).and_then(|t| t.progress.scan_id)
                {
                    state.scans.remove(&scan_id);
                    state.plans.retain(|_, p| p.plan.scan_id != scan_id);
                }
            }
            state.tasks.insert(
                id,
                Task {
                    started: Instant::now(),
                    progress: TaskProgress {
                        task_id: id,
                        status: TaskStatus::Running,
                        processed: 0,
                        total: mock::COUNT,
                        scan_id: None,
                        error: None,
                    },
                },
            );
        }
        let engine = self.clone();
        if thread::Builder::new()
            .name("pc-care-mock-scan".into())
            .spawn(move || {
                engine.run_scan(id, request, emit);
            })
            .is_err()
        {
            self.finish_error(id, ApiError::Internal)?;
            return Err(ApiError::Internal);
        }
        Ok(id)
    }

    fn run_scan(
        &self,
        id: ScanTaskId,
        request: StartScanRequest,
        mut emit: impl FnMut(TaskProgress) -> bool,
    ) {
        let result = (|| -> Result<(), ApiError> {
            let initial = self.get_task(id)?;
            if !emit(initial) {
                return Err(ApiError::Disconnected);
            }
            let mut found = Vec::new();
            for candidate in mock::candidates() {
                thread::sleep(Duration::from_millis(30));
                let progress = {
                    let mut state = self.lock()?;
                    let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
                    if task.progress.status != TaskStatus::Running {
                        let progress = task.progress.clone();
                        drop(state);
                        emit(progress);
                        return Ok(());
                    }
                    if task.started.elapsed() >= Duration::from_millis(request.timeout_ms.into()) {
                        return Err(ApiError::Timeout);
                    }
                    if request.scenario == MockScenario::AdapterError {
                        return Err(ApiError::AdapterError);
                    }
                    found.push(candidate);
                    task.progress.processed += 1;
                    task.progress.clone()
                };
                if !emit(progress) {
                    return Err(ApiError::Disconnected);
                }
            }
            let progress = {
                let mut state = self.lock()?;
                let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
                if task.progress.status != TaskStatus::Running {
                    let progress = task.progress.clone();
                    drop(state);
                    emit(progress);
                    return Ok(());
                }
                if task.started.elapsed() >= Duration::from_millis(request.timeout_ms.into()) {
                    return Err(ApiError::Timeout);
                }
                let scan_id = ScanId::new();
                task.progress.status = TaskStatus::Completed;
                task.progress.scan_id = Some(scan_id);
                let progress = task.progress.clone();
                state.scans.insert(
                    scan_id,
                    ScanSnapshot {
                        id: scan_id,
                        task_id: id,
                        source: request.kind,
                        candidates: found,
                    },
                );
                progress
            };
            emit(progress);
            Ok(())
        })();
        if let Err(error) = result
            && let Ok(progress) = self.finish_error(id, error)
        {
            emit(progress);
        }
    }

    fn finish_error(&self, id: ScanTaskId, error: ApiError) -> Result<TaskProgress, ApiError> {
        let mut state = self.lock()?;
        let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
        if task.progress.status == TaskStatus::Running {
            task.progress.status = TaskStatus::Failed;
            task.progress.error = Some(error);
        }
        Ok(task.progress.clone())
    }

    pub fn get_task(&self, id: ScanTaskId) -> Result<TaskProgress, ApiError> {
        self.lock()?
            .tasks
            .get(&id)
            .map(|t| t.progress.clone())
            .ok_or(ApiError::NotFound)
    }
    pub fn cancel_task(&self, id: ScanTaskId) -> Result<TaskProgress, ApiError> {
        let mut state = self.lock()?;
        let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
        if task.progress.status == TaskStatus::Running {
            task.progress.status = TaskStatus::Cancelled;
        }
        Ok(task.progress.clone())
    }
    pub fn get_scan_result(
        &self,
        id: ScanId,
        offset: u32,
        limit: u32,
    ) -> Result<CandidatePage, ApiError> {
        if !(1..=500).contains(&limit) {
            return Err(ApiError::InvalidRequest);
        }
        let state = self.lock()?;
        let scan = state.scans.get(&id).ok_or(ApiError::NotFound)?;
        Ok(CandidatePage {
            scan_id: id,
            offset,
            total: scan.candidates.len() as u32,
            candidates: scan
                .candidates
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect(),
        })
    }

    pub fn build_operation_plan(
        &self,
        scan_id: ScanId,
        ids: Vec<CandidateId>,
    ) -> Result<OperationPlan, ApiError> {
        if ids.is_empty()
            || ids.len() > 500
            || ids.iter().collect::<HashSet<_>>().len() != ids.len()
        {
            return Err(ApiError::InvalidRequest);
        }
        let mut state = self.lock()?;
        let scan = state.scans.get(&scan_id).ok_or(ApiError::NotFound)?;
        let mut targets = Vec::new();
        let mut total = 0u64;
        let mut warnings = vec![PlanWarning::SimulationOnly];
        for id in ids {
            let candidate = scan
                .candidates
                .iter()
                .find(|c| c.id == id)
                .ok_or(ApiError::NotFound)?;
            if candidate.safety == SafetyClass::Protected {
                return Err(ApiError::Protected);
            }
            total = total
                .checked_add(candidate.bytes.0)
                .ok_or(ApiError::InvalidRequest)?;
            targets.push(candidate.clone());
        }
        if targets.iter().any(|c| c.safety == SafetyClass::Review) {
            warnings.push(PlanWarning::ReviewRequired);
        }
        state
            .plans
            .retain(|_, p| !p.executed && p.created.elapsed() < PLAN_TTL);
        if state.plans.len() >= MAX_PLANS {
            return Err(ApiError::Capacity);
        }
        let plan = OperationPlan {
            id: PlanId::new(),
            scan_id,
            simulated: true,
            targets,
            planned_bytes: ByteCount(total),
            expires_in_ms: PLAN_TTL.as_millis() as u32,
            warnings,
        };
        state.plans.insert(
            plan.id,
            StoredPlan {
                plan: plan.clone(),
                created: Instant::now(),
                executed: false,
            },
        );
        Ok(plan)
    }

    pub fn execute_plan(
        &self,
        id: PlanId,
        confirmation: bool,
    ) -> Result<OperationResult, ApiError> {
        // shortcut: simulation only; add durable outcome recovery before real effects in SPE-63.
        if !confirmation {
            return Err(ApiError::ConfirmationRequired);
        }
        let mut state = self.lock()?;
        let stored = state.plans.get(&id).ok_or(ApiError::NotFound)?;
        if stored.executed {
            return Err(ApiError::AlreadyExecuted);
        }
        if stored.created.elapsed() >= PLAN_TTL {
            return Err(ApiError::PlanExpired);
        }
        let scan = state
            .scans
            .get(&stored.plan.scan_id)
            .ok_or(ApiError::NotFound)?;
        let mut items = Vec::new();
        for candidate in &stored.plan.targets {
            let unchanged =
                scan.candidates.iter().any(|c| c == candidate) && mock::revalidate(candidate);
            let error = if candidate.safety == SafetyClass::Protected {
                Some(ApiError::Protected)
            } else if !unchanged {
                Some(ApiError::TargetChanged)
            } else {
                None
            };
            items.push(ItemResult {
                candidate_id: candidate.id,
                status: if error.is_some() {
                    ItemStatus::Skipped
                } else {
                    ItemStatus::Success
                },
                error,
                actual_reclaimed_bytes: ByteCount(0),
            });
        }
        state.plans.get_mut(&id).ok_or(ApiError::NotFound)?.executed = true;
        Ok(OperationResult {
            plan_id: id,
            simulated: true,
            items,
        })
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
