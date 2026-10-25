use super::*;
use crate::{
    domain::{contracts::*, execution::*},
    win::adapters::{
        execution::ExecutionManager,
        files::{FilePolicy, FileSnapshot, FileSource},
    },
};
use std::{
    fs,
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    policy: FilePolicy,
    snapshot: FileSnapshot,
    plan: OperationPlan,
}
impl Fixture {
    fn new() -> Self {
        let root = super::super::tests::fixture();
        let profile = root.join("Profile");
        let temp = profile.join("AppData/Local/Temp");
        fs::create_dir_all(&temp).unwrap();
        let source = temp.join("artificial.tmp");
        fs::write(&source, b"synthetic recovery fixture").unwrap();
        let policy = FilePolicy::for_fixture(&profile).unwrap();
        let mut plan = super::super::tests::plan();
        let snapshot = policy
            .snapshot(
                plan.targets[0].id,
                FileSource::UserTemp,
                source.to_str().unwrap(),
            )
            .unwrap();
        plan.targets[0].bytes = ByteCount(snapshot.metadata().allocated_size);
        plan.planned_bytes = plan.targets[0].bytes;
        // The mapping must use the held backend snapshot, not this display value.
        plan.targets[0].display_path = r"C:\untrusted\display.tmp".into();
        Self {
            root: root.join("journal"),
            policy,
            snapshot,
            plan,
        }
    }
    fn started(&self) -> (JournalSession, ExecutionRecord) {
        let session = JournalSession::open(&self.root, true).unwrap();
        let mut record = ExecutionRecord::new(&self.plan).unwrap();
        session.create(&record).unwrap();
        record.items[0].status = ExecutionItemStatus::Started;
        session.replace(&record).unwrap();
        (session, record)
    }
}

#[test]
fn mapping_is_durable_before_any_staging_and_never_uses_a_display_path() {
    let f = Fixture::new();
    let (session, mut record) = f.started();
    let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
    record.items[0].status = ExecutionItemStatus::Pending;
    session.replace(&record).unwrap();
    assert_eq!(
        session.prepare_recovery(f.plan.id, &held),
        Err(ApiError::InvalidRequest)
    );
    record.items[0].status = ExecutionItemStatus::Started;
    session.replace(&record).unwrap();
    let stage = session.prepare_recovery(f.plan.id, &held).unwrap();
    assert_eq!(stage.parent(), Some(f.root.as_path()));
    assert!(!stage.exists());
    let mapping = session.read_recovery(f.plan.id).unwrap();
    assert_eq!(mapping.original_path, f.snapshot.path());
    assert_eq!(mapping.metadata, *f.snapshot.metadata());
    assert_eq!(session.pending_recoveries().unwrap(), vec![f.plan.id]);
    session.settle_recovery(f.plan.id).unwrap();
    assert!(session.recovery_pending(f.plan.id).unwrap());
    drop(held);
    drop(session);
    let reopened = JournalSession::open(&f.root, false).unwrap();
    assert!(reopened.recovery_pending(f.plan.id).unwrap());
    assert_eq!(
        reopened.read_recovery(f.plan.id).unwrap().original_path,
        f.snapshot.path()
    );
    assert_eq!(
        fs::read(f.snapshot.path()).unwrap(),
        b"synthetic recovery fixture"
    );
    let ledger = fs::read_to_string(reopened.path(f.plan.id)).unwrap();
    assert!(!ledger.contains("artificial.tmp"));
    assert!(!ledger.contains("untrusted"));
}

