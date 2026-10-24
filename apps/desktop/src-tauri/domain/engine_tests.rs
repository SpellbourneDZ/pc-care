use super::*;
use crate::domain::contracts::ApiRequest;
use std::sync::mpsc;

fn request() -> StartScanRequest {
    StartScanRequest {
        kind: ScanKind::Mock,
        scenario: MockScenario::Complete,
        timeout_ms: 5_000,
    }
}

#[test]
fn cancelled_worker_keeps_its_slot_until_terminal_delivery() {
    let engine = Engine::new(true);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (events_tx, events_rx) = mpsc::channel();
    let id = engine
        .start_scan(request(), move |progress| {
            if progress.status == TaskStatus::Running && progress.processed == 0 {
                ready_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            }
            events_tx.send(progress).is_ok()
        })
        .unwrap();
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    engine.cancel_task(id).unwrap();
    for _ in 1..MAX_TASKS {
        let other = ScanTaskId::new();
        engine.lock().unwrap().tasks.insert(
            other,
            Task {
                started: Instant::now(),
                worker_finished: false,
                progress: TaskProgress {
                    task_id: other,
                    status: TaskStatus::Running,
                    processed: 0,
                    total: mock::COUNT,
                    scan_id: None,
                    error: None,
                },
            },
        );
    }
    assert_error(engine.start_scan(request(), |_| true), ApiError::Capacity);
    release_tx.send(()).unwrap();
    let events: Vec<_> = events_rx.iter().collect();
    assert_eq!(
        events
            .iter()
            .filter(|p| p.status == TaskStatus::Cancelled)
            .count(),
        1
    );
    assert_eq!(events.last().unwrap().status, TaskStatus::Cancelled);
}

#[test]
fn cancellation_before_worker_starts_emits_terminal_once() {
    let engine = Engine::new(true);
    let id = ScanTaskId::new();
    engine.lock().unwrap().tasks.insert(
        id,
        Task {
            started: Instant::now(),
            worker_finished: false,
            progress: TaskProgress {
                task_id: id,
                status: TaskStatus::Cancelled,
                processed: 0,
                total: mock::COUNT,
                scan_id: None,
                error: None,
            },
        },
    );
    let mut events = Vec::new();
    engine.run_scan(id, request(), |progress| {
        events.push(progress);
        true
    });
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, TaskStatus::Cancelled);
    assert!(engine.lock().unwrap().scans.is_empty());
}

#[test]
fn task_and_plan_retention_are_bounded() {
    let engine = Engine::new(true);
    let mut ids = Vec::new();
    for _ in 0..MAX_TASKS {
        let id = ScanTaskId::new();
        ids.push(id);
        engine.lock().unwrap().tasks.insert(
            id,
            Task {
                started: Instant::now(),
                worker_finished: false,
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
    assert_error(engine.start_scan(request(), |_| true), ApiError::Capacity);
    engine.cancel_task(ids[0]).unwrap();
    engine
        .lock()
        .unwrap()
        .tasks
        .get_mut(&ids[0])
        .unwrap()
        .worker_finished = true;
    let (scan_id, _) = scan(&engine);
    assert_eq!(engine.lock().unwrap().tasks.len(), MAX_TASKS);
    assert_error(engine.get_task(ids[0]), ApiError::NotFound);
    let candidate = engine.get_scan_result(scan_id, 0, 1).unwrap().candidates[0].id;
    let mut plans = Vec::new();
    for _ in 0..MAX_PLANS {
        plans.push(
            engine
                .build_operation_plan(scan_id, vec![candidate])
                .unwrap(),
        );
    }
    assert_error(
        engine.build_operation_plan(scan_id, vec![candidate]),
        ApiError::Capacity,
    );
    engine.execute_plan(plans[0].id, true).unwrap();
    assert!(
        engine
            .build_operation_plan(scan_id, vec![candidate])
            .is_ok()
    );
    assert_eq!(engine.lock().unwrap().plans.len(), MAX_PLANS);
}
fn scan(engine: &Engine) -> (ScanId, Vec<TaskProgress>) {
    let (tx, rx) = mpsc::channel();
    engine
        .start_scan(request(), move |p| tx.send(p).is_ok())
        .unwrap();
    let events: Vec<_> = rx.iter().collect();
    let final_event = events.last().unwrap();
    assert_eq!(final_event.status, TaskStatus::Completed);
    (final_event.scan_id.unwrap(), events)
}
fn assert_error<T: std::fmt::Debug>(actual: Result<T, ApiError>, expected: ApiError) {
    assert!(matches!(actual, Err(error) if error == expected));
}

#[test]
fn scan_progress_pagination_and_identity_are_consistent() {
    let engine = Engine::new(true);
    let (id, events) = scan(&engine);
    assert_eq!(events.first().unwrap().processed, 0);
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].processed <= pair[1].processed)
    );
    let first = engine.get_scan_result(id, 0, 2).unwrap();
    let second = engine.get_scan_result(id, 2, 2).unwrap();
    assert_eq!(first.total, 3);
    assert_eq!(second.candidates.len(), 1);
    assert_ne!(first.candidates[0].id, second.candidates[0].id);
    assert!(
        engine
            .get_scan_result(id, u32::MAX, 1)
            .unwrap()
            .candidates
            .is_empty()
    );
    assert_error(engine.get_scan_result(id, 0, 0), ApiError::InvalidRequest);
    assert_error(engine.get_scan_result(id, 0, 501), ApiError::InvalidRequest);
    assert_error(
        engine.get_scan_result(ScanId::new(), 0, 1),
        ApiError::NotFound,
    );
}

