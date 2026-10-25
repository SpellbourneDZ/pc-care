use super::*;
use crate::domain::execution::{ExecutionItemStatus, ExecutionPhase};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use uuid::Uuid;

fn fixture() -> (ExecutionManager, OperationPlan) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/policy-fixtures")
        .join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&root).unwrap();
    let manager = ExecutionManager::new(Some(root.canonicalize().unwrap()));
    let plan = OperationPlan {
        id: PlanId::new(),
        scan_id: ScanId::new(),
        simulated: false,
        planned_bytes: ByteCount(2),
        expires_in_ms: 30000,
        warnings: vec![],
        targets: (0..2)
            .map(|_| Candidate {
                id: CandidateId::new(),
                source: ScanKind::Cleanup,
                source_key: "user-temp-v1".into(),
                rule_id: "user-temp-v1".into(),
                display_path: "C:\\fixture\\only.tmp".into(),
                identity: "opaque".into(),
                bytes: ByteCount(1),
                logical_bytes: ByteCount(1),
                safety: SafetyClass::Safe,
            })
            .collect(),
    };
    (manager, plan)
}

#[test]
fn acknowledgement_cancel_and_replay_do_not_repeat_effects() {
    let (manager, plan) = fixture();
    let id = plan.id;
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    manager
        .start(plan.clone(), move |_, _, _| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(RecycleReceipt)
        })
        .unwrap();
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let cancelled = manager.cancel(id).unwrap();
    assert!(cancelled.cancel_requested);
    assert_eq!(cancelled.record.phase, ExecutionPhase::Running);
    assert!(
        manager
            .start(plan.clone(), |_, _, _| panic!("replayed effect"))
            .is_ok()
    );
    let mut other = plan.clone();
    other.id = PlanId::new();
    assert!(matches!(
        manager.start(other, |_, _, _| panic!("concurrent effect")),
        Err(ApiError::Capacity)
    ));
    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let current = manager.get(id).unwrap();
        if current.record.phase != ExecutionPhase::Running {
            assert_eq!(current.record.phase, ExecutionPhase::Cancelled);
            assert_eq!(current.record.items[0].status, ExecutionItemStatus::Success);
            assert_eq!(current.record.items[1].status, ExecutionItemStatus::Skipped);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let reopened = ExecutionManager::new(manager.root.clone());
    assert!(
        reopened
            .start(plan, |_, _, _| panic!("restart replay"))
            .is_ok()
    );
    assert_eq!(
        reopened.get(id).unwrap().record.phase,
        ExecutionPhase::Cancelled
    );
}

#[test]
fn adapter_panic_recovers_unknown_without_keeping_the_worker_slot() {
    let (manager, plan) = fixture();
    let id = plan.id;
    manager
        .start(plan, |_, _, _| panic!("artificial adapter panic"))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let current = manager.get(id).unwrap();
        if current.record.phase != ExecutionPhase::Running {
            assert_eq!(current.record.phase, ExecutionPhase::Interrupted);
            assert_eq!(current.record.items[0].status, ExecutionItemStatus::Unknown);
            assert!(!current.journal_error);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let restarted = ExecutionManager::new(manager.root.clone());
    assert_eq!(
        restarted.get(id).unwrap().record.phase,
        ExecutionPhase::Interrupted
    );
}