#[test]
fn resolved_mapping_can_advance_to_the_next_item_only_after_a_durable_receipt() {
    let mut f = Fixture::new();
    let second_path = PathBuf::from(f.snapshot.path()).with_file_name("second.tmp");
    fs::write(&second_path, b"second synthetic fixture").unwrap();
    let second = f
        .policy
        .snapshot(
            CandidateId::new(),
            FileSource::UserTemp,
            second_path.to_str().unwrap(),
        )
        .unwrap();
    let mut target = f.plan.targets[0].clone();
    target.id = second.id();
    target.bytes = ByteCount(second.metadata().allocated_size);
    f.plan.planned_bytes.0 += target.bytes.0;
    f.plan.targets.push(target);
    let (session, mut record) = f.started();
    let first = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
    let first_stage = session.prepare_recovery(f.plan.id, &first).unwrap();
    drop(first);
    let held = f.policy.reserve(second.id(), &second).unwrap();
    assert_eq!(
        session.prepare_recovery(f.plan.id, &held),
        Err(ApiError::Capacity)
    );
    record.items[0].status = ExecutionItemStatus::Success;
    record.items[0].recycled = true; // Synthetic receipt; no source effect is performed.
    record.items[1].status = ExecutionItemStatus::Started;
    session.replace(&record).unwrap();
    session.settle_recovery(f.plan.id).unwrap();
    assert!(!session.recovery_pending(f.plan.id).unwrap());
    let next_stage = session.prepare_recovery(f.plan.id, &held).unwrap();
    assert_ne!(first_stage, next_stage);
    assert_eq!(
        session.read_recovery(f.plan.id).unwrap().candidate_id,
        second.id()
    );
    assert!(!first_stage.exists() && !next_stage.exists());
    assert_eq!(
        fs::read(f.snapshot.path()).unwrap(),
        b"synthetic recovery fixture"
    );
}