#[test]
fn plans_require_owned_ids_selection_confirmation_and_single_use() {
    let engine = Engine::new(true);
    let (id, _) = scan(&engine);
    let page = engine.get_scan_result(id, 0, 3).unwrap();
    let ids = vec![page.candidates[0].id, page.candidates[1].id];
    assert_error(
        engine.build_operation_plan(id, vec![]),
        ApiError::InvalidRequest,
    );
    assert_error(
        engine.build_operation_plan(id, vec![ids[0], ids[0]]),
        ApiError::InvalidRequest,
    );
    assert_error(
        engine.build_operation_plan(id, vec![CandidateId::new()]),
        ApiError::NotFound,
    );
    assert_error(
        engine.build_operation_plan(id, vec![page.candidates[2].id]),
        ApiError::Protected,
    );
    let plan = engine.build_operation_plan(id, ids).unwrap();
    assert_eq!(plan.planned_bytes, ByteCount(3072));
    assert!(plan.simulated && plan.warnings.contains(&PlanWarning::ReviewRequired));
    assert_error(
        engine.execute_plan(plan.id, false),
        ApiError::ConfirmationRequired,
    );
    let result = engine.execute_plan(plan.id, true).unwrap();
    assert!(result.simulated);
    assert!(
        result
            .items
            .iter()
            .all(|r| r.status == ItemStatus::Success && r.actual_reclaimed_bytes == ByteCount(0))
    );
    assert_error(
        engine.execute_plan(plan.id, true),
        ApiError::AlreadyExecuted,
    );
    assert_error(engine.execute_plan(PlanId::new(), true), ApiError::NotFound);
}

#[test]
fn foreign_scan_ids_and_changed_identity_cannot_execute() {
    let engine = Engine::new(true);
    let (first, _) = scan(&engine);
    let (second, _) = scan(&engine);
    let candidate = engine.get_scan_result(first, 0, 1).unwrap().candidates[0].clone();
    assert_error(
        engine.build_operation_plan(second, vec![candidate.id]),
        ApiError::NotFound,
    );
    let plan = engine
        .build_operation_plan(first, vec![candidate.id])
        .unwrap();
    // Simulate an adapter replacing a target after scan/plan.
    {
        let mut state = engine.lock().unwrap();
        state.plans.get_mut(&plan.id).unwrap().plan.targets[0].identity = "replaced".into();
    }
    let result = engine.execute_plan(plan.id, true).unwrap();
    assert_eq!(result.items[0].status, ItemStatus::Skipped);
    assert_eq!(result.items[0].error, Some(ApiError::TargetChanged));
}

#[test]
fn expired_plan_and_concurrent_replay_are_rejected() {
    let engine = Engine::new(true);
    let (id, _) = scan(&engine);
    let candidate = engine.get_scan_result(id, 0, 1).unwrap().candidates[0].id;
    let expired = engine.build_operation_plan(id, vec![candidate]).unwrap();
    engine
        .lock()
        .unwrap()
        .plans
        .get_mut(&expired.id)
        .unwrap()
        .created -= PLAN_TTL;
    assert_error(engine.execute_plan(expired.id, true), ApiError::PlanExpired);
    let plan = engine.build_operation_plan(id, vec![candidate]).unwrap();
    let other = engine.clone();
    let worker = thread::spawn(move || other.execute_plan(plan.id, true));
    let local = engine.execute_plan(plan.id, true);
    let remote = worker.join().unwrap();
    assert_eq!(
        [local.is_ok(), remote.is_ok()]
            .into_iter()
            .filter(|ok| *ok)
            .count(),
        1
    );
}

