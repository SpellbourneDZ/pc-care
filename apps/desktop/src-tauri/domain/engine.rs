use super::{contracts::*, mock};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

pub enum ScanEvent {
    Checkpoint,
    Visited,
    Candidate(Candidate),
    Issue(ScanIssue),
    StorageNode(StorageNode),
}
pub type ScanSink<'a> = dyn FnMut(ScanEvent) -> Result<(), ApiError> + 'a;
const MAX_CANDIDATES: usize = 10_000;
const MAX_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
const MAX_ISSUES: usize = 128;
const MAX_VISITED: u32 = 100_000;
const MAX_TASKS: usize = 32;
const MAX_PLANS: usize = 32;
pub const MAX_PLAN_ITEMS: usize = 500;
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
    storage: HashMap<ScanId, (Vec<StorageNode>, u64)>,
    storage_last: HashMap<String, (String, u64)>,
}
struct Task {
    source: ScanKind,
    progress: TaskProgress,
    started: Instant,
    worker_finished: bool,
}
struct StoredPlan {
    plan: OperationPlan,
    created: Instant,
    executed: bool,
}

impl Engine {
    pub(crate) fn storage_last(&self, root: &str) -> Result<Option<(String, u64)>, ApiError> {
        Ok(self
            .lock()?
            .storage_last
            .get(&root[..2].to_ascii_uppercase())
            .cloned())
    }
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
        if !self.mock_enabled || request.kind != ScanKind::Mock {
            return Err(ApiError::Unsupported);
        }
        self.start_worker(
            ScanKind::Mock,
            request.timeout_ms,
            Some(mock::COUNT),
            emit,
            move |sink| mock_scan(request.scenario, sink),
        )
    }

    pub fn start_cleanup_scan(
        &self,
        timeout_ms: u32,
        emit: impl FnMut(TaskProgress) -> bool + Send + 'static,
        worker: impl FnOnce(&mut ScanSink<'_>) -> Result<(), ApiError> + Send + 'static,
    ) -> Result<ScanTaskId, ApiError> {
        self.start_worker(ScanKind::Cleanup, timeout_ms, None, emit, worker)
    }

    pub fn start_storage_scan(
        &self,
        timeout_ms: u32,
        emit: impl FnMut(TaskProgress) -> bool + Send + 'static,
        worker: impl FnOnce(&mut ScanSink<'_>) -> Result<(), ApiError> + Send + 'static,
    ) -> Result<ScanTaskId, ApiError> {
        self.start_worker(ScanKind::Storage, timeout_ms, None, emit, worker)
    }

    fn start_worker(
        &self,
        source: ScanKind,
        timeout_ms: u32,
        total: Option<u32>,
        emit: impl FnMut(TaskProgress) -> bool + Send + 'static,
        worker: impl FnOnce(&mut ScanSink<'_>) -> Result<(), ApiError> + Send + 'static,
    ) -> Result<ScanTaskId, ApiError> {
        let maximum = if source == ScanKind::Mock {
            30_000
        } else {
            300_000
        };
        if !(1..=maximum).contains(&timeout_ms) {
            return Err(ApiError::InvalidRequest);
        }
        let id = ScanTaskId::new();
        {
            let mut state = self.lock()?;
            if source != ScanKind::Mock
                && state
                    .tasks
                    .values()
                    .any(|task| task.source != ScanKind::Mock && !task.worker_finished)
            {
                return Err(ApiError::Capacity);
            }
            if state.tasks.len() >= MAX_TASKS {
                let oldest = state
                    .tasks
                    .iter()
                    .filter(|(_, task)| task.worker_finished)
                    .min_by_key(|(_, task)| task.started)
                    .map(|(id, _)| *id)
                    .ok_or(ApiError::Capacity)?;
                if let Some(scan_id) = state.tasks.remove(&oldest).and_then(|t| t.progress.scan_id)
                {
                    state.scans.remove(&scan_id);
                    state.plans.retain(|_, p| p.plan.scan_id != scan_id);
                }
            }
            if source == ScanKind::Storage {
                // A single retained storage report bounds memory independently of task history.
                state.storage.clear();
                state
                    .scans
                    .retain(|_, scan| scan.source != ScanKind::Storage);
            }
            state.tasks.insert(
                id,
                Task {
                    source,
                    started: Instant::now(),
                    worker_finished: false,
                    progress: TaskProgress {
                        task_id: id,
                        status: TaskStatus::Running,
                        processed: 0,
                        total,
                        scan_id: None,
                        error: None,
                    },
                },
            );
        }
        let engine = self.clone();
        if thread::Builder::new()
            .name("pc-care-scan".into())
            .spawn(move || {
                engine.run_worker(id, source, timeout_ms, worker, emit);
                if let Ok(mut state) = engine.lock()
                    && let Some(task) = state.tasks.get_mut(&id)
                {
                    task.worker_finished = true;
                }
            })
            .is_err()
        {
            self.finish_error(id, ApiError::Internal)?;
            self.lock()?
                .tasks
                .get_mut(&id)
                .ok_or(ApiError::NotFound)?
                .worker_finished = true;
            return Err(ApiError::Internal);
        }
        Ok(id)
    }

    #[cfg(test)]
    fn run_scan(
        &self,
        id: ScanTaskId,
        request: StartScanRequest,
        emit: impl FnMut(TaskProgress) -> bool,
    ) {
        self.run_worker(
            id,
            request.kind,
            request.timeout_ms,
            |sink| mock_scan(request.scenario, sink),
            emit,
        );
    }

    fn run_worker(
        &self,
        id: ScanTaskId,
        source: ScanKind,
        timeout_ms: u32,
        worker: impl FnOnce(&mut ScanSink<'_>) -> Result<(), ApiError>,
        mut emit: impl FnMut(TaskProgress) -> bool,
    ) {
        let result = (|| -> Result<(), ApiError> {
            let initial = self.get_task(id)?;
            let running = initial.status == TaskStatus::Running;
            if !emit(initial) {
                return Err(ApiError::Disconnected);
            }
            if !running {
                return Ok(());
            }
            let mut candidates = Vec::new();
            let mut storage_nodes = Vec::new();
            let mut issues: Vec<ScanIssue> = Vec::new();
            let mut snapshot_bytes = 0_usize;
            let mut last_emit = Instant::now();
            let outcome = worker(&mut |event| {
                let progress = {
                    let mut state = self.lock()?;
                    let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
                    if task.progress.status != TaskStatus::Running {
                        return Err(ApiError::NotReady);
                    }
                    if task.started.elapsed() >= Duration::from_millis(timeout_ms.into()) {
                        return Err(ApiError::Timeout);
                    }
                    match event {
                        ScanEvent::Visited => {
                            if task.progress.processed
                                >= if source == ScanKind::Storage {
                                    1_000_000
                                } else {
                                    MAX_VISITED
                                }
                            {
                                return Err(ApiError::Capacity);
                            }
                            task.progress.processed += 1;
                        }
                        ScanEvent::Candidate(candidate) => {
                            if candidate.source != source {
                                return Err(ApiError::Internal);
                            }
                            if candidates.len() >= MAX_CANDIDATES {
                                return Err(ApiError::Capacity);
                            }
                            snapshot_bytes += std::mem::size_of::<Candidate>()
                                + candidate.display_path.len()
                                + candidate.identity.len()
                                + candidate.source_key.len()
                                + candidate.rule_id.len();
                            if snapshot_bytes > MAX_SNAPSHOT_BYTES {
                                return Err(ApiError::Capacity);
                            }
                            candidates.push(candidate);
                        }
                        ScanEvent::Issue(issue) => {
                            if let Some(group) = issues.iter_mut().find(|group| {
                                group.rule_id == issue.rule_id && group.reason == issue.reason
                            }) {
                                group.count = group.count.saturating_add(issue.count);
                            } else {
                                if issues.len() >= MAX_ISSUES {
                                    return Err(ApiError::Capacity);
                                }
                                snapshot_bytes += std::mem::size_of::<ScanIssue>()
                                    + issue.rule_id.len()
                                    + issue.sample_path.as_ref().map_or(0, String::len);
                                if snapshot_bytes
                                    > if source == ScanKind::Storage {
                                        super::storage::MAX_BYTES
                                    } else {
                                        MAX_SNAPSHOT_BYTES
                                    }
                                {
                                    return Err(ApiError::Capacity);
                                }
                                issues.push(issue);
                            }
                        }
                        ScanEvent::StorageNode(node) => {
                            if source != ScanKind::Storage
                                || node.id as usize != storage_nodes.len()
                            {
                                return Err(ApiError::Internal);
                            }
                            if storage_nodes.len() >= super::storage::MAX_NODES {
                                return Err(ApiError::Capacity);
                            }
                            snapshot_bytes += std::mem::size_of::<StorageNode>()
                                + node.name.len()
                                + node.path.len();
                            if snapshot_bytes > super::storage::MAX_BYTES {
                                return Err(ApiError::Capacity);
                            }
                            storage_nodes.push(node);
                        }
                        ScanEvent::Checkpoint => {}
                    }
                    task.progress.clone()
                };
                if source == ScanKind::Mock || last_emit.elapsed() >= Duration::from_millis(50) {
                    last_emit = Instant::now();
                    if !emit(progress) {
                        return Err(ApiError::Disconnected);
                    }
                }
                Ok(())
            });
            let progress = {
                let mut state = self.lock()?;
                let task = state.tasks.get_mut(&id).ok_or(ApiError::NotFound)?;
                if task.progress.status != TaskStatus::Running {
                    let progress = task.progress.clone();
                    drop(state);
                    emit(progress);
                    return Ok(());
                }
                let truncated = outcome == Err(ApiError::Capacity);
                if !truncated {
                    outcome?;
                }
                if task.started.elapsed() >= Duration::from_millis(timeout_ms.into()) {
                    return Err(ApiError::Timeout);
                }
                if truncated {
                    issues.push(ScanIssue {
                        rule_id: "scan".into(),
                        reason: super::policy::Denial::LimitReached,
                        count: 1,
                        sample_path: None,
                    });
                }
                let scan_id = ScanId::new();
                if source == ScanKind::Storage {
                    super::storage::aggregate(&mut storage_nodes)?;
                }
                let completed = if source == ScanKind::Storage {
                    Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|_| ApiError::Internal)?
                            .as_millis() as u64,
                    )
                } else {
                    None
                };
                task.progress.status = TaskStatus::Completed;
                task.progress.scan_id = Some(scan_id);
                let progress = task.progress.clone();
                if let Some(completed) = completed {
                    let root = storage_nodes
                        .first()
                        .ok_or(ApiError::Internal)?
                        .path
                        .clone();
                    state
                        .storage_last
                        .insert(root[..2].to_ascii_uppercase(), (root, completed));
                    state.storage.insert(scan_id, (storage_nodes, completed));
                }
                state.scans.insert(
                    scan_id,
                    ScanSnapshot {
                        id: scan_id,
                        task_id: id,
                        source,
                        candidates,
                        issues,
                        truncated,
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
            issues: scan.issues.clone(),
            truncated: scan.truncated,
            candidates: scan
                .candidates
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect(),
        })
    }

    pub fn get_storage_page(
        &self,
        id: ScanId,
        query: StorageQuery,
    ) -> Result<StoragePage, ApiError> {
        let state = self.lock()?;
        let scan = state.scans.get(&id).ok_or(ApiError::NotFound)?;
        let (nodes, completed) = state.storage.get(&id).ok_or(ApiError::NotFound)?;
        super::storage::page(scan, nodes, *completed, query)
    }

    pub(crate) fn storage_node(&self, id: ScanId, node_id: u32) -> Result<StorageNode, ApiError> {
        self.lock()?
            .storage
            .get(&id)
            .and_then(|(nodes, _)| nodes.get(node_id as usize))
            .cloned()
            .ok_or(ApiError::NotFound)
    }

    pub fn build_operation_plan(
        &self,
        scan_id: ScanId,
        ids: Vec<CandidateId>,
    ) -> Result<OperationPlan, ApiError> {
        if ids.is_empty()
            || ids.len() > MAX_PLAN_ITEMS
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
        if scan.source != ScanKind::Mock {
            return Err(ApiError::Unsupported);
        }
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

fn mock_scan(scenario: MockScenario, sink: &mut ScanSink<'_>) -> Result<(), ApiError> {
    for candidate in mock::candidates() {
        thread::sleep(Duration::from_millis(30));
        if scenario == MockScenario::AdapterError {
            return Err(ApiError::AdapterError);
        }
        sink(ScanEvent::Visited)?;
        sink(ScanEvent::Candidate(candidate))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