#[test]
fn malformed_future_locked_and_hardlinked_maps_fail_closed() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let (session, _) = f.started();
    let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
    session.prepare_recovery(f.plan.id, &held).unwrap();
    let path = session.recovery_path(f.plan.id);
    fs::rename(&path, path.with_extension("RECOVERY")).unwrap();
    assert_eq!(session.pending_recoveries().unwrap(), vec![f.plan.id]);
    let original = fs::read(&path).unwrap();
    let mut invalid: serde_json::Value = serde_json::from_slice(&original).unwrap();
    invalid["schema_version"] = 99.into();
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(matches!(
        session.pending_recoveries(),
        Err(ApiError::Unsupported)
    ));
    fs::write(&path, b"{broken").unwrap();
    assert!(session.pending_recoveries().is_err());
    fs::write(&path, &original).unwrap();
    let locked = OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert!(session.pending_recoveries().is_err());
    drop(locked);
    fs::hard_link(&path, f.root.join("map-link")).unwrap();
    assert!(session.pending_recoveries().is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn an_orphaned_mapping_is_an_error_instead_of_an_empty_pending_list() {
    let f = Fixture::new();
    let (session, _) = f.started();
    let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
    session.prepare_recovery(f.plan.id, &held).unwrap();
    fs::rename(
        session.path(f.plan.id),
        f.root.join("preserved-ledger.backup"),
    )
    .unwrap();
    assert!(matches!(
        session.pending_recoveries(),
        Err(ApiError::AdapterError)
    ));
    assert!(matches!(
        session.recovery_pending(f.plan.id),
        Err(ApiError::AdapterError)
    ));
    assert_eq!(
        fs::read(f.snapshot.path()).unwrap(),
        b"synthetic recovery fixture"
    );
}

#[test]
fn simulated_and_foreign_selections_cannot_prepare_a_mapping() {
    let f = Fixture::new();
    let (session, _) = f.started();
    let foreign = f
        .policy
        .snapshot(CandidateId::new(), FileSource::UserTemp, f.snapshot.path())
        .unwrap();
    let held = f.policy.reserve(foreign.id(), &foreign).unwrap();
    assert_eq!(
        session.prepare_recovery(f.plan.id, &held),
        Err(ApiError::InvalidRequest)
    );
    assert!(!session.recovery_pending(f.plan.id).unwrap());
    drop(held);
    let mut dry = Fixture::new();
    dry.plan.simulated = true;
    let session = JournalSession::open(&dry.root, true).unwrap();
    session
        .create(&ExecutionRecord::new(&dry.plan).unwrap())
        .unwrap();
    let held = dry
        .policy
        .reserve(dry.snapshot.id(), &dry.snapshot)
        .unwrap();
    assert_eq!(
        session.prepare_recovery(dry.plan.id, &held),
        Err(ApiError::InvalidRequest)
    );
    assert!(session.pending_recoveries().unwrap().is_empty());
}

#[test]
fn storage_inside_or_above_a_cleanup_root_is_rejected() {
    let f = Fixture::new();
    for storage in [
        f.root.parent().unwrap().to_path_buf(),
        PathBuf::from(f.snapshot.path())
            .parent()
            .unwrap()
            .join("unsafe-journal"),
    ] {
        let session = JournalSession::open(&storage, true).unwrap();
        let mut record = ExecutionRecord::new(&f.plan).unwrap();
        session.create(&record).unwrap();
        record.items[0].status = ExecutionItemStatus::Started;
        session.replace(&record).unwrap();
        let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
        assert_eq!(
            session.prepare_recovery(f.plan.id, &held),
            Err(ApiError::Protected)
        );
        assert!(!session.recovery_pending(f.plan.id).unwrap());
    }
    assert_eq!(
        fs::read(f.snapshot.path()).unwrap(),
        b"synthetic recovery fixture"
    );
}

#[test]
fn failed_mapping_settlement_preserves_the_durable_receipt_and_can_be_reconciled_without_effects() {
    use std::{os::windows::fs::OpenOptionsExt, sync::Mutex};
    let f = Fixture::new();
    let id = f.plan.id;
    let root = f.root.clone();
    let held_map = Arc::new(Mutex::new(None));
    let shared = held_map.clone();
    let manager = ExecutionManager::new(Some(root.clone()));
    manager
        .start(f.plan, move |_, _, session| {
            let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
            session.prepare_recovery(id, &held).unwrap();
            *shared.lock().unwrap() = Some(
                OpenOptions::new()
                    .read(true)
                    .share_mode(FILE_SHARE_READ)
                    .open(session.recovery_path(id))
                    .unwrap(),
            );
            Ok(RecycleReceipt) // Fake receipt: only journal metadata is written in this test.
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let current = manager.get(id).unwrap();
        if current.record.phase != ExecutionPhase::Running {
            assert_eq!(current.record.items[0].status, ExecutionItemStatus::Success);
            assert!(current.recovery_pending && current.journal_error);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    drop(held_map.lock().unwrap().take());
    let restarted = ExecutionManager::new(Some(root));
    let recovered = restarted.get(id).unwrap();
    assert_eq!(
        recovered.record.items[0].status,
        ExecutionItemStatus::Success
    );
    assert!(!recovered.recovery_pending);
    assert!(!recovered.journal_error);
}

#[test]
fn adapter_failure_with_a_pending_map_stops_the_batch_and_blocks_new_plans_after_restart() {
    for panic_after_mapping in [false, true] {
        let f = Fixture::new();
        let id = f.plan.id;
        let root = f.root.clone();
        let mut next = f.plan.clone();
        next.id = PlanId::new();
        let manager = ExecutionManager::new(Some(root.clone()));
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        manager
            .start(f.plan, move |_, _, session| {
                counted.fetch_add(1, Ordering::SeqCst);
                let held = f.policy.reserve(f.snapshot.id(), &f.snapshot).unwrap();
                session.prepare_recovery(id, &held).unwrap();
                assert!(
                    !panic_after_mapping,
                    "synthetic panic after recovery mapping"
                );
                Err(ExecutionFailure::AdapterError)
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let current = manager.get(id).unwrap();
            if current.record.phase != ExecutionPhase::Running {
                assert_eq!(current.record.phase, ExecutionPhase::Interrupted);
                assert_eq!(current.record.items[0].status, ExecutionItemStatus::Unknown);
                assert!(current.recovery_pending);
                assert!(!current.journal_error);
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let restarted = ExecutionManager::new(Some(root));
        assert!(restarted.get(id).unwrap().recovery_pending);
        assert!(matches!(
            restarted.start(next, |_, _, _| panic!("new effect with pending recovery")),
            Err(ApiError::Capacity)
        ));
    }
}