#[test]
fn cancellation_emits_terminal_state_without_publishing_snapshot() {
    let engine = Engine::new(true);
    let other = engine.clone();
    let (tx, rx) = mpsc::channel();
    let id = engine
        .start_scan(request(), move |p| {
            if p.status == TaskStatus::Running && p.processed == 1 {
                other.cancel_task(p.task_id).unwrap();
            }
            tx.send(p).is_ok()
        })
        .unwrap();
    let events: Vec<_> = rx.iter().collect();
    assert_eq!(events.last().unwrap().status, TaskStatus::Cancelled);
    assert!(events.iter().all(|p| p.scan_id.is_none()));
    assert_eq!(
        engine.cancel_task(id).unwrap().status,
        TaskStatus::Cancelled
    );
    assert!(engine.lock().unwrap().scans.is_empty());
}

#[test]
fn timeout_adapter_failure_and_disconnection_are_visible() {
    for (timeout_ms, scenario, expected) in [
        (1, MockScenario::Complete, ApiError::Timeout),
        (5_000, MockScenario::AdapterError, ApiError::AdapterError),
    ] {
        let engine = Engine::new(true);
        let (tx, rx) = mpsc::channel();
        engine
            .start_scan(
                StartScanRequest {
                    timeout_ms,
                    scenario,
                    ..request()
                },
                move |p| tx.send(p).is_ok(),
            )
            .unwrap();
        let last = rx.iter().last().unwrap();
        assert_eq!(last.status, TaskStatus::Failed);
        assert_eq!(last.error, Some(expected));
        assert!(last.scan_id.is_none());
    }
    let engine = Engine::new(true);
    let (tx, rx) = mpsc::channel();
    let id = engine
        .start_scan(request(), move |_| {
            tx.send(()).unwrap();
            false
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(1)).unwrap();
    // Wait for worker completion by observing its terminal callback.
    rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        engine.get_task(id).unwrap().error,
        Some(ApiError::Disconnected)
    );
}

#[test]
fn production_disables_mock_and_limits_are_validated() {
    assert_error(
        Engine::new(false).start_scan(request(), |_| true),
        ApiError::Unsupported,
    );
    assert_error(
        Engine::new(true).start_scan(
            StartScanRequest {
                timeout_ms: 0,
                ..request()
            },
            |_| true,
        ),
        ApiError::InvalidRequest,
    );
    assert_error(
        Engine::new(true).start_scan(
            StartScanRequest {
                timeout_ms: 30_001,
                ..request()
            },
            |_| true,
        ),
        ApiError::InvalidRequest,
    );
    assert_error(
        Engine::new(true).cancel_task(ScanTaskId::new()),
        ApiError::NotFound,
    );
}

#[test]
fn hostile_or_unknown_ipc_fields_are_rejected() {
    for request in [
        r#"{"type":"start_scan","data":{"kind":"mock","scenario":"complete","timeout_ms":1000,"path":"C:\\Windows"}}"#,
        r#"{"type":"execute_plan","data":{"plan_id":"C:\\Windows","explicit_confirmation":true}}"#,
        r#"{"type":"delete_file","data":{"path":"C:\\Windows"}}"#,
        r#"{"type":"start_scan","data":{"kind":"filesystem","scenario":"complete","timeout_ms":1000}}"#,
        r#"{"type":"start_scan","data":{"kind":"mock","scenario":"complete","timeout_ms":-1}}"#,
    ] {
        assert!(serde_json::from_str::<ApiRequest>(request).is_err());
    }
    let bytes = ByteCount(u64::MAX);
    assert_eq!(
        serde_json::to_string(&bytes).unwrap(),
        "\"18446744073709551615\""
    );
    assert_eq!(
        serde_json::from_str::<ByteCount>("\"18446744073709551615\"").unwrap(),
        bytes
    );
    assert!(serde_json::from_str::<ByteCount>("18446744073709551615").is_err());
}
